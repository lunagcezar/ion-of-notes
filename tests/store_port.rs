//! Tests for the persistence port (ports-and-adapters / DIP).
//!
//! These are the new tests written first for spec-008: the server must depend
//! on a `WorkspaceStore` abstraction that can be swapped without touching the
//! domain logic or the public constructors.

mod common;

use common::{page_draft_json, remove, temp_path};
use not_notion::model::Workspace;
use not_notion::server::{ApiRequest, Method, WorkspaceApi, WorkspaceServer};
use not_notion::store::{load, JsonStore, NoStore, WorkspaceStore};
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
struct RecordingStore {
    count: Arc<Mutex<usize>>,
}

impl WorkspaceStore for RecordingStore {
    fn persist(&self, _workspace: &Workspace) -> std::io::Result<()> {
        *self.count.lock().unwrap() += 1;
        Ok(())
    }
}

#[test]
fn server_accepts_a_swappable_store_without_touching_disk() {
    let store = RecordingStore::default();
    let mut server = WorkspaceServer::with_store(Workspace::seed("x"), store.clone());

    let created = server.handle(&ApiRequest::new(
        Method::Post,
        "/api/pages",
        Some(page_draft_json("New", None)),
    ));
    assert_eq!(created.status, 201);
    assert_eq!(*store.count.lock().unwrap(), 1);

    let replacement = serde_json::to_string(&Workspace::new("replaced")).unwrap();
    let put = server.handle(&ApiRequest::new(
        Method::Put,
        "/api/workspace",
        Some(replacement),
    ));
    assert_eq!(put.status, 200);
    assert_eq!(*store.count.lock().unwrap(), 2);
    assert_eq!(server.snapshot().name, "replaced");
}

#[test]
fn json_store_persists_to_its_configured_path() {
    let path = temp_path("json-store", "json");
    remove(&path);
    let store = JsonStore::new(path.clone());
    let workspace = Workspace::seed("disk");
    store.persist(&workspace).unwrap();
    assert_eq!(load(&path).unwrap(), workspace);
    assert_eq!(store.path(), path.as_path());
    remove(&path);
}

#[test]
fn no_store_persist_is_a_noop() {
    let store = NoStore;
    assert!(store.persist(&Workspace::seed("x")).is_ok());
}

#[test]
fn with_path_still_persists_after_a_mutation() {
    let path = temp_path("with-path", "json");
    remove(&path);
    let mut server = WorkspaceServer::with_path(Workspace::new("disk"), path.clone());
    let created = server.handle(&ApiRequest::new(
        Method::Post,
        "/api/pages",
        Some(page_draft_json("Persisted", None)),
    ));
    assert_eq!(created.status, 201);
    let on_disk = load(&path).unwrap();
    assert_eq!(on_disk.pages.len(), 1);
    assert_eq!(on_disk.pages[0].title, "Persisted");
    remove(&path);
}
