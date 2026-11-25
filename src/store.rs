//! JSON persistence and the persistence **ports** of the application.
//!
//! `WorkspaceStore` and `CollabStore` are the abstractions the server depends
//! on (ports and adapters / Dependency Inversion). `JsonStore` and
//! `JsonCollabStore` are the default file adapters; `NoStore` and
//! `NoCollabStore` are null objects for in-memory servers and tests. Swapping
//! the JSON files for another backend means writing new adapters, not editing
//! the server.

use crate::collab::CollabState;
use crate::model::Workspace;
use serde::Serialize;
use serde::de::DeserializeOwned;

pub mod sqlite;
pub use sqlite::SqliteStore;
#[cfg(feature = "postgres")]
pub mod postgres;
#[cfg(feature = "postgres")]
pub use postgres::PostgresStore;
#[cfg(feature = "redis")]
pub mod redis_store;
#[cfg(feature = "redis")]
pub use redis_store::RedisRateLimitStore;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// The default location of the JSON workspace file.
///
/// This is an infrastructure decision owned by the persistence adapter, not
/// by the presentation layer; the composition root (`app`) is its only caller.
pub fn data_path() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("ion-of-notes").join("workspace.json")
}

/// The default location of the JSON collaboration document.
///
/// Kept beside `workspace.json` but in its own file, so a corrupt
/// collaboration document cannot destroy pages (spec-019).
pub fn collab_path() -> PathBuf {
    data_path().with_file_name("collab.json")
}

/// The default location of the multi-workspace catalog (spec-058).
pub fn catalog_path() -> PathBuf {
    data_path().with_file_name("workspaces.json")
}

/// Persistence port for workspace snapshots.
///
/// The trait is deliberately small (Interface Segregation): the core only
/// needs to persist a snapshot after a successful mutation.
pub trait WorkspaceStore: Send + Sync {
    /// Persist one full workspace snapshot.
    fn persist(&self, workspace: &Workspace) -> io::Result<()>;

    /// Reload the last durable workspace snapshot, when the adapter supports it.
    fn load(&self) -> io::Result<Option<Workspace>> {
        Ok(None)
    }
}

/// Persistence port for collaboration state.
pub trait CollabStore: Send + Sync {
    /// Persist one full collaboration snapshot.
    fn persist(&self, state: &CollabState) -> io::Result<()>;

    /// Reload the last durable collaboration snapshot, when supported.
    fn load(&self) -> io::Result<Option<CollabState>> {
        Ok(None)
    }
}

/// Atomic snapshot port for adapters that can commit both documents together.
pub trait SnapshotStore: Send + Sync {
    /// Persist workspace and collaboration state in one durable commit.
    fn persist_snapshots(&self, workspace: &Workspace, collab: &CollabState) -> io::Result<()>;

    /// Persist one page (plus workspace metadata and collaboration state).
    ///
    /// Adapters that store whole documents can use the default full snapshot.
    fn persist_page(
        &self,
        workspace: &Workspace,
        collab: &CollabState,
        page_id: &str,
    ) -> io::Result<()> {
        let _ = page_id;
        self.persist_snapshots(workspace, collab)
    }

    /// Persist one block plus its page metadata and collaboration state.
    ///
    /// Adapters without a block table can fall back to the page write.
    fn persist_block(
        &self,
        workspace: &Workspace,
        collab: &CollabState,
        page_id: &str,
        block_id: &str,
    ) -> io::Result<()> {
        let _ = block_id;
        self.persist_page(workspace, collab, page_id)
    }
}

/// One shared rate-limit decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimitHit {
    pub allowed: bool,
    pub retry_after: u64,
}

/// Shared rate-limit store (in-process or SQLite today; Redis later).
pub trait RateLimitStore: Send + Sync {
    /// Account for one request under `key` in a fixed `window_secs` window.
    fn check(&self, key: &str, window_secs: u64, limit: u32, now: u64) -> io::Result<RateLimitHit>;
}

/// Null-object adapter: keeps a server purely in memory (tests, previews).
#[derive(Debug, Clone, Copy, Default)]
pub struct NoStore;

impl WorkspaceStore for NoStore {
    fn persist(&self, _workspace: &Workspace) -> io::Result<()> {
        Ok(())
    }
}

/// Null-object adapter for collaboration state.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoCollabStore;

impl CollabStore for NoCollabStore {
    fn persist(&self, _state: &CollabState) -> io::Result<()> {
        Ok(())
    }
}

/// JSON-file adapter for workspace snapshots.
#[derive(Debug, Clone)]
pub struct JsonStore {
    path: PathBuf,
}

impl JsonStore {
    /// Create an adapter that persists to `path`.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// The file this adapter writes to.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl WorkspaceStore for JsonStore {
    fn persist(&self, workspace: &Workspace) -> io::Result<()> {
        save(&self.path, workspace)
    }

