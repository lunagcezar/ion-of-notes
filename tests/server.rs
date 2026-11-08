//! Tests for the server contract, routes, and HTTP adapter (public API only).

mod common;

use common::{data, page_draft_json, remove, temp_path};
use not_notion::model::{Page, SearchResult, Workspace};
use not_notion::server::{
    ApiRequest, ApiResponse, Envelope, Method, WorkspaceApi, WorkspaceServer, format_http_response,
    parse_http_request, serve_on,
};
use not_notion::store;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

#[test]
fn health_route() {
    let mut s = WorkspaceServer::new(Workspace::seed("x"));
    let r = s.handle(&ApiRequest::new(Method::Get, "/health", None));
    assert_eq!(r.status, 200);
    assert!(r.body.contains("ok"));
}

#[test]
fn workspace_snapshot_route() {
    let ws = Workspace::seed("x");
    let mut s = WorkspaceServer::new(ws.clone());
    let r = s.handle(&ApiRequest::new(Method::Get, "/api/workspace", None));
    assert_eq!(r.status, 200);
    let env: Envelope<Workspace> = serde_json::from_str(&r.body).unwrap();
    assert_eq!(env.data.unwrap(), ws);
}

#[test]
fn create_page_returns_201_and_bumps_version() {
    let mut s = WorkspaceServer::new(Workspace::seed("x"));
    let before = s.workspace.pages.len();
    let version = s.workspace.version;
    let r = s.handle(&ApiRequest::new(
        Method::Post,
        "/api/pages",
        Some(page_draft_json("New", None)),
    ));
    assert_eq!(r.status, 201);
    let page: Page = data(&r);
    assert!(!page.id.is_empty());
    assert_eq!(page.title, "New");
    assert_eq!(s.workspace.pages.len(), before + 1);
    assert_eq!(s.workspace.version, version + 1);
}

#[test]
fn create_page_rejects_unknown_parent() {
    let mut s = WorkspaceServer::new(Workspace::seed("x"));
    let before = s.workspace.pages.len();
    let r = s.handle(&ApiRequest::new(
        Method::Post,
        "/api/pages",
        Some(page_draft_json("Orphan", Some("does-not-exist"))),
    ));
    assert_eq!(r.status, 400);
    assert!(r.body.contains("error"));
    assert_eq!(s.workspace.pages.len(), before);
}

#[test]
fn get_unknown_page_returns_404() {
    let mut s = WorkspaceServer::new(Workspace::seed("x"));
    let r = s.handle(&ApiRequest::new(Method::Get, "/api/pages/nope", None));
    assert_eq!(r.status, 404);
}

#[test]
fn patch_persists_to_disk() {
    let path = temp_path("patch", "json");
    remove(&path);
    let mut s = WorkspaceServer::with_path(Workspace::seed("x"), path.clone());
    let mut page = s.workspace.pages[0].clone();
    page.title = "Changed title".to_string();
    let r = s.handle(&ApiRequest::new(
        Method::Patch,
        format!("/api/pages/{}", page.id),
        Some(serde_json::to_string(&page).unwrap()),
    ));
    assert_eq!(r.status, 200);
    let on_disk = store::load(&path).unwrap();
    assert_eq!(on_disk.pages[0].title, "Changed title");
    remove(&path);
}

#[test]
fn delete_removes_subtree_and_persists() {
    let path = temp_path("delete", "json");
    remove(&path);
    let mut s = WorkspaceServer::with_path(Workspace::new("x"), path.clone());
    let parent: Page = data(&s.handle(&ApiRequest::new(
        Method::Post,
        "/api/pages",
        Some(page_draft_json("Parent", None)),
    )));
    let child = s.handle(&ApiRequest::new(
        Method::Post,
        "/api/pages",
        Some(page_draft_json("Child", Some(&parent.id))),
    ));
    assert_eq!(child.status, 201);
    let r = s.handle(&ApiRequest::new(
        Method::Delete,
        format!("/api/pages/{}", parent.id),
        None,
    ));
    assert_eq!(r.status, 200);
    assert!(s.workspace.pages.is_empty());
    assert!(store::load(&path).unwrap().pages.is_empty());
    remove(&path);
}

#[test]
fn search_route_returns_ranked_results() {
    let mut ws = Workspace::seed("x");
    ws.add_page(Page::new("road", "Roadmap", "R")).unwrap();
    let mut s = WorkspaceServer::new(ws);
    let r = s.handle(&ApiRequest::new(Method::Get, "/api/search?q=road", None));
    assert_eq!(r.status, 200);
    let results: Vec<SearchResult> = data(&r);
    assert_eq!(results[0].page_id, "road");
}

#[test]
fn unknown_route_and_bad_json_do_not_panic() {
    let mut s = WorkspaceServer::new(Workspace::seed("x"));
    assert_eq!(
        s.handle(&ApiRequest::new(Method::Get, "/api/nope", None))
            .status,
        404
    );
    assert_eq!(
        s.handle(&ApiRequest::new(
            Method::Post,
            "/api/pages",
            Some("{not json".to_string())
        ))
        .status,
        400
    );
    assert_eq!(
        s.handle(&ApiRequest::new(Method::Get, "/health", None))
            .status,
        200
    );
}

#[test]
fn parse_http_request_extracts_method_path_body() {
    let body = "{\"title\":\"Hi\"}";
    let raw = format!(
        "POST /api/pages HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let req = parse_http_request(raw.as_bytes()).unwrap();
    assert_eq!(req.method, Method::Post);
    assert_eq!(req.path, "/api/pages");
    assert_eq!(req.body.as_deref(), Some(body));
    assert!(parse_http_request(b"garbage").is_none());
}

#[test]
fn serve_on_handles_two_concurrent_connections() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let server = WorkspaceServer::new(Workspace::seed("x"));
    std::thread::spawn(move || {
        let _ = serve_on(listener, server);
    });

    let mut first = TcpStream::connect(addr).unwrap();
    let mut second = TcpStream::connect(addr).unwrap();
    first
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    second
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();

    let request = b"GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    first.write_all(request).unwrap();
    second.write_all(request).unwrap();

    let mut first_response = String::new();
    let mut second_response = String::new();
    first.read_to_string(&mut first_response).unwrap();
    second.read_to_string(&mut second_response).unwrap();

    assert!(first_response.starts_with("HTTP/1.1 200"));
    assert!(second_response.starts_with("HTTP/1.1 200"));
}

#[test]
fn format_http_response_has_valid_headers() {
    let bytes = format_http_response(&ApiResponse {
        status: 200,
        body: "{}".to_string(),
        request_id: None,
    });
    let text = String::from_utf8(bytes).unwrap();
    assert!(text.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(text.contains("Content-Length: 2\r\n"));
    assert!(text.contains("Content-Type: application/json"));
    assert!(text.ends_with("{}"));
}
