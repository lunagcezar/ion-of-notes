//! Real-listener smoke test for the shipped Axum adapter (spec-067).

use ion_of_notes::model::Workspace;
use ion_of_notes::server::web::router;
use ion_of_notes::server::{
    API_VERSION, AUTH_HEADER, OWNER_TOKEN, VERSION_HEADER, WorkspaceServer,
};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

async fn start() -> (String, JoinHandle<()>) {
    let app = router(WorkspaceServer::new(Workspace::seed("smoke")));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{address}"), handle)
}

fn get(url: &str) -> (u16, ureq::http::HeaderMap, String) {
    let mut response = ureq::get(url).call().expect("HTTP request");
    let status = response.status().as_u16();
    let headers = response.headers().clone();
    let body = response.body_mut().read_to_string().expect("body");
    (status, headers, body)
}

#[tokio::test(flavor = "multi_thread")]
async fn real_listener_serves_operational_and_v2_endpoints() {
    let (base, handle) = start().await;

    let (status, _, body) = get(&format!("{base}/health"));
    assert_eq!(status, 200);
    assert!(body.contains("ok"));

    let (status, _, body) = get(&format!("{base}/ready"));
    assert_eq!(status, 200);
    assert!(body.contains("ready"));

    let (status, _, body) = get(&format!("{base}/metrics"));
    assert_eq!(status, 200);
    assert!(body.contains("ion_of_notes_up 1"));

    let mut response = ureq::get(&format!("{base}/api/v2/workspace"))
        .header(VERSION_HEADER, API_VERSION)
        .header(AUTH_HEADER, format!("Bearer {OWNER_TOKEN}"))
        .call()
        .expect("v2 request");
    let status = response.status().as_u16();
    let request_id = response
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let body = response.body_mut().read_to_string().expect("v2 body");
    assert_eq!(status, 200);
    assert!(request_id.starts_with("req_"));
    let envelope: serde_json::Value = serde_json::from_str(&body).expect("v2 json");
    assert_eq!(envelope["data"]["name"], "smoke");

    // Give the runtime a moment to flush the response, then stop the listener.
    tokio::time::sleep(Duration::from_millis(10)).await;
    handle.abort();
    let _ = handle.await;
}
