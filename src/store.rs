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

/// Persistence port for workspace snapshots.
///
/// The trait is deliberately small (Interface Segregation): the core only
/// needs to persist a snapshot after a successful mutation.
pub trait WorkspaceStore: Send + Sync {
    /// Persist one full workspace snapshot.
    fn persist(&self, workspace: &Workspace) -> io::Result<()>;
}

/// Persistence port for collaboration state.
pub trait CollabStore: Send + Sync {
    /// Persist one full collaboration snapshot.
    fn persist(&self, state: &CollabState) -> io::Result<()>;
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
        Err(_) => {
            fs::copy(&temporary, path)?;
            fs::remove_file(&temporary)?;
            Ok(())
        }
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
    read_json(path)
}

/// Load the workspace, or create and save a neutral demo workspace.
pub fn load_or_seed(path: &Path) -> Workspace {
    match load(path) {
        Ok(workspace) => workspace,
        Err(_) => {
            let workspace = Workspace::seed("ion-of-notes");
            let _ = save(path, &workspace);
            workspace
        }
    }
}

/// Write a collaboration snapshot as pretty-printed JSON.
pub fn save_collab(path: &Path, state: &CollabState) -> io::Result<()> {
    write_json(path, state)
}

/// Read a collaboration snapshot from JSON.
pub fn load_collab(path: &Path) -> io::Result<CollabState> {
    read_json(path)
}

/// Load collaboration state, or create and save the documented seed.
pub fn load_collab_or_seed(path: &Path) -> CollabState {
    match load_collab(path) {
        Ok(state) => state,
        Err(_) => {
            let state = CollabState::new();
            let _ = save_collab(path, &state);
            state
        }
    }
}
