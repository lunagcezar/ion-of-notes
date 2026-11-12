//! SQLite adapters for workspace, collaboration and auth state (spec-039).
//!
//! The first slice stores the existing JSON documents in SQLite, so behavior
//! is preserved while the server gains a durable, transactional backend.
//! Normalized page/block tables are a later spec; the store ports stay the
//! same.

use crate::auth::{Account, AuthStore, Session};
use crate::collab::CollabState;
use crate::model::{
    CommentDto, DataSource, Database, FileUpload, Page, View, Workspace, WorkspaceSettings, now_iso,
};
use crate::store::{CollabStore, RateLimitHit, RateLimitStore, SnapshotStore, WorkspaceStore};
use rusqlite::{Connection, OptionalExtension, params};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// SQLite adapter implementing all three persistence ports.
#[derive(Clone)]
pub struct SqliteStore {
    path: PathBuf,
    connection: Arc<Mutex<Connection>>,
}

/// Workspace fields that are not pages. Pages live in their own table so a
/// one-page write does not serialize the whole workspace (spec-114).
#[derive(serde::Serialize, serde::Deserialize)]
struct WorkspaceMeta {
    name: String,
    version: u64,
    settings: WorkspaceSettings,
    #[serde(default)]
    databases: Vec<Database>,
    #[serde(default)]
    data_sources: Vec<DataSource>,
    #[serde(default)]
    views: Vec<View>,
    #[serde(default)]
    comments: Vec<CommentDto>,
    #[serde(default)]
    file_uploads: Vec<FileUpload>,
}

impl WorkspaceMeta {
    fn from_workspace(workspace: &Workspace) -> Self {
        Self {
            name: workspace.name.clone(),
            version: workspace.version,
            settings: workspace.settings.clone(),
            databases: workspace.databases.clone(),
            data_sources: workspace.data_sources.clone(),
            views: workspace.views.clone(),
            comments: workspace.comments.clone(),
            file_uploads: workspace.file_uploads.clone(),
        }
    }

    fn into_workspace(self, pages: Vec<Page>) -> Workspace {
        Workspace::from_stored_parts(
            self.name,
            pages,
            self.version,
            self.settings,
            self.databases,
            self.data_sources,
            self.views,
            self.comments,
            self.file_uploads,
        )
    }
}

fn serialize_page(page: &Page) -> rusqlite::Result<String> {
    serde_json::to_string(page)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
}

fn write_meta(connection: &Connection, meta: &WorkspaceMeta, now: &str) -> rusqlite::Result<()> {
    let json = serde_json::to_string(meta)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    connection.execute(
        "INSERT INTO workspace_meta (id, json, updated_at) VALUES (1, ?1, ?2)
         ON CONFLICT(id) DO UPDATE SET json = excluded.json, updated_at = excluded.updated_at",
        params![json, now],
    )?;
    Ok(())
}

fn write_page(
    connection: &Connection,
    page: &Page,
    position: usize,
    now: &str,
) -> rusqlite::Result<()> {
    let json = serialize_page(page)?;
    connection.execute(
        "INSERT INTO workspace_pages (id, position, json, updated_at) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(id) DO UPDATE SET
             position = excluded.position,
             json = excluded.json,
             updated_at = excluded.updated_at",
        params![page.id, position as i64, json, now],
    )?;
    Ok(())
}

fn write_all_pages(
    connection: &Connection,
    workspace: &Workspace,
    now: &str,
) -> rusqlite::Result<()> {
    connection.execute("DELETE FROM workspace_pages", [])?;
    for (position, page) in workspace.pages.iter().enumerate() {
        write_page(connection, page, position, now)?;
    }
    Ok(())
}

fn refresh_positions(connection: &Connection, workspace: &Workspace) -> rusqlite::Result<()> {
    for (position, page) in workspace.pages.iter().enumerate() {
        connection.execute(
            "UPDATE workspace_pages SET position = ?1 WHERE id = ?2",
            params![position as i64, page.id],
        )?;
    }
    Ok(())
}