    fn load(&self) -> io::Result<Option<Workspace>> {
        match load(&self.path) {
            Ok(workspace) => Ok(Some(workspace)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }
}

/// JSON-file adapter for collaboration state.
#[derive(Debug, Clone)]
pub struct JsonCollabStore {
    path: PathBuf,
}

impl JsonCollabStore {
    /// Create an adapter that persists to `path`.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// The file this adapter writes to.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl CollabStore for JsonCollabStore {
    fn persist(&self, state: &CollabState) -> io::Result<()> {
        save_collab(&self.path, state)
    }

    fn load(&self) -> io::Result<Option<CollabState>> {
        match load_collab(&self.path) {
            Ok(collab) => Ok(Some(collab)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }
}

/// Write a JSON document through a sibling temporary file, then move it into
/// place. An interrupted save cannot leave a truncated target behind.
fn write_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("tmp");
    {
        let file = fs::File::create(&temporary)?;
        let mut writer = io::BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, value)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        writer.flush()?;
    }
    match fs::rename(&temporary, path) {
        Ok(()) => Ok(()),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::AlreadyExists | io::ErrorKind::PermissionDenied
            ) =>
        {
            // Some platforms cannot replace an existing file in one rename.
            // Keep the previous document as a sibling backup until the new
            // file is in place; never copy over the live file in place.
            let backup = path.with_extension("bak");
            let _ = fs::remove_file(&backup);
            if path.exists() {
                fs::rename(path, &backup)?;
            }
            match fs::rename(&temporary, path) {
                Ok(()) => {
                    let _ = fs::remove_file(&backup);
                    Ok(())
                }
                Err(error) => {
                    let _ = fs::rename(&backup, path);
                    Err(error)
                }
            }
        }
        Err(error) => Err(error),
    }
}

/// Read one JSON document.
fn read_json<T: DeserializeOwned>(path: &Path) -> io::Result<T> {
    let text = fs::read_to_string(path)?;
    serde_json::from_str(&text).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Write a workspace snapshot as pretty-printed JSON.
pub fn save(path: &Path, workspace: &Workspace) -> io::Result<()> {
    write_json(path, workspace)
}

/// Read a workspace snapshot from JSON.
pub fn load(path: &Path) -> io::Result<Workspace> {
    let mut workspace: Workspace = read_json(path)?;
    workspace.normalize_legacy_icons();
    Ok(workspace)
}

/// Load the workspace, or create and save a neutral demo workspace.
///
/// Only a missing file seeds a new workspace. A corrupt, unreadable or
/// permission-denied file is an error: overwriting it would destroy the only
/// recovery evidence.
pub fn load_or_seed(path: &Path) -> io::Result<Workspace> {
    match load(path) {
        Ok(workspace) => Ok(workspace),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let workspace = Workspace::seed("ion-of-notes");
            save(path, &workspace)?;
            Ok(workspace)
        }
        Err(error) => Err(error),
    }
}

/// Load the workspace catalog, migrating the current single workspace when the
/// catalog file does not exist yet (spec-058).
pub fn load_catalog_or_seed(
    path: &Path,
    workspace_path: &Path,
) -> io::Result<crate::model::WorkspaceCatalog> {
    match read_json::<crate::model::WorkspaceCatalog>(path) {
        Ok(catalog) if !catalog.workspaces.is_empty() => {
            let mut catalog = catalog;
            for workspace in &mut catalog.workspaces {
                workspace.normalize_legacy_icons();
            }
            Ok(catalog)
        }
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "workspace catalog is empty",
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let workspace = load_or_seed(workspace_path)?;
            let catalog = crate::model::WorkspaceCatalog::new(workspace);
            save_catalog(path, &catalog)?;
            Ok(catalog)
        }
        Err(error) => Err(error),
    }
}

/// Write the workspace catalog as pretty-printed JSON.
pub fn save_catalog(path: &Path, catalog: &crate::model::WorkspaceCatalog) -> io::Result<()> {
    write_json(path, catalog)
}

/// Write a collaboration snapshot as pretty-printed JSON.
pub fn save_collab(path: &Path, state: &CollabState) -> io::Result<()> {
    write_json(path, state)
}

/// Read a collaboration snapshot from JSON.
pub fn load_collab(path: &Path) -> io::Result<CollabState> {
    read_json(path)
}

/// Load collaboration state, or create and save the documented seed when the
/// file does not exist.
///
/// Any other error (corrupt JSON, permissions, I/O) is returned and the
/// existing file is left untouched (spec-139); it must never be replaced by a
/// fresh empty collaboration document.
pub fn load_collab_or_seed(path: &Path) -> io::Result<CollabState> {
    match load_collab(path) {
        Ok(state) => Ok(state),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let state = CollabState::new();
            save_collab(path, &state)?;
            Ok(state)
        }
        Err(error) => Err(error),
    }
}
