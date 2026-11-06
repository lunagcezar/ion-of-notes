//! Tests for the client layer (public API only).

use not_notion::client::{Client, ClientError};
use not_notion::model::{Page, PageDraft, Workspace};
use not_notion::server::{ApiRequest, ApiResponse, WorkspaceApi, WorkspaceServer};

struct Stub {
    status: u16,
    body: String,
}

impl WorkspaceApi for Stub {
    fn handle(&mut self, _req: &ApiRequest) -> ApiResponse {
        ApiResponse {
            status: self.status,
            body: self.body.clone(),
        }
    }
}

fn server() -> WorkspaceServer {
    WorkspaceServer::new(Workspace::seed("client-tests"))
}

#[test]
fn workspace_matches_server_state() {
    let expected = Workspace::seed("client-tests");
    let mut c = Client::new(server());
    assert_eq!(c.workspace().unwrap(), expected);
}

#[test]
fn create_then_get_page() {
    let mut c = Client::new(server());
    let page = c
        .create_page(&PageDraft {
            title: "Fresh".to_string(),
            icon: "F".to_string(),
            parent: None,
        })
        .unwrap();
    assert!(!page.id.is_empty());
    assert_eq!(c.get_page(&page.id).unwrap().title, "Fresh");
}

#[test]
fn update_page_changes_title() {
    let mut c = Client::new(server());
    let mut page = c.workspace().unwrap().pages[0].clone();
    page.title = "Renamed".to_string();
    c.update_page(&page).unwrap();
    assert_eq!(c.get_page(&page.id).unwrap().title, "Renamed");
}

#[test]
fn delete_page_makes_it_unavailable() {
    let mut c = Client::new(server());
    let id = c.workspace().unwrap().pages[0].id.clone();
    c.delete_page(&id).unwrap();
    assert!(matches!(c.get_page(&id), Err(ClientError::NotFound(_))));
}

#[test]
fn search_results_decode() {
    let mut c = Client::new(server());
    let results = c.search("getting").unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].page_title, "Getting Started");
}

#[test]
fn http_error_maps_to_client_error() {
    let mut c = Client::new(Stub {
        status: 500,
        body: "boom".to_string(),
    });
    assert_eq!(c.workspace(), Err(ClientError::Http(500, "boom".to_string())));
}

#[test]
fn bad_json_maps_to_decode_error() {
    let mut c = Client::new(Stub {
        status: 200,
        body: "{not json".to_string(),
    });
    assert!(matches!(c.workspace(), Err(ClientError::Decode(_))));
}

#[test]
fn replace_workspace_roundtrip() {
    let mut c = Client::new(server());
    let mut replacement = Workspace::new("replaced");
    replacement
        .add_page(Page::new("only", "Only page", "O"))
        .unwrap();
    let returned = c.replace_workspace(&replacement).unwrap();
    assert_eq!(returned.name, "replaced");
    assert_eq!(returned.pages.len(), 1);
    assert_eq!(c.workspace().unwrap().pages[0].title, "Only page");
}