fn write_collab(connection: &Connection, collab: &CollabState, now: &str) -> rusqlite::Result<()> {
    let json = serde_json::to_string(collab)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    connection.execute(
        "INSERT INTO collab (id, json, updated_at) VALUES (1, ?1, ?2)
         ON CONFLICT(id) DO UPDATE SET json = excluded.json, updated_at = excluded.updated_at",
        params![json, now],
    )?;
    Ok(())
}

fn read_pages(connection: &Connection) -> rusqlite::Result<Vec<Page>> {
    let mut statement =
        connection.prepare("SELECT json FROM workspace_pages ORDER BY position, id")?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
    let mut pages = Vec::new();
    for row in rows {
        let json = row?;
        let page: Page = serde_json::from_str(&json).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?;
        pages.push(page);
    }
    Ok(pages)
}

fn migrate_legacy(connection: &mut Connection) -> io::Result<()> {
    let has_meta: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM workspace_meta WHERE id = 1)",
            [],
            |row| row.get(0),
        )
        .map_err(io::Error::other)?;
    if has_meta {
        return Ok(());
    }
    let legacy: Option<String> = connection
        .query_row("SELECT json FROM workspace WHERE id = 1", [], |row| {
            row.get(0)
        })
        .optional()
        .map_err(io::Error::other)?;
    let Some(json) = legacy else {
        return Ok(());
    };
    let workspace: Workspace = serde_json::from_str(&json)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let now = now_iso();
    let transaction = connection.transaction().map_err(io::Error::other)?;
    write_meta(
        &transaction,
        &WorkspaceMeta::from_workspace(&workspace),
        &now,
    )
    .map_err(io::Error::other)?;
    write_all_pages(&transaction, &workspace, &now).map_err(io::Error::other)?;
    transaction.commit().map_err(io::Error::other)?;
    Ok(())
}

