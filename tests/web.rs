//! Axum HTTP adapter and contract-suite tests (spec-038).

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, Request, StatusCode};
use ion_of_notes::auth::{Account, AuthStore, MemoryAuthStore, SessionResponse, hash_password};
use ion_of_notes::collab::{Access, CollabState, Member, Role, Share};
use ion_of_notes::model::{PageSummary, Workspace, now_iso};
use ion_of_notes::server::web::{router, serve_axum};
use ion_of_notes::server::{
    API_VERSION, ApiError, Envelope, ListResponse, OWNER_TOKEN, WorkspaceServer,
};
use ion_of_notes::store::{NoCollabStore, NoStore};
use ion_of_notes::webhooks::{WebhookCreatedDto, WebhookSubscriptionDto};
use std::time::Duration;
use tower::ServiceExt;

fn server() -> WorkspaceServer {
    WorkspaceServer::new(Workspace::seed("web-tests"))
}

fn request(method: &str, path: &str, body: Option<String>) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .body(body.map(Body::from).unwrap_or_else(Body::empty))
        .unwrap()
}

fn v2_request(method: &str, path: &str, body: Option<String>) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .header("ion-of-notes-version", API_VERSION)
        .header("authorization", format!("Bearer {OWNER_TOKEN}"))
        .body(body.map(Body::from).unwrap_or_else(Body::empty))
        .unwrap()
}

async fn send(app: &Router, request: Request<Body>) -> (StatusCode, HeaderMap, String) {
    let response = app.clone().oneshot(request).await.expect("router response");
    let status = response.status();
    let headers = response.headers().clone();
    let body = to_bytes(response.into_body(), 4 * 1024 * 1024)
        .await
        .expect("response body");
    (
        status,
        headers,
        String::from_utf8(body.to_vec()).expect("utf8 body"),
    )
}

fn error(body: &str) -> ApiError {
    serde_json::from_str(body).expect("structured v2 error")
}

fn header<'a>(headers: &'a HeaderMap, name: &str) -> &'a str {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
}

