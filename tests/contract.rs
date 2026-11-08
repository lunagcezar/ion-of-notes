//! Contract tests for the v2 foundation (spec-030).

mod common;

use common::page_draft_json;
use not_notion::model::Workspace;
use not_notion::server::{
    API_VERSION, ApiError, ApiRequest, ApiResponse, Capability, Connection, Envelope, Method,
    OWNER_TOKEN, WorkspaceApi, WorkspaceServer, format_http_response, parse_http_request, serve_on,
};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

fn v2(method: Method, path: &str, body: Option<String>) -> ApiRequest {
    v2_as(OWNER_TOKEN, method, path, body)
}

fn v2_as(token: &str, method: Method, path: &str, body: Option<String>) -> ApiRequest {
    ApiRequest::new(method, path, body)
        .with_header("not-notion-version", API_VERSION)
        .with_header("authorization", format!("Bearer {token}"))
}

fn error(response: &ApiResponse) -> ApiError {
    serde_json::from_str(&response.body).expect("v2 error body")
}

#[test]
fn v2_requires_version_header() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let before = server.workspace.clone();
    let request = ApiRequest::new(Method::Get, "/api/v2/workspace", None)
        .with_header("authorization", format!("Bearer {OWNER_TOKEN}"));
    let response = server.handle(&request);

    assert_eq!(response.status, 400);
    let error = error(&response);
    assert_eq!(error.object, "error");
    assert_eq!(error.status, 400);
    assert_eq!(error.code, "missing_version");
    assert!(!error.request_id.is_empty());
    assert_eq!(server.workspace, before);
}

#[test]
fn v2_rejects_unsupported_version() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let request = ApiRequest::new(Method::Get, "/api/v2/workspace", None)
        .with_header("not-notion-version", "1900-01-01")
        .with_header("authorization", format!("Bearer {OWNER_TOKEN}"));
    let response = server.handle(&request);

    assert_eq!(response.status, 400);
    let error = error(&response);
    assert_eq!(error.code, "unsupported_version");
    let supported = error
        .additional_data
        .as_ref()
        .and_then(|data| data.get("supported_versions"))
        .and_then(|value| value.as_array())
        .expect("supported_versions");
    assert!(supported.iter().any(|value| value == API_VERSION));
}

#[test]
fn v2_reports_invalid_json() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let response = server.handle(&v2(Method::Post, "/api/v2/pages", Some("{".to_string())));

    assert_eq!(response.status, 400);
    assert_eq!(error(&response).code, "invalid_json");
}

#[test]
fn v2_reports_validation_error() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let response = server.handle(&v2(
        Method::Post,
        "/api/v2/pages",
        Some(page_draft_json("", None)),
    ));

    assert_eq!(response.status, 400);
    assert_eq!(error(&response).code, "validation_error");
}

#[test]
fn v2_requires_a_bearer_token() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let request = ApiRequest::new(Method::Get, "/api/v2/workspace", None)
        .with_header("not-notion-version", API_VERSION);
    let response = server.handle(&request);

    assert_eq!(response.status, 401);
    assert_eq!(error(&response).code, "unauthorized");
}

#[test]
fn v2_rejects_an_unknown_token() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let response = server.handle(&v2_as(
        "not-a-token",
        Method::Get,
        "/api/v2/workspace",
        None,
    ));

    assert_eq!(response.status, 401);
    assert_eq!(error(&response).code, "unauthorized");
}

#[test]
fn v2_enforces_capabilities_separately() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    server.register_connection(Connection::new(
        "reader",
        "reader-token",
        "m-owner",
        "local-workspace",
        [Capability::ReadContent],
    ));

    let read = server.handle(&v2_as(
        "reader-token",
        Method::Get,
        "/api/v2/workspace",
        None,
    ));
    assert_eq!(read.status, 200);

    let write = server.handle(&v2_as(
        "reader-token",
        Method::Post,
        "/api/v2/pages",
        Some(page_draft_json("Nope", None)),
    ));
    assert_eq!(write.status, 403);
    let error = error(&write);
    assert_eq!(error.code, "restricted_resource");
    assert_eq!(
        error
            .additional_data
            .as_ref()
            .and_then(|data| data.get("required_capability"))
            .and_then(|value| value.as_str()),
        Some("insert_content")
    );
}

#[test]
fn capabilities_are_independent() {
    let read_only = Connection::new(
        "reader",
        "reader-token",
        "m-owner",
        "local-workspace",
        [Capability::ReadContent],
    );
    assert!(read_only.has(Capability::ReadContent));
    assert!(!read_only.has(Capability::UpdateContent));
    assert!(!read_only.has(Capability::InsertContent));
    assert!(!read_only.has(Capability::ReadComments));
    assert!(!read_only.has(Capability::UserInfoWithEmail));

    let owner = Connection::owner("m-owner", "local-workspace");
    assert!(Capability::all().into_iter().all(|c| owner.has(c)));
}