impl std::fmt::Debug for SqliteStore {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SqliteStore")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl SqliteStore {
    /// Open (or create) a SQLite database and its schema.
    pub fn open(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }
        let mut connection = Connection::open(&path).map_err(io::Error::other)?;
        connection
            .execute_batch(
                "PRAGMA journal_mode = WAL;
                 PRAGMA foreign_keys = ON;
                 CREATE TABLE IF NOT EXISTS workspace (
                     id INTEGER PRIMARY KEY CHECK (id = 1),
                     json TEXT NOT NULL,
                     updated_at TEXT NOT NULL
                 );
                 CREATE TABLE IF NOT EXISTS workspace_meta (
                     id INTEGER PRIMARY KEY CHECK (id = 1),
                     json TEXT NOT NULL,
                     updated_at TEXT NOT NULL
                 );
                 CREATE TABLE IF NOT EXISTS workspace_pages (
                     id TEXT PRIMARY KEY,
                     position INTEGER NOT NULL,
                     json TEXT NOT NULL,
                     updated_at TEXT NOT NULL
                 );
                 CREATE INDEX IF NOT EXISTS workspace_pages_position
                     ON workspace_pages(position);
                 CREATE TABLE IF NOT EXISTS collab (
                     id INTEGER PRIMARY KEY CHECK (id = 1),
                     json TEXT NOT NULL,
                     updated_at TEXT NOT NULL
                 );
                 CREATE TABLE IF NOT EXISTS accounts (
                     id TEXT PRIMARY KEY,
                     member_id TEXT NOT NULL,
                     email TEXT NOT NULL UNIQUE,
                     password_hash TEXT NOT NULL,
                     created_at TEXT NOT NULL
                 );
                 CREATE TABLE IF NOT EXISTS sessions (
                     token_hash TEXT PRIMARY KEY,
                     member_id TEXT NOT NULL,
                     created_at TEXT NOT NULL,
                     expires_at TEXT NOT NULL
                 );
                 CREATE INDEX IF NOT EXISTS sessions_member
                     ON sessions(member_id);
                 CREATE TABLE IF NOT EXISTS rate_limits (
                     key TEXT PRIMARY KEY,
                     window_start INTEGER NOT NULL,
                     count INTEGER NOT NULL
                 );",
            )
            .map_err(io::Error::other)?;
        migrate_legacy(&mut connection)?;
        Ok(Self {
            path,
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    /// The database file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Persist a workspace snapshot (disambiguated inherent helper).
    pub fn persist_workspace(&self, workspace: &Workspace) -> io::Result<()> {
        <Self as WorkspaceStore>::persist(self, workspace)
    }

    /// Persist a collaboration snapshot (disambiguated inherent helper).
    pub fn persist_collab(&self, state: &CollabState) -> io::Result<()> {
        <Self as CollabStore>::persist(self, state)
    }

    /// Load the workspace document, if present.
    pub fn load_workspace(&self) -> io::Result<Option<Workspace>> {
        self.with_connection(|connection| {
            let meta_json: Option<String> = connection
                .query_row("SELECT json FROM workspace_meta WHERE id = 1", [], |row| {
                    row.get(0)
                })
                .optional()?;
            let Some(meta_json) = meta_json else {
                return Ok(None);
            };
            let meta: WorkspaceMeta = serde_json::from_str(&meta_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?;
            let pages = read_pages(connection)?;
            Ok(Some(meta.into_workspace(pages)))
        })
    }

    /// Load the collaboration document, if present.
    pub fn load_collab(&self) -> io::Result<Option<CollabState>> {
        let json: Option<String> = self.with_connection(|connection| {
            connection
                .query_row("SELECT json FROM collab WHERE id = 1", [], |row| row.get(0))
                .optional()
        })?;
        json.map(|json| {
            serde_json::from_str(&json)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
        })
        .transpose()
    }

    fn with_connection<T>(
        &self,
        operation: impl FnOnce(&Connection) -> rusqlite::Result<T>,
    ) -> io::Result<T> {
        let connection = self
            .connection
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        operation(&connection).map_err(io::Error::other)
    }

    fn with_connection_mut<T>(
        &self,
        operation: impl FnOnce(&mut Connection) -> rusqlite::Result<T>,
    ) -> io::Result<T> {
        let mut connection = self
            .connection
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        operation(&mut connection).map_err(io::Error::other)
    }
}

impl RateLimitStore for SqliteStore {
    fn check(&self, key: &str, window_secs: u64, limit: u32, now: u64) -> io::Result<RateLimitHit> {
        if limit == 0 {
            return Ok(RateLimitHit {
                allowed: true,
                retry_after: 0,
            });
        }
        self.with_connection_mut(|connection| {
            let transaction = connection.transaction()?;
            let row: Option<(u64, u32)> = transaction
                .query_row(
                    "SELECT window_start, count FROM rate_limits WHERE key = ?1",
                    params![key],
                    |row| Ok((row.get::<_, i64>(0)? as u64, row.get::<_, i64>(1)? as u32)),
                )
                .optional()?;
            let (window_start, count) = match row {
                Some((window_start, count)) if now.saturating_sub(window_start) < window_secs => {
                    (window_start, count)
                }
                _ => (now, 0),
            };
            if count >= limit {
                transaction.commit()?;
                return Ok(RateLimitHit {
                    allowed: false,
                    retry_after: window_secs
                        .saturating_sub(now.saturating_sub(window_start))
                        .max(1),
                });
            }
            transaction.execute(
                "INSERT INTO rate_limits (key, window_start, count)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(key) DO UPDATE SET
                     window_start = excluded.window_start,
                     count = excluded.count",
                params![key, window_start as i64, (count + 1) as i64],
            )?;
            transaction.commit()?;
            Ok(RateLimitHit {
                allowed: true,
                retry_after: 0,
            })
        })
    }
}

impl WorkspaceStore for SqliteStore {
    fn persist(&self, workspace: &Workspace) -> io::Result<()> {
        let meta = WorkspaceMeta::from_workspace(workspace);
        let now = now_iso();
        self.with_connection(|connection| {
            write_meta(connection, &meta, &now)?;
            write_all_pages(connection, workspace, &now)
        })?;
        Ok(())
    }

    fn load(&self) -> io::Result<Option<Workspace>> {
        self.load_workspace()
    }
}

impl CollabStore for SqliteStore {
    fn persist(&self, state: &CollabState) -> io::Result<()> {
        let json = serde_json::to_string(state)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        self.with_connection(|connection| {
            connection.execute(
                "INSERT INTO collab (id, json, updated_at) VALUES (1, ?1, ?2)
                 ON CONFLICT(id) DO UPDATE SET json = excluded.json, updated_at = excluded.updated_at",
                params![json, now_iso()],
            )
        })?;
        Ok(())
    }

    fn load(&self) -> io::Result<Option<CollabState>> {
        self.load_collab()
    }
}

impl SnapshotStore for SqliteStore {
    fn persist_snapshots(&self, workspace: &Workspace, collab: &CollabState) -> io::Result<()> {
        let meta = WorkspaceMeta::from_workspace(workspace);
        let now = now_iso();
        self.with_connection_mut(|connection| {
            let transaction = connection.transaction()?;
            write_meta(&transaction, &meta, &now)?;
            write_all_pages(&transaction, workspace, &now)?;
            write_collab(&transaction, collab, &now)?;
            transaction.commit()
        })?;
        Ok(())
    }

    fn persist_page(
        &self,
        workspace: &Workspace,
        collab: &CollabState,
        page_id: &str,
    ) -> io::Result<()> {
        let meta = WorkspaceMeta::from_workspace(workspace);
        let now = now_iso();
        self.with_connection_mut(|connection| {
            let transaction = connection.transaction()?;
            write_meta(&transaction, &meta, &now)?;
            match workspace.pages.iter().position(|page| page.id == page_id) {
                Some(position) => {
                    write_page(&transaction, &workspace.pages[position], position, &now)?
                }
                None => {
                    transaction.execute(
                        "DELETE FROM workspace_pages WHERE id = ?1",
                        params![page_id],
                    )?;
                }
            }
            refresh_positions(&transaction, workspace)?;
            write_collab(&transaction, collab, &now)?;
            transaction.commit()
        })?;
        Ok(())
    }
}

impl AuthStore for SqliteStore {
    fn find_account(&self, email: &str) -> io::Result<Option<Account>> {
        self.with_connection(|connection| {
            connection
                .query_row(
                    "SELECT id, member_id, email, password_hash, created_at
                     FROM accounts WHERE email = ?1 COLLATE NOCASE",
                    params![email],
                    |row| {
                        Ok(Account {
                            id: row.get(0)?,
                            member_id: row.get(1)?,
                            email: row.get(2)?,
                            password_hash: row.get(3)?,
                            created_at: row.get(4)?,
                        })
                    },
                )
                .optional()
        })
    }

    fn upsert_account(&self, account: &Account) -> io::Result<()> {
        self.with_connection(|connection| {
            connection.execute(
                "INSERT INTO accounts (id, member_id, email, password_hash, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(email) DO UPDATE SET
                     member_id = excluded.member_id,
                     password_hash = excluded.password_hash,
                     created_at = excluded.created_at",
                params![
                    account.id,
                    account.member_id,
                    account.email,
                    account.password_hash,
                    account.created_at
                ],
            )
        })?;
        Ok(())
    }

    fn save_session(&self, session: &Session) -> io::Result<()> {
        self.with_connection(|connection| {
            connection.execute(
                "INSERT INTO sessions (token_hash, member_id, created_at, expires_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(token_hash) DO UPDATE SET
                     member_id = excluded.member_id,
                     expires_at = excluded.expires_at",
                params![
                    session.token_hash,
                    session.member_id,
                    session.created_at,
                    session.expires_at
                ],
            )
        })?;
        Ok(())
    }

    fn find_session(&self, token_hash: &str) -> io::Result<Option<Session>> {
        self.with_connection(|connection| {
            connection
                .query_row(
                    "SELECT token_hash, member_id, created_at, expires_at
                     FROM sessions WHERE token_hash = ?1",
                    params![token_hash],
                    |row| {
                        Ok(Session {
                            token_hash: row.get(0)?,
                            member_id: row.get(1)?,
                            created_at: row.get(2)?,
                            expires_at: row.get(3)?,
                        })
                    },
                )
                .optional()
        })
    }

    fn delete_session(&self, token_hash: &str) -> io::Result<()> {
        self.with_connection(|connection| {
            connection.execute(
                "DELETE FROM sessions WHERE token_hash = ?1",
                params![token_hash],
            )
        })?;
        Ok(())
    }

    fn prune_sessions(&self, now: &str) -> io::Result<()> {
        self.with_connection(|connection| {
            connection.execute("DELETE FROM sessions WHERE expires_at <= ?1", params![now])
        })?;
        Ok(())
    }
}
