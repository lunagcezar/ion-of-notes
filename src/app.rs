//! Application composition root (bootstrap).
//!
//! This is the only module that wires concrete adapters to the core ports
//! (Dependency Inversion). It loads or seeds the workspace, builds the
//! `WorkspaceServer` with the JSON persistence adapter, and wraps it in the
//! `Client`. `main.rs` and the desktop window ask this layer for a ready-made
//! object graph instead of assembling it themselves, so the wiring exists
//! once (DRY) and has a single reason to change (SRP).
//!
//! Direction of dependencies: `app -> client -> server -> model` and
//! `server -> store`. The domain never depends on this outer layer, and no
//! egui type appears here.

use crate::client::Client;
use crate::model::Workspace;
use crate::server::WorkspaceServer;
use crate::store;
use std::path::PathBuf;

/// Compose the object graph for the `--serve` loopback API server.
pub fn build_server() -> WorkspaceServer {
    build_server_at(store::data_path())
}

/// Compose a server backed by the JSON file at `path` (a testable seam).
pub fn build_server_at(path: PathBuf) -> WorkspaceServer {
    let workspace = store::load_or_seed(&path);
    WorkspaceServer::with_path(workspace, path)
}

/// Compose the object graph for the desktop window.
pub fn build_desktop() -> (Client<WorkspaceServer>, Workspace) {
    build_desktop_at(store::data_path())
}

/// Compose a desktop client backed by the JSON file at `path`.
pub fn build_desktop_at(path: PathBuf) -> (Client<WorkspaceServer>, Workspace) {
    let loaded = store::load_or_seed(&path);
    let mut client = Client::new(WorkspaceServer::with_path(loaded.clone(), path));
    let workspace = client.workspace().unwrap_or(loaded);
    (client, workspace)
}
