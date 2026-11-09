//! Tests for the client layer (public API only).

use not_notion::client::{Client, ClientError, RetryPolicy, encode_body};
use not_notion::model::{Page, PageDraft, Workspace};
use not_notion::server::{ApiRequest, ApiResponse, Method, WorkspaceApi, WorkspaceServer};
use serde::Serialize;

struct Stub {
    status: u16,
    body: String,
}

impl WorkspaceApi for Stub {
    fn handle(&mut self, _req: &ApiRequest) -> ApiResponse {
        ApiResponse {
            status: self.status,
            body: self.body.clone(),
            request_id: None,
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
fn v2_client_requires_a_token() {
    let mut c = Client::new(server());
    assert_eq!(c.workspace_v2().unwrap_err(), ClientError::MissingToken);
}

#[test]
fn v2_client_roundtrip() {
    let mut c = Client::with_token(server(), "owner-token");
    let before = c.workspace_v2().unwrap().pages.len();
    let page = c
        .create_page_v2(&PageDraft {
            title: "V2".to_string(),
            icon: "V".to_string(),
            parent: None,
        })
        .unwrap();
    assert_eq!(c.get_page_v2(&page.id).unwrap().title, "V2");
    assert_eq!(c.workspace_v2().unwrap().pages.len(), before + 1);
}

#[test]
fn v2_client_reports_structured_errors() {
    let mut c = Client::with_token(server(), "owner-token");
    let error = c
        .create_page_v2(&PageDraft {
            title: String::new(),
            icon: "P".to_string(),
            parent: None,
        })
        .unwrap_err();
    match error {
        ClientError::Api(error) => {
            assert_eq!(error.status, 400);
            assert_eq!(error.code, "validation_error");
            assert!(!error.request_id.is_empty());
        }
        other => panic!("expected structured api error, got {other:?}"),
    }
}

#[test]
fn v2_client_reports_unauthorized() {
    let mut c = Client::with_token(server(), "wrong-token");
    match c.workspace_v2().unwrap_err() {
        ClientError::Api(error) => {
            assert_eq!(error.status, 401);
            assert_eq!(error.code, "unauthorized");
        }
        other => panic!("expected structured api error, got {other:?}"),
    }
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
    assert_eq!(
        c.workspace(),
        Err(ClientError::Http(500, "boom".to_string()))
    );
}

#[test]
fn bad_json_maps_to_decode_error() {
    let mut c = Client::new(Stub {
        status: 200,
        body: "{not json".to_string(),
    });
    assert!(matches!(c.workspace(), Err(ClientError::Decode(_))));
}

struct FailingSerialize;

impl Serialize for FailingSerialize {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Err(<S::Error as serde::ser::Error>::custom("encode failed"))
    }
}

#[test]
fn encode_failure_maps_to_encode_error() {
    let error = encode_body(&FailingSerialize).unwrap_err();
    assert!(matches!(error, ClientError::Encode(_)));
    assert!(error.to_string().contains("encode failed"));
}

#[test]
fn page_versions_are_newest_first_after_updates() {
    let mut c = Client::new(server());
    let mut page = c.get_page("p-home").unwrap();
    page.title = "First revision".to_string();
    c.update_page(&page).unwrap();
    page.title = "Second revision".to_string();
    c.update_page(&page).unwrap();

    let versions = c.page_versions("p-home").unwrap();
    assert!(versions.len() >= 2);
    assert!(
        versions
            .windows(2)
            .all(|pair| pair[0].version > pair[1].version)
    );
    assert!(versions.iter().any(|v| v.title == "Second revision"));
}

#[test]
fn retry_policy_respects_retry_after_and_caps_attempts() {
    let policy = RetryPolicy::default();

    let limited = policy.decide(Method::Get, 429, Some("rate_limited"), Some(7), 0, 0);
    assert!(limited.retry);
    assert_eq!(limited.delay_millis, 7_000);

    let blocked = policy.decide(
        Method::Get,
        429,
        Some("public_api_request_blocked"),
        Some(7),
        0,
        0,
    );
    assert!(!blocked.retry);

    let overload = policy.decide(Method::Post, 529, Some("service_overload"), None, 0, 0);
    assert!(overload.retry);
    assert_eq!(overload.delay_millis, 1_000);

    let write_timeout = policy.decide(Method::Post, 503, Some("service_unavailable"), None, 0, 0);
    assert!(!write_timeout.retry);

    let read_timeout = policy.decide(Method::Get, 503, None, None, 1, 250);
    assert!(read_timeout.retry);
    assert_eq!(read_timeout.delay_millis, 2_250);

    let exhausted = policy.decide(Method::Get, 429, Some("rate_limited"), Some(1), 5, 0);
    assert!(!exhausted.retry);
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