#[tokio::test]
async fn health_route_works_through_axum() {
    let app = router(server());
    let (status, headers, body) = send(&app, request("GET", "/health", None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        header(&headers, "content-type"),
        "application/json; charset=utf-8"
    );
    let envelope: Envelope<String> = serde_json::from_str(&body).unwrap();
    assert_eq!(envelope.data.unwrap(), "ok");
}

#[test]
fn unauthenticated_server_refuses_a_public_bind() {
    let error = serve_axum("0.0.0.0:0", server()).expect_err("public bind must be refused");
    assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
}

#[tokio::test]
async fn ready_route_reports_ready() {
    let app = router(server());
    let (status, headers, body) = send(&app, request("GET", "/ready", None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        header(&headers, "content-type"),
        "application/json; charset=utf-8"
    );
    let envelope: Envelope<String> = serde_json::from_str(&body).unwrap();
    assert_eq!(envelope.data.unwrap(), "ready");
}

#[tokio::test]
async fn metrics_route_exposes_prometheus_text() {
    let app = router(server());
    let _ = send(&app, request("GET", "/api/pages", None)).await;
    let (status, headers, body) = send(&app, request("GET", "/metrics", None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        header(&headers, "content-type"),
        "text/plain; version=0.0.4"
    );
    assert!(body.contains("ion_of_notes_up 1"));
    assert!(body.contains("ion_of_notes_uptime_seconds"));
    assert!(body.contains("ion_of_notes_http_requests_total 1"));
    assert!(body.contains("ion_of_notes_webhook_pending"));
    assert!(body.contains("ion_of_notes_webhook_dead_letters"));
    assert!(body.contains("ion_of_notes_webhook_deliveries_total"));
    assert!(body.contains("ion_of_notes_pages"));
    assert!(body.contains("ion_of_notes_blocks"));
    assert!(body.contains("ion_of_notes_members"));
    assert!(body.contains("ion_of_notes_workspace_version"));
    assert!(body.contains("ion_of_notes_sync_seq"));
    assert!(body.contains("ion_of_notes_request_log_entries"));
}

#[tokio::test]
async fn v1_pages_route_works_through_axum() {
    let app = router(server());
    let (status, _, body) = send(&app, request("GET", "/api/pages", None)).await;
    assert_eq!(status, StatusCode::OK);
    let envelope: Envelope<Vec<PageSummary>> = serde_json::from_str(&body).unwrap();
    assert!(!envelope.data.unwrap().is_empty());
}

#[tokio::test]
async fn snapshot_reads_stay_correct_after_a_write() {
    let app = router(server());
    let (status, _, body) = send(&app, request("GET", "/api/workspace", None)).await;
    assert_eq!(status, StatusCode::OK);
    let envelope: Envelope<Workspace> = serde_json::from_str(&body).unwrap();
    let mut workspace = envelope.data.unwrap();
    let page_id = workspace.pages[0].id.clone();
    workspace.pages[0].title = "Snapshot title".to_string();
    let page_json = serde_json::to_string(&workspace.pages[0]).unwrap();

    let (status, _, _) = send(
        &app,
        request("PATCH", &format!("/api/pages/{page_id}"), Some(page_json)),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _, body) = send(&app, request("GET", "/api/workspace", None)).await;
    assert_eq!(status, StatusCode::OK);
    let envelope: Envelope<Workspace> = serde_json::from_str(&body).unwrap();
    assert_eq!(
        envelope.data.unwrap().pages[0].title,
        "Snapshot title",
        "the read snapshot must be invalidated after a write"
    );

    let (status, _, body) =
        send(&app, request("GET", &format!("/api/pages/{page_id}"), None)).await;
    assert_eq!(status, StatusCode::OK);
    let envelope: Envelope<ion_of_notes::model::Page> = serde_json::from_str(&body).unwrap();
    assert_eq!(envelope.data.unwrap().title, "Snapshot title");

    let (status, _, _) = send(&app, request("GET", "/api/pages/missing", None)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn snapshot_reads_require_auth_when_configured() {
    let server = WorkspaceServer::with_stores_and_auth(
        Workspace::seed("web-auth"),
        CollabState::new(),
        NoStore,
        NoCollabStore,
        MemoryAuthStore::default(),
    );
    let app = router(server);
    let (status, _, body) = send(&app, request("GET", "/api/workspace", None)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let envelope: Envelope<()> = serde_json::from_str(&body).unwrap();
    assert!(envelope.data.is_none());
}

#[tokio::test]
async fn snapshot_reads_are_projections_for_a_member() {
    let workspace = Workspace::seed("web-projection");
    let mut collab = CollabState::new();
    collab.shares.clear();
    collab.members.push(Member::new(
        "m-member",
        "Morgan Member",
        "member@workspace.local",
        Role::Member,
    ));
    collab.shares.push(Share {
        page_id: "p-home".to_string(),
        subject: "m-member".to_string(),
        access: Access::Edit,
    });
    let store = MemoryAuthStore::default();
    store
        .upsert_account(&Account {
            id: "account-m-member".to_string(),
            member_id: "m-member".to_string(),
            email: "member@workspace.local".to_string(),
            password_hash: hash_password("secret").expect("hash"),
            created_at: now_iso(),
        })
        .expect("account");
    let server =
        WorkspaceServer::with_stores_and_auth(workspace, collab, NoStore, NoCollabStore, store);
    let app = router(server);

    let login = Request::builder()
        .method("POST")
        .uri("/api/v2/auth/login")
        .header("ion-of-notes-version", API_VERSION)
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"email":"member@workspace.local","password":"secret"}"#,
        ))
        .unwrap();
    let (status, _, body) = send(&app, login).await;
    assert_eq!(status, StatusCode::OK);
    let session: Envelope<SessionResponse> = serde_json::from_str(&body).unwrap();
    let token = session.data.expect("session").token;

    let authorized = |path: &str| {
        Request::builder()
            .method("GET")
            .uri(path)
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap()
    };
    let (status, _, body) = send(&app, authorized("/api/workspace")).await;
    assert_eq!(status, StatusCode::OK);
    let envelope: Envelope<Workspace> = serde_json::from_str(&body).unwrap();
    let projected = envelope.data.expect("workspace");
    assert_eq!(projected.pages.len(), 1);
    assert_eq!(projected.pages[0].id, "p-home");

    let (status, _, body) = send(&app, authorized("/api/pages")).await;
    assert_eq!(status, StatusCode::OK);
    let envelope: Envelope<Vec<PageSummary>> = serde_json::from_str(&body).unwrap();
    let pages = envelope.data.expect("pages");
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].id, "p-home");
}

#[tokio::test]
async fn v2_requires_version_then_token() {
    let app = router(server());

    let (status, headers, body) = send(&app, request("GET", "/api/v2/workspace", None)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error(&body).code, "missing_version");
    assert_eq!(error(&body).request_id, header(&headers, "x-request-id"));

    let version_only = Request::builder()
        .method("GET")
        .uri("/api/v2/workspace")
        .header("ion-of-notes-version", API_VERSION)
        .body(Body::empty())
        .unwrap();
    let (status, headers, body) = send(&app, version_only).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(error(&body).code, "unauthorized");
    assert_eq!(error(&body).request_id, header(&headers, "x-request-id"));
}

#[tokio::test]
async fn v2_workspace_roundtrip_has_request_id() {
    let app = router(server());
    let (status, headers, body) = send(&app, v2_request("GET", "/api/v2/workspace", None)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(header(&headers, "x-request-id").starts_with("req_"));
    let envelope: Envelope<Workspace> = serde_json::from_str(&body).unwrap();
    assert_eq!(envelope.data.unwrap().name, "web-tests");
}

#[tokio::test]
async fn rate_limit_sets_retry_after() {
    let mut server = server();
    server.configure_rate_limits(1, 100, Duration::from_secs(60));
    let app = router(server);

    let (status, _, _) = send(&app, v2_request("GET", "/api/v2/workspace", None)).await;
    assert_eq!(status, StatusCode::OK);

    let (status, headers, body) = send(&app, v2_request("GET", "/api/v2/workspace", None)).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(header(&headers, "retry-after"), "60");
    assert_eq!(error(&body).code, "rate_limited");
}

#[tokio::test]
async fn oversized_body_is_a_validation_error() {
    let app = router(server());
    let body = format!(
        r#"{{"title":"{}","icon":"P","parent":null}}"#,
        "x".repeat(501 * 1024)
    );
    let (status, _, body) = send(&app, v2_request("POST", "/api/v2/pages", Some(body))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error(&body).code, "validation_error");
}

#[tokio::test]
async fn query_string_and_pagination_are_preserved() {
    let app = router(server());
    let (status, _, body) = send(&app, v2_request("GET", "/api/v2/pages?page_size=2", None)).await;
    assert_eq!(status, StatusCode::OK);
    let envelope: Envelope<ListResponse<PageSummary>> = serde_json::from_str(&body).unwrap();
    assert_eq!(envelope.data.unwrap().results.len(), 2);
}

#[tokio::test]
async fn webhook_roundtrip_works_through_axum() {
    let app = router(server());
    let create = v2_request(
        "POST",
        "/api/v2/webhooks",
        Some(r#"{"url":"https://example.com/hook","events":["page.created"]}"#.to_string()),
    );
    let (status, _, body) = send(&app, create).await;
    assert_eq!(status, StatusCode::CREATED);
    let created: Envelope<WebhookCreatedDto> = serde_json::from_str(&body).unwrap();
    let id = created.data.unwrap().id;

    let (status, _, body) = send(&app, v2_request("GET", "/api/v2/webhooks", None)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        !body.contains("verification_token"),
        "the list response must not expose the verification secret"
    );
    let list: Envelope<Vec<WebhookSubscriptionDto>> = serde_json::from_str(&body).unwrap();
    assert_eq!(list.data.unwrap().len(), 1);

    let (status, _, _) = send(
        &app,
        v2_request("DELETE", &format!("/api/v2/webhooks/{id}"), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn unknown_route_returns_a_structured_404() {
    let app = router(server());
    let (status, _, body) = send(&app, v2_request("GET", "/api/v2/nope", None)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(error(&body).code, "object_not_found");
}

#[tokio::test]
async fn v2_snapshot_reads_are_invalidated_by_writes() {
    let app = router(server());
    let (status, _, body) = send(&app, v2_request("GET", "/api/v2/workspace", None)).await;
    assert_eq!(status, StatusCode::OK);
    let before: Envelope<Workspace> = serde_json::from_str(&body).unwrap();
    let before = before.data.expect("workspace").pages.len();

    let draft = serde_json::json!({
        "title": "Snapshot page",
        "icon": "",
        "parent": null,
    })
    .to_string();
    let (status, _, _) = send(&app, v2_request("POST", "/api/v2/pages", Some(draft))).await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, _, body) = send(&app, v2_request("GET", "/api/v2/workspace", None)).await;
    assert_eq!(status, StatusCode::OK);
    let after: Envelope<Workspace> = serde_json::from_str(&body).unwrap();
    assert_eq!(after.data.expect("workspace").pages.len(), before + 1);

    let (status, _, body) = send(&app, v2_request("GET", "/api/v2/search?q=Snapshot", None)).await;
    assert_eq!(status, StatusCode::OK);
    let results: Envelope<ListResponse<serde_json::Value>> = serde_json::from_str(&body).unwrap();
    let results = results.data.expect("search list");
    assert!(
        results
            .results
            .iter()
            .any(|item| item["page_title"] == "Snapshot page"),
        "{body}"
    );
}
