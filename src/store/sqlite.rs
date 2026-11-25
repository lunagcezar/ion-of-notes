//! SQLite adapters for workspace, collaboration and auth state (spec-039).
//!
//! The first slice stores the existing JSON documents in SQLite, so behavior
//! is preserved while the server gains a durable, transactional backend.
//! Normalized page/block tables are a later spec; the store ports stay the
//! same.

use crate::auth::{Account, AuthStore, Session};
use crate::collab::CollabState;
use crate::model::{
    Block, CommentDto, DataSource, Database, FileUpload, Page, View, Workspace, WorkspaceSettings,
    now_iso,
};
use crate::store::{CollabStore, RateLimitHit, RateLimitStore, SnapshotStore, WorkspaceStore};
use rusqlite::{Connection, OptionalExtension, params};
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

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
    // Blocks live in their own table; the page row keeps only page metadata.
    let mut stored = page.clone();
    stored.blocks.clear();
    serde_json::to_string(&stored)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
}

fn serialize_block(block: &Block) -> rusqlite::Result<String> {
    // Children live in their own rows; keep the block's own fields only.
    let mut stored = block.clone();
    stored.children.clear();
    serde_json::to_string(&stored)
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

fn write_block_row(
    connection: &Connection,
    page_id: &str,
    parent_id: Option<&str>,
    position: usize,
    block: &Block,
    now: &str,
) -> rusqlite::Result<()> {
    let json = serialize_block(block)?;
    connection.execute(
        "INSERT INTO workspace_blocks (id, page_id, parent_id, position, json, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(id) DO UPDATE SET
             page_id = excluded.page_id,
             parent_id = excluded.parent_id,
             position = excluded.position,
             json = excluded.json,
             updated_at = excluded.updated_at",
        params![block.id, page_id, parent_id, position as i64, json, now],
    )?;
    Ok(())
}

fn write_blocks_recursive(
    connection: &Connection,
    page_id: &str,
    parent_id: Option<&str>,
    blocks: &[Block],
    now: &str,
) -> rusqlite::Result<()> {
    for (position, block) in blocks.iter().enumerate() {
        write_block_row(connection, page_id, parent_id, position, block, now)?;
        write_blocks_recursive(
            connection,
            page_id,
            Some(block.id.as_str()),
            &block.children,
            now,
        )?;
    }
    Ok(())
}

fn delete_blocks_for_page(connection: &Connection, page_id: &str) -> rusqlite::Result<()> {
    connection.execute(
        "DELETE FROM workspace_blocks WHERE page_id = ?1",
        params![page_id],
    )?;
    Ok(())
}

fn delete_block_subtree(
    connection: &Connection,
    page_id: &str,
    block_id: &str,
) -> rusqlite::Result<()> {
    let mut stack = vec![block_id.to_string()];
    while let Some(id) = stack.pop() {
        let mut statement = connection
            .prepare("SELECT id FROM workspace_blocks WHERE page_id = ?1 AND parent_id = ?2")?;
        let children: Vec<String> = statement
            .query_map(params![page_id, id], |row| row.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(statement);
        stack.extend(children);
        connection.execute(
            "DELETE FROM workspace_blocks WHERE page_id = ?1 AND id = ?2",
            params![page_id, id],
        )?;
    }
    Ok(())
}

fn find_block_slot<'a>(
    blocks: &'a [Block],
    id: &str,
    parent_id: Option<&str>,
) -> Option<(Option<String>, usize, &'a Block)> {
    for (position, block) in blocks.iter().enumerate() {
        if block.id == id {
            return Some((parent_id.map(str::to_string), position, block));
        }
        if let Some(found) = find_block_slot(&block.children, id, Some(block.id.as_str())) {
            return Some(found);
        }
    }
    None
}
fn write_all_pages(
    connection: &Connection,
    workspace: &Workspace,
    now: &str,
) -> rusqlite::Result<()> {
    connection.execute("DELETE FROM workspace_pages", [])?;
    connection.execute("DELETE FROM workspace_blocks", [])?;
    for (position, page) in workspace.pages.iter().enumerate() {
        write_page(connection, page, position, now)?;
        write_blocks_recursive(connection, &page.id, None, &page.blocks, now)?;
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

struct StoredBlock {
    parent_id: Option<String>,
    position: i64,
    block: Block,
}

fn read_blocks(connection: &Connection) -> rusqlite::Result<HashMap<String, Vec<StoredBlock>>> {
    let mut statement = connection.prepare(
        "SELECT page_id, parent_id, position, json FROM workspace_blocks
         ORDER BY page_id, position, id",
    )?;
    let rows = statement.query_map([], |row| {
        let page_id: String = row.get(0)?;
        let parent_id: Option<String> = row.get(1)?;
        let position: i64 = row.get(2)?;
        let json: String = row.get(3)?;
        let block: Block = serde_json::from_str(&json).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                3,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?;
        Ok((
            page_id,
            StoredBlock {
                parent_id,
                position,
                block,
            },
        ))
    })?;
    let mut by_page: HashMap<String, Vec<StoredBlock>> = HashMap::new();
    for row in rows {
        let (page_id, block) = row?;
        by_page.entry(page_id).or_default().push(block);
    }
    Ok(by_page)
}

fn attach_blocks(page: &mut Page, rows: Vec<StoredBlock>) {
    let mut children: HashMap<Option<String>, Vec<(i64, Block)>> = HashMap::new();
    for row in rows {
        children
            .entry(row.parent_id)
            .or_default()
            .push((row.position, row.block));
    }
    let mut visited = HashSet::new();
    page.blocks = build_block_level(None, &mut children, &mut visited);
}

fn build_block_level(
    parent: Option<&str>,
    children: &mut HashMap<Option<String>, Vec<(i64, Block)>>,
    visited: &mut HashSet<String>,
) -> Vec<Block> {
    let mut level = children
        .remove(&parent.map(str::to_string))
        .unwrap_or_default();
    level.sort_by_key(|(position, _)| *position);
    level
        .into_iter()
        .filter_map(|(_, mut block)| {
            if !visited.insert(block.id.clone()) {
                return None;
            }
            block.children = build_block_level(Some(&block.id), children, visited);
            Some(block)
        })
        .collect()
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

/// Split pages written by older versions into `workspace_blocks` rows.
///
/// The operation is idempotent: after the first run, page rows no longer carry
/// blocks and the loop does nothing.
fn migrate_blocks(connection: &mut Connection) -> io::Result<()> {
    let transaction = connection.transaction().map_err(io::Error::other)?;
    let now = now_iso();
    let mut statement = transaction
        .prepare("SELECT id, position, json FROM workspace_pages ORDER BY position, id")
        .map_err(io::Error::other)?;
    let rows: Vec<(String, i64, String)> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(io::Error::other)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(io::Error::other)?;
    drop(statement);
    for (page_id, position, json) in rows {
        let page: Page = serde_json::from_str(&json)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        if page.blocks.is_empty() {
            continue;
        }
        let has_blocks: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM workspace_blocks WHERE page_id = ?1)",
                params![page_id],
                |row| row.get(0),
            )
            .map_err(io::Error::other)?;
        if !has_blocks {
            write_blocks_recursive(&transaction, &page_id, None, &page.blocks, &now)
                .map_err(io::Error::other)?;
        }
        write_page(&transaction, &page, position as usize, &now).map_err(io::Error::other)?;
    }
    transaction.commit().map_err(io::Error::other)
}

impl std::fmt::Debug for SqliteStore {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SqliteStore")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

/// Apply pragmas and schema, retrying while another instance is switching the
/// journal mode or creating the initial tables.
fn configure_connection(connection: &Connection) -> io::Result<()> {
    const SCHEMA: &str = "PRAGMA journal_mode = WAL;
         PRAGMA busy_timeout = 5000;
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
         CREATE TABLE IF NOT EXISTS workspace_blocks (
             id TEXT PRIMARY KEY,
             page_id TEXT NOT NULL,
             parent_id TEXT,
             position INTEGER NOT NULL,
             json TEXT NOT NULL,
             updated_at TEXT NOT NULL
         );
         CREATE INDEX IF NOT EXISTS workspace_blocks_parent
             ON workspace_blocks(page_id, parent_id, position);
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
         );";
    for attempt in 0..50 {
        match connection.execute_batch(SCHEMA) {
            Ok(()) => return Ok(()),
            Err(rusqlite::Error::SqliteFailure(error, _))
                if error.code == rusqlite::ErrorCode::DatabaseBusy && attempt < 49 =>
            {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(error) => return Err(io::Error::other(error)),
        }
    }
    Err(io::Error::other("SQLite schema setup kept returning busy"))
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
        // Set the busy timeout before the first write pragma: concurrent
        // instance opens otherwise race on `journal_mode = WAL`.
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(io::Error::other)?;
        configure_connection(&connection)?;
        migrate_legacy(&mut connection)?;
        migrate_blocks(&mut connection)?;
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
            let mut pages = read_pages(connection)?;
            let mut blocks = read_blocks(connection)?;
            for page in &mut pages {
                attach_blocks(page, blocks.remove(&page.id).unwrap_or_default());
            }
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
            let transaction =
                connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
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
            let existing_position: Option<i64> = transaction
                .query_row(
                    "SELECT position FROM workspace_pages WHERE id = ?1",
                    params![page_id],
                    |row| row.get(0),
                )
                .optional()?;
            let page_position = workspace.pages.iter().position(|page| page.id == page_id);
            match page_position {
                Some(position) => {
                    let page = &workspace.pages[position];
                    write_page(&transaction, page, position, &now)?;
                    delete_blocks_for_page(&transaction, page_id)?;
                    write_blocks_recursive(&transaction, page_id, None, &page.blocks, &now)?;
                }
                None => {
                    delete_blocks_for_page(&transaction, page_id)?;
                    transaction.execute(
                        "DELETE FROM workspace_pages WHERE id = ?1",
                        params![page_id],
                    )?;
                }
            }
            // A text edit keeps its position. Only a create, delete or reorder
            // needs the O(pages) position refresh.
            let new_position = page_position.map(|position| position as i64);
            if existing_position != new_position {
                refresh_positions(&transaction, workspace)?;
            }
            write_collab(&transaction, collab, &now)?;
            transaction.commit()
        })?;
        Ok(())
    }

    fn persist_block(
        &self,
        workspace: &Workspace,
        collab: &CollabState,
        page_id: &str,
        block_id: &str,
    ) -> io::Result<()> {
        let meta = WorkspaceMeta::from_workspace(workspace);
        let now = now_iso();
        self.with_connection_mut(|connection| {
            let transaction = connection.transaction()?;
            write_meta(&transaction, &meta, &now)?;
            let Some(page_position) = workspace.pages.iter().position(|page| page.id == page_id)
            else {
                // Data-source rows and unknown ids are not part of this table.
                transaction.commit()?;
                return Ok(());
            };
            let page = &workspace.pages[page_position];
            write_page(&transaction, page, page_position, &now)?;
            match find_block_slot(&page.blocks, block_id, None) {
                Some((parent_id, position, block)) => {
                    write_block_row(
                        &transaction,
                        page_id,
                        parent_id.as_deref(),
                        position,
                        block,
                        &now,
                    )?;
                }
                None => delete_block_subtree(&transaction, page_id, block_id)?,
            }
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

    fn delete_member_sessions(&self, member_id: &str) -> io::Result<()> {
        self.with_connection(|connection| {
            connection.execute(
                "DELETE FROM sessions WHERE member_id = ?1",
                params![member_id],
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
