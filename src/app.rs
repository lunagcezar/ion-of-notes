//! Application composition root (bootstrap).
//!
//! This is the only module that wires concrete adapters to the core ports
//! (Dependency Inversion). It loads or seeds the workspace and collaboration
//! documents, builds the `WorkspaceServer` with both JSON persistence
//! adapters, and wraps it in the `Client`. `main.rs` and the desktop window
//! ask this layer for a ready-made object graph instead of assembling it
//! themselves, so the wiring exists once (DRY) and has a single reason to
//! change (SRP).
//!
//! Direction of dependencies: `app -> client -> server -> model` and
//! `server -> store`. The domain never depends on this outer layer, and no
//! egui type appears here.

use crate::client::Client;
use crate::model::Workspace;
use crate::server::WorkspaceServer;
use crate::store;
use std::path::{Path, PathBuf};

/// Derive a collaboration path for a caller-supplied workspace path.
///
/// This keeps `build_*_at` testable and isolated: `workspace.json` becomes
/// `workspace.collab.json`, while production uses `store::collab_path()`.
fn collab_path_for(workspace_path: &Path) -> PathBuf {
    workspace_path.with_extension("collab.json")
}

/// Compose the object graph for the `--serve` loopback API server.
pub fn build_server() -> WorkspaceServer {
    build_server_at_paths(store::data_path(), store::collab_path())
}

/// Compose a server backed by the JSON files derived from `path`.
pub fn build_server_at(path: PathBuf) -> WorkspaceServer {
    let collab_path = collab_path_for(&path);
    build_server_at_paths(path, collab_path)
}

/// Compose a server backed by explicit workspace and collaboration paths.
pub fn build_server_at_paths(workspace_path: PathBuf, collab_path: PathBuf) -> WorkspaceServer {
    let workspace = store::load_or_seed(&workspace_path);
    WorkspaceServer::with_paths(workspace, workspace_path, collab_path)
}

/// Compose the object graph for the desktop window.
pub fn build_desktop() -> (Client<WorkspaceServer>, Workspace) {
    build_desktop_at_paths(store::data_path(), store::collab_path())
}

/// Compose a desktop client backed by the JSON files derived from `path`.
pub fn build_desktop_at(path: PathBuf) -> (Client<WorkspaceServer>, Workspace) {
    let collab_path = collab_path_for(&path);
    build_desktop_at_paths(path, collab_path)
}

/// Compose a desktop client backed by explicit workspace and collaboration
/// paths.
pub fn build_desktop_at_paths(
    workspace_path: PathBuf,
    collab_path: PathBuf,
) -> (Client<WorkspaceServer>, Workspace) {
    let loaded = store::load_or_seed(&workspace_path);
    let mut client = Client::new(WorkspaceServer::with_paths(
        loaded.clone(),
        workspace_path,
        collab_path,
    ));
    let workspace = client.workspace().unwrap_or(loaded);
    (client, workspace)
}
