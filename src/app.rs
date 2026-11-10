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

use crate::auth::{self, Account, AuthStore};
use crate::client::Client;
use crate::collab::{CollabState, Role};
use crate::model::{Workspace, now_iso};
use crate::remote::RemoteApi;
use crate::server::realtime::{Awareness, ServerMessage};
use crate::server::{ApiRequest, ApiResponse, WorkspaceApi, WorkspaceServer};
use crate::store::{self, SqliteStore};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The desktop transport: local in-process server or remote HTTP/WebSocket.
pub enum DesktopApi {
    Local(Box<WorkspaceServer>),
    Remote(RemoteApi),
}

impl WorkspaceApi for DesktopApi {
    fn handle(&mut self, request: &ApiRequest) -> ApiResponse {
        match self {
            DesktopApi::Local(server) => server.handle(request),
            DesktopApi::Remote(remote) => remote.handle(request),
        }
    }

    fn crdt_sync(
        &mut self,
        page_id: &str,
        state_vector: Option<&[u8]>,
    ) -> Result<(Vec<u8>, Vec<u8>), String> {
        match self {
            DesktopApi::Local(server) => server.crdt_sync(page_id, state_vector),
            DesktopApi::Remote(remote) => remote.crdt_sync(page_id, state_vector),
        }
    }

    fn apply_crdt_update(
        &mut self,
        page_id: &str,
        update: &[u8],
        actor: &str,
    ) -> Result<Vec<u8>, String> {
        match self {
            DesktopApi::Local(server) => server.apply_crdt_update(page_id, update, actor),
            DesktopApi::Remote(remote) => remote.apply_crdt_update(page_id, update, actor),
        }
    }

    fn subscribe_realtime(&self) -> Option<tokio::sync::broadcast::Receiver<ServerMessage>> {
        match self {
            DesktopApi::Local(server) => server.subscribe_realtime(),
            DesktopApi::Remote(remote) => remote.subscribe_realtime(),
        }
    }

    fn publish_awareness(&mut self, awareness: Awareness) -> Result<(), String> {
        match self {
            DesktopApi::Local(server) => server.publish_awareness(awareness),
            DesktopApi::Remote(remote) => remote.publish_awareness(awareness),
        }
    }
}

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

/// Compose and run the Axum API server for `--serve`.
///
/// The desktop binary and the server-only binary share this wiring, so the
/// JSON/SQLite choice and the listener startup exist once.
pub fn serve(addr: &str, database: Option<PathBuf>) -> io::Result<()> {
    let server = match database {
        Some(database) => build_server_sqlite(database)?,
        None => build_server(),
    };
    crate::server::serve_axum(addr, server)
}

/// Compose a server backed by SQLite, migrating the default JSON documents
/// when the database is empty.
pub fn build_server_sqlite(database: PathBuf) -> io::Result<WorkspaceServer> {
    build_server_sqlite_at(database, store::data_path(), store::collab_path())
}

/// Compose a SQLite server with explicit JSON migration sources.
pub fn build_server_sqlite_at(
    database: PathBuf,
    workspace_path: PathBuf,
    collab_path: PathBuf,
) -> io::Result<WorkspaceServer> {
    let sqlite = SqliteStore::open(&database)?;
    let workspace = match sqlite.load_workspace()? {
        Some(workspace) => workspace,
        None => {
            let workspace = store::load_or_seed(&workspace_path);
            sqlite.persist_workspace(&workspace)?;
            workspace
        }
    };
    let collab = match sqlite.load_collab()? {
        Some(collab) => collab,
        None => {
            let collab = store::load_collab_or_seed(&collab_path);
            sqlite.persist_collab(&collab)?;
            collab
        }
    };
    seed_owner_account(&sqlite, &collab)?;
    let shared_rate_store = Arc::new(sqlite.clone());
    let mut server = WorkspaceServer::with_stores_and_auth(
        workspace,
        collab,
        sqlite.clone(),
        sqlite.clone(),
        sqlite,
    );
    server.use_shared_rate_limits(shared_rate_store);
    Ok(server)
}

fn seed_owner_account(store: &SqliteStore, collab: &CollabState) -> io::Result<()> {
    let Some(owner) = collab
        .members
        .iter()
        .find(|member| member.role == Role::Owner)
    else {
        return Ok(());
    };
    let email = owner.email.to_lowercase();
    if store.find_account(&email)?.is_some() {
        return Ok(());
    }
    let password = std::env::var("ION_OF_NOTES_OWNER_PASSWORD")
        .unwrap_or_else(|_| auth::DEFAULT_OWNER_PASSWORD.to_string());
    let password_hash = auth::hash_password(&password).map_err(io::Error::other)?;
    store.upsert_account(&Account {
        id: format!("account-{}", owner.id),
        member_id: owner.id.clone(),
        email,
        password_hash,
        created_at: now_iso(),
    })?;
    if std::env::var("ION_OF_NOTES_OWNER_PASSWORD").is_err() {
        eprintln!(
            "warning: seeded the owner account with the development password; set ION_OF_NOTES_OWNER_PASSWORD"
        );
    }
    Ok(())
}

/// Compose the object graph for the desktop window.
pub fn build_desktop() -> (Client<DesktopApi>, Workspace) {
    build_desktop_at_paths(store::data_path(), store::collab_path())
}

/// Compose a remote desktop client over HTTP and WebSocket.
pub fn build_remote_desktop(
    base_url: &str,
    token: &str,
) -> Result<(Client<DesktopApi>, Workspace), String> {
    let remote = RemoteApi::connect(base_url, token)?;
    let mut client = Client::with_token(DesktopApi::Remote(remote), token);
    let workspace = client.workspace().map_err(|error| error.to_string())?;
    Ok((client, workspace))
}

/// Compose a desktop client backed by the JSON files derived from `path`.
pub fn build_desktop_at(path: PathBuf) -> (Client<DesktopApi>, Workspace) {
    let collab_path = collab_path_for(&path);
    build_desktop_at_paths(path, collab_path)
}

/// Compose a desktop client backed by explicit workspace and collaboration
/// paths.
pub fn build_desktop_at_paths(
    workspace_path: PathBuf,
    collab_path: PathBuf,
) -> (Client<DesktopApi>, Workspace) {
    let loaded = store::load_or_seed(&workspace_path);
    let mut client = Client::new(DesktopApi::Local(Box::new(WorkspaceServer::with_paths(
        loaded.clone(),
        workspace_path,
        collab_path,
    ))));
    let workspace = client.workspace().unwrap_or(loaded);
    (client, workspace)
}