#[test]
fn v2_request_id_is_logged() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let response = server.handle(&v2(Method::Get, "/api/v2/workspace", None));

    assert_eq!(response.status, 200);
    let request_id = response.request_id.expect("v2 request id");
    let entry = server.request_log().last().expect("request log entry");
    assert_eq!(entry.request_id, request_id);
    assert_eq!(entry.status, 200);
    assert_eq!(entry.method, "GET");
}

#[test]
fn v2_error_request_id_is_logged() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let request = ApiRequest::new(Method::Get, "/api/v2/workspace", None)
        .with_header("authorization", format!("Bearer {OWNER_TOKEN}"));
    let response = server.handle(&request);

    assert_eq!(response.status, 400);
    let request_id = response.request_id.clone().expect("v2 request id");
    assert_eq!(error(&response).request_id, request_id);
    let entry = server.request_log().last().expect("request log entry");
    assert_eq!(entry.request_id, request_id);
    assert_eq!(entry.status, 400);
}

#[test]
fn v2_unknown_route_is_structured_404() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let response = server.handle(&v2(Method::Get, "/api/v2/does-not-exist", None));

    assert_eq!(response.status, 404);
    assert_eq!(error(&response).code, "object_not_found");
}

#[test]
fn v1_is_unchanged() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let response = server.handle(&ApiRequest::new(Method::Get, "/api/workspace", None));

    assert_eq!(response.status, 200);
    assert_eq!(response.request_id, None);
    let envelope: Envelope<Workspace> = serde_json::from_str(&response.body).unwrap();
    assert!(envelope.data.is_some());
}

#[test]
fn v2_create_and_read_roundtrip() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let created = server.handle(&v2(
        Method::Post,
        "/api/v2/pages",
        Some(page_draft_json("V2 page", None)),
    ));
    assert_eq!(created.status, 201);
    let page: not_notion::model::Page = serde_json::from_str::<Envelope<_>>(&created.body)
        .unwrap()
        .data
        .unwrap();

    let read = server.handle(&v2(
        Method::Get,
        &format!("/api/v2/pages/{}", page.id),
        None,
    ));
    assert_eq!(read.status, 200);
    let read_page: not_notion::model::Page = serde_json::from_str::<Envelope<_>>(&read.body)
        .unwrap()
        .data
        .unwrap();
    assert_eq!(read_page.title, "V2 page");
}

#[test]
fn http_response_echoes_the_request_id() {
    let response = ApiResponse {
        status: 200,
        body: "{}".to_string(),
        request_id: Some("req_test".to_string()),
    };
    let text = String::from_utf8(format_http_response(&response)).unwrap();
    assert!(text.contains("X-Request-Id: req_test\r\n"));
}

#[test]
fn http_parser_collects_version_and_auth_headers() {
    let raw = format!(
        "GET /api/v2/workspace HTTP/1.1\r\nHost: localhost\r\nNot-Notion-Version: {API_VERSION}\r\nAuthorization: Bearer {OWNER_TOKEN}\r\nConnection: close\r\n\r\n"
    );
    let request = parse_http_request(raw.as_bytes()).unwrap();
    assert_eq!(request.header("not-notion-version"), Some(API_VERSION));
    let expected = format!("Bearer {OWNER_TOKEN}");
    assert_eq!(request.header("authorization"), Some(expected.as_str()));
}

#[test]
fn v2_http_roundtrip_has_request_id_header() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let server = WorkspaceServer::new(Workspace::seed("x"));
    std::thread::spawn(move || {
        let _ = serve_on(listener, server);
    });

    let mut stream = TcpStream::connect(addr).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let request = format!(
        "GET /api/v2/workspace HTTP/1.1\r\nHost: localhost\r\nNot-Notion-Version: {API_VERSION}\r\nAuthorization: Bearer {OWNER_TOKEN}\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(request.as_bytes()).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();

    assert!(response.starts_with("HTTP/1.1 200"));
    assert!(response.contains("X-Request-Id: req_"));
}

#[test]
fn v2_persistence_failure_is_structured() {
    struct FailingStore;
    impl not_notion::store::WorkspaceStore for FailingStore {
        fn persist(&self, _workspace: &Workspace) -> std::io::Result<()> {
            Err(std::io::Error::other("disk full"))
        }
    }

    let mut server = WorkspaceServer::with_store(Workspace::seed("x"), FailingStore);
    let response = server.handle(&v2(
        Method::Post,
        "/api/v2/pages",
        Some(page_draft_json("Will fail", None)),
    ));

    assert_eq!(response.status, 500);
    assert_eq!(error(&response).code, "internal_server_error");
}
