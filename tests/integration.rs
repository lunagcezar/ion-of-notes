//! End-to-end scenarios over the real client, server, and JSON store.

mod common;

use common::{remove, temp_path};
use not_notion::client::Client;
use not_notion::model::{PageDraft, Workspace};
use not_notion::server::WorkspaceServer;
use not_notion::store;

#[test]
fn full_workspace_lifecycle_is_persisted() {
    let path = temp_path("lifecycle", "json");
    remove(&path);
    let server = WorkspaceServer::with_path(Workspace::seed("integration"), path.clone());
    let mut client = Client::new(server);

    let created = client
        .create_page(&PageDraft {
            title: "Sprint notes".to_string(),
            icon: "S".to_string(),
            parent: None,
        })
        .unwrap();

    let mut page = created.clone();
    page.title = "Sprint 12 notes".to_string();
    client.update_page(&page).unwrap();

    let results = client.search("SPRINT 12").unwrap();
    assert_eq!(results[0].page_id, created.id);

    client.delete_page(&created.id).unwrap();
    assert!(client.get_page(&created.id).is_err());

    let on_disk = store::load(&path).unwrap();
    assert!(on_disk.pages.iter().all(|p| p.id != created.id));
    remove(&path);
}
