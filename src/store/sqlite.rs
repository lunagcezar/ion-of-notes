//! SQLite adapters for workspace, collaboration and auth state (spec-039).
//!
//! The first slice stores the existing JSON documents in SQLite, so behavior
//! is preserved while the server gains a durable, transactional backend.
//! Normalized page/block tables are a later spec; the store ports stay the
//! same.

use crate::auth::{Account, AuthStore, Session};
use crate::collab::CollabState;
use crate::model::{Workspace, now_iso};
use crate::store::{CollabStore, WorkspaceStore};
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
        let connection = Connection::open(&path).map_err(io::Error::other)?;
        connection
            .execute_batch(
                "PRAGMA journal_mode = WAL;
                 PRAGMA foreign_keys = ON;
                 CREATE TABLE IF NOT EXISTS workspace (
                     id INTEGER PRIMARY KEY CHECK (id = 1),
                     json TEXT NOT NULL,
                     updated_at TEXT NOT NULL
                 );
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
                     ON sessions(member_id);",
            )
            .map_err(io::Error::other)?;
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
        let json: Option<String> = self.with_connection(|connection| {
            connection
                .query_row("SELECT json FROM workspace WHERE id = 1", [], |row| {
                    row.get(0)
                })
                .optional()
        })?;
        json.map(|json| {
            serde_json::from_str(&json)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
        })
        .transpose()
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
}

impl WorkspaceStore for SqliteStore {
    fn persist(&self, workspace: &Workspace) -> io::Result<()> {
        let json = serde_json::to_string(workspace)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        self.with_connection(|connection| {
            connection.execute(
                "INSERT INTO workspace (id, json, updated_at) VALUES (1, ?1, ?2)
                 ON CONFLICT(id) DO UPDATE SET json = excluded.json, updated_at = excluded.updated_at",
                params![json, now_iso()],
            )
        })?;
        Ok(())
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
