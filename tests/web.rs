//! Axum HTTP adapter and contract-suite tests (spec-038).

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, Request, StatusCode};
use ion_of_notes::model::{PageSummary, Workspace};
use ion_of_notes::server::web::router;
use ion_of_notes::server::{
    API_VERSION, ApiError, Envelope, ListResponse, OWNER_TOKEN, WorkspaceServer,
};
use ion_of_notes::webhooks::WebhookSubscription;
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
    let created: Envelope<WebhookSubscription> = serde_json::from_str(&body).unwrap();
    let id = created.data.unwrap().id;

    let (status, _, body) = send(&app, v2_request("GET", "/api/v2/webhooks", None)).await;
    assert_eq!(status, StatusCode::OK);
    let list: Envelope<Vec<WebhookSubscription>> = serde_json::from_str(&body).unwrap();
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
