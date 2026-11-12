//! Shared SQLite rate-limit tests (spec-041).

mod common;

use ion_of_notes::model::Workspace;
use ion_of_notes::server::{ApiError, ApiResponse, Method, RequestTransport, WorkspaceServer};
use ion_of_notes::store::{RateLimitStore, SqliteStore};
use std::sync::Arc;
use std::time::Duration;

fn error(response: &ApiResponse) -> ApiError {
    serde_json::from_str(&response.body).expect("structured error")
}

#[test]
fn sqlite_rate_limits_are_shared_across_servers() {
    let db = common::temp_path("shared-limits", "sqlite3");
    let workspace = Workspace::seed("shared-limits");
    let store1 = SqliteStore::open(&db).expect("store 1");
    let store2 = SqliteStore::open(&db).expect("store 2");

    let mut server1 = WorkspaceServer::new(workspace.clone());
    let mut server2 = WorkspaceServer::new(workspace);
    server1.configure_shared_rate_limits(Arc::new(store1), 100, 2, Duration::from_secs(60));
    server2.configure_shared_rate_limits(Arc::new(store2), 100, 2, Duration::from_secs(60));

    assert_eq!(
        server1
            .handle(&common::v2_request(Method::Get, "/api/v2/workspace", None))
            .status,
        200
    );
    assert_eq!(
        server2
            .handle(&common::v2_request(Method::Get, "/api/v2/workspace", None))
            .status,
        200
    );

    let response = server1.handle(&common::v2_request(Method::Get, "/api/v2/workspace", None));
    assert_eq!(response.status, 429);
    let error = error(&response);
    assert_eq!(error.code, "rate_limited");
    assert_eq!(
        error.additional_data.as_ref().unwrap()["rate_limit_reason"],
        "public_api_space_request_rate_limit"
    );

    common::remove(&db);
}

#[test]
fn sqlite_rate_limit_window_resets() {
    let db = common::temp_path("shared-window", "sqlite3");
    let store = SqliteStore::open(&db).expect("store");

    assert!(store.check("window", 60, 1, 100).expect("first").allowed);
    let denied = store.check("window", 60, 1, 101).expect("denied");
    assert!(!denied.allowed);
    assert_eq!(denied.retry_after, 59);
    assert!(store.check("window", 60, 1, 160).expect("reset").allowed);

    common::remove(&db);
}
