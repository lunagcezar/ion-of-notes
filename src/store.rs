//! JSON persistence and the persistence **port** of the application.
//!
//! The free functions below are the low-level JSON file adapter. Above them
//! sits `WorkspaceStore`, the abstraction the server depends on (ports and
//! adapters / Dependency Inversion): swapping the JSON file for another
//! backend means writing a new adapter, not editing the server.

use crate::model::Workspace;
use std::fs;
use std::io;
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
    base.join("not-notion").join("workspace.json")
}

/// Persistence port. Implementations decide where a workspace snapshot goes.
/// The trait is deliberately small (Interface Segregation): the core only
/// needs to persist a snapshot after a successful mutation.
pub trait WorkspaceStore {
    /// Persist one full workspace snapshot.
    fn persist(&self, workspace: &Workspace) -> io::Result<()>;
}

/// Null-object adapter: keeps a server purely in memory (tests, previews).
#[derive(Debug, Clone, Copy, Default)]
pub struct NoStore;

impl WorkspaceStore for NoStore {
    fn persist(&self, _workspace: &Workspace) -> io::Result<()> {
        Ok(())
    }
}

/// JSON-file adapter: the default persistence backend.
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
}

/// Write a workspace snapshot as pretty-printed JSON.
pub fn save(path: &Path, workspace: &Workspace) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(workspace)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(path, json)
}

/// Read a workspace snapshot from JSON.
pub fn load(path: &Path) -> io::Result<Workspace> {
    let text = fs::read_to_string(path)?;
    serde_json::from_str(&text).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Load the workspace, or create and save a neutral demo workspace.
pub fn load_or_seed(path: &Path) -> Workspace {
    match load(path) {
        Ok(workspace) => workspace,
        Err(_) => {
            let workspace = Workspace::seed("not-notion");
            let _ = save(path, &workspace);
            workspace
        }
    }
}
