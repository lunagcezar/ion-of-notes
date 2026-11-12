//! Tests for the application composition root (public API only).
//!
//! The review asks for an `app` / bootstrap layer that connects concrete
//! adapters to the core ports. These tests exercise it through a supplied
//! path so no real user data file is touched (deterministic, DRY helpers).

mod common;

use common::{remove, temp_path};
use ion_of_notes::app::{build_desktop_at, build_desktop_catalog_at_paths, build_server_at};
use ion_of_notes::server::{ApiRequest, Method, RequestTransport};

#[test]
fn build_desktop_at_seeds_and_serves_the_workspace() {
    let path = temp_path("bootstrap-desktop", "json");
    let collab = path.with_extension("collab.json");
    remove(&path);
    remove(&collab);
    let (mut client, workspace) = build_desktop_at(path.clone());
    assert!(!workspace.pages.is_empty(), "the demo workspace is seeded");
    assert_eq!(client.workspace().unwrap(), workspace);
    remove(&path);
    remove(&collab);
}

#[test]
fn replace_workspace_moves_the_local_server_to_the_new_snapshot() {
    let path = temp_path("bootstrap-replace", "json");
    let collab = path.with_extension("collab.json");
    remove(&path);
    remove(&collab);
    let (mut client, workspace) = build_desktop_at(path.clone());
    let mut second = ion_of_notes::model::Workspace::seed("Second");
    second.settings.id = "second".to_string();
    client.replace_workspace(&second).unwrap();
    let loaded = client.workspace().unwrap();
    assert_eq!(loaded.name, "Second");
    assert_eq!(loaded.settings.id, "second");
    assert_ne!(workspace.name, loaded.name);
    remove(&path);
    remove(&collab);
}

#[test]
fn build_desktop_catalog_at_paths_returns_an_active_catalog() {
    let path = temp_path("bootstrap-catalog", "json");
    let collab = path.with_extension("collab.json");
    let catalog_path = path.with_file_name("workspaces.json");
    remove(&path);
    remove(&collab);
    remove(&catalog_path);
    let (mut client, workspace, catalog) =
        build_desktop_catalog_at_paths(path.clone(), collab.clone());
    assert_eq!(catalog.summaries().len(), 1);
    assert!(catalog.summaries()[0].active);
    assert_eq!(client.workspace().unwrap().name, workspace.name);
    remove(&path);
    remove(&collab);
    remove(&catalog_path);
}

#[cfg(not(feature = "desktop"))]
#[test]
fn core_build_exposes_the_data_stack_without_the_desktop_ui() {
    fn assert_api<T: RequestTransport>() {}
    assert_api::<ion_of_notes::server::WorkspaceServer>();
    let _ = ion_of_notes::app::build_server;
}

#[test]
fn build_server_at_answers_health() {
    let path = temp_path("bootstrap-server", "json");
    let collab = path.with_extension("collab.json");
    remove(&path);
    remove(&collab);
    let mut server = build_server_at(path.clone());
    let response = server.handle(&ApiRequest::new(Method::Get, "/health", None));
    assert_eq!(response.status, 200);
    remove(&path);
    remove(&collab);
}
