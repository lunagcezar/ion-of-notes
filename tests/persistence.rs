//! Persistence integrity tests for atomic workspace/collab commits (spec-074).

mod common;

use ion_of_notes::collab::CollabState;
use ion_of_notes::model::Workspace;
use ion_of_notes::server::{ApiRequest, Method, WorkspaceApi, WorkspaceServer};
use ion_of_notes::store::sqlite::SqliteStore;
use ion_of_notes::store::{CollabStore, SnapshotStore, WorkspaceStore};
use std::io;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct MemorySnapshotStore {
    state: Arc<Mutex<(Workspace, CollabState)>>,
    fail: Arc<Mutex<bool>>,
}

impl MemorySnapshotStore {
    fn new(workspace: Workspace, collab: CollabState) -> Self {
        Self {
            state: Arc::new(Mutex::new((workspace, collab))),
            fail: Arc::new(Mutex::new(false)),
        }
    }

    fn set_fail(&self, fail: bool) {
        *self.fail.lock().expect("lock") = fail;
    }

    fn state(&self) -> (Workspace, CollabState) {
        self.state.lock().expect("lock").clone()
    }
}

impl WorkspaceStore for MemorySnapshotStore {
    fn persist(&self, workspace: &Workspace) -> io::Result<()> {
        self.state.lock().expect("lock").0 = workspace.clone();
        Ok(())
    }

    fn load(&self) -> io::Result<Option<Workspace>> {
        Ok(Some(self.state.lock().expect("lock").0.clone()))
    }
}

impl CollabStore for MemorySnapshotStore {
    fn persist(&self, collab: &CollabState) -> io::Result<()> {
        self.state.lock().expect("lock").1 = collab.clone();
        Ok(())
    }

    fn load(&self) -> io::Result<Option<CollabState>> {
        Ok(Some(self.state.lock().expect("lock").1.clone()))
    }
}

impl SnapshotStore for MemorySnapshotStore {
    fn persist_snapshots(&self, workspace: &Workspace, collab: &CollabState) -> io::Result<()> {
        if *self.fail.lock().expect("lock") {
            return Err(io::Error::other("injected snapshot failure"));
        }
        let mut state = self.state.lock().expect("lock");
        state.0 = workspace.clone();
        state.1 = collab.clone();
        Ok(())
    }
}

fn server_with_snapshot_store(
    workspace: Workspace,
    collab: CollabState,
) -> (WorkspaceServer, MemorySnapshotStore) {
    let store = MemorySnapshotStore::new(workspace.clone(), collab.clone());
    let mut server = WorkspaceServer::with_stores(workspace, collab, store.clone(), store.clone());
    server.use_snapshot_store(store.clone());
    (server, store)
}

#[test]
fn atomic_snapshot_store_receives_both_documents() {
    let (mut server, store) =
        server_with_snapshot_store(Workspace::seed("atomic"), CollabState::new());
    let request = ApiRequest::new(
        Method::Post,
        "/api/pages",
        Some(common::page_draft_json("Saved atomically", None)),
    );
    let response = server.handle(&request);
    assert_eq!(response.status, 201);

    let (workspace, collab) = store.state();
    assert!(
        workspace
            .pages
            .iter()
            .any(|page| page.title == "Saved atomically")
    );
    assert!(!collab.changes.is_empty());
}

#[test]
fn failed_snapshot_write_reloads_the_last_durable_state() {
    let (mut server, store) =
        server_with_snapshot_store(Workspace::seed("atomic"), CollabState::new());
    store.set_fail(true);
    let request = ApiRequest::new(
        Method::Post,
        "/api/pages",
        Some(common::page_draft_json("Lost page", None)),
    );
    let response = server.handle(&request);
    assert_eq!(response.status, 500);
    assert!(
        server
            .workspace
            .pages
            .iter()
            .all(|page| page.title != "Lost page"),
        "failed persistence must not leave the page in memory"
    );
}

#[test]
fn sqlite_persist_snapshots_roundtrips_both_documents() {
    let db = common::temp_path("atomic-sqlite", "sqlite3");
    common::remove(&db);
    let store = SqliteStore::open(&db).expect("open");
    let workspace = Workspace::seed("atomic-sqlite");
    let collab = CollabState::new();

    store
        .persist_snapshots(&workspace, &collab)
        .expect("persist snapshots");

    assert_eq!(
        store.load_workspace().expect("workspace").expect("present"),
        workspace
    );
    assert_eq!(
        store.load_collab().expect("collab").expect("present"),
        collab
    );
    common::remove(&db);
}
