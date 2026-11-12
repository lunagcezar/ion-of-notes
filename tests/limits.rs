//! Limits, rate limiting, block limits and operational errors (spec-037).

mod common;

use common::{data, v2_request, v2_request_as};
use ion_of_notes::model::{Page, Workspace};
use ion_of_notes::server::{
    ApiError, Capability, Connection, Method, RequestTransport, WorkspaceServer, limits,
};
use std::time::Duration;

fn server() -> WorkspaceServer {
    WorkspaceServer::new(Workspace::seed("limits-tests"))
}

fn error(response: &ion_of_notes::server::ApiResponse) -> ApiError {
    serde_json::from_str(&response.body).expect("structured v2 error")
}

fn retry_after(error: &ApiError) -> &str {
    error
        .additional_data
        .as_ref()
        .and_then(|data| data.get("retry_after"))
        .and_then(|value| value.as_str())
        .expect("additional_data.retry_after")
}

#[test]
fn per_connection_limit_returns_429_with_retry_after() {
    let mut server = server();
    server.configure_rate_limits(2, 100, Duration::from_secs(60));

    for _ in 0..2 {
        assert_eq!(
            server
                .handle(&v2_request(Method::Get, "/api/v2/workspace", None))
                .status,
            200
        );
    }
    let response = server.handle(&v2_request(Method::Get, "/api/v2/workspace", None));
    assert_eq!(response.status, 429);
    let error = error(&response);
    assert_eq!(error.code, "rate_limited");
    assert_eq!(
        error.additional_data.as_ref().unwrap()["rate_limit_reason"],
        "public_api_request_rate_limit"
    );
    assert_eq!(retry_after(&error), "60");
}

#[test]
fn http_adapter_emits_the_retry_after_header() {
    let mut server = server();
    server.configure_rate_limits(1, 100, Duration::from_secs(60));
    assert_eq!(
        server
            .handle(&v2_request(Method::Get, "/api/v2/workspace", None))
            .status,
        200
    );
    let response = server.handle(&v2_request(Method::Get, "/api/v2/workspace", None));
    assert_eq!(response.status, 429);
    let text = String::from_utf8(ion_of_notes::server::format_http_response(&response)).unwrap();
    assert!(text.starts_with("HTTP/1.1 429 Too Many Requests"));
    assert!(text.contains("Retry-After: 60"));
}

#[test]
fn workspace_limit_is_shared_across_connections() {
    let mut server = server();
    server.configure_rate_limits(100, 2, Duration::from_secs(60));
    let workspace_id = server.workspace.settings.id.clone();
    server.register_connection(Connection::new(
        "connection-two",
        "second-token",
        "m-owner",
        workspace_id,
        Capability::all(),
    ));

    assert_eq!(
        server
            .handle(&v2_request(Method::Get, "/api/v2/workspace", None))
            .status,
        200
    );
    assert_eq!(
        server
            .handle(&v2_request(Method::Get, "/api/v2/workspace", None))
            .status,
        200
    );
    let response = server.handle(&v2_request_as(
        "second-token",
        Method::Get,
        "/api/v2/workspace",
        None,
    ));
    assert_eq!(response.status, 429);
    let error = error(&response);
    assert_eq!(
        error.additional_data.as_ref().unwrap()["rate_limit_reason"],
        "public_api_space_request_rate_limit"
    );
}

#[test]
fn overload_returns_529_and_recovers() {
    let mut server = server();
    server.set_overloaded(true);
    let response = server.handle(&v2_request(Method::Get, "/api/v2/workspace", None));
    assert_eq!(response.status, 529);
    let error = error(&response);
    assert_eq!(error.code, "service_overload");
    assert!(!retry_after(&error).is_empty());

    server.set_overloaded(false);
    assert_eq!(
        server
            .handle(&v2_request(Method::Get, "/api/v2/workspace", None))
            .status,
        200
    );
}

#[test]
fn request_byte_boundaries_are_enforced() {
    let base = r#"{"title":"T","icon":"P","parent":null,"padding":""}"#;
    let make = |bytes: usize| {
        let padding = bytes.saturating_sub(base.len());
        base.replace(
            "\"padding\":\"\"",
            &format!("\"padding\":\"{}\"", "x".repeat(padding)),
        )
    };

    assert!(limits::validate_request(Some(&make(499 * 1024))).is_ok());
    assert!(limits::validate_request(Some(&make(500 * 1024))).is_ok());
    let violation = limits::validate_request(Some(&make(500 * 1024 + 1))).unwrap_err();
    assert_eq!(violation.limit, "request_bytes");
}

#[test]
fn endpoint_rejects_oversized_body_before_mutation() {
    let mut server = server();
    let before = server.workspace.pages.len();
    let body = format!(
        r#"{{"title":"{}","icon":"P","parent":null}}"#,
        "x".repeat(501 * 1024)
    );
    let response = server.handle(&v2_request(Method::Post, "/api/v2/pages", Some(body)));
    assert_eq!(response.status, 400);
    assert_eq!(error(&response).code, "validation_error");
    assert_eq!(server.workspace.pages.len(), before);
}

#[test]
fn block_element_boundaries_are_enforced() {
    let make = |count: usize| {
        let children: Vec<serde_json::Value> = (0..count)
            .map(|index| serde_json::json!({ "type": "paragraph", "text": format!("b{index}") }))
            .collect();
        serde_json::json!({ "children": children }).to_string()
    };

    assert!(limits::validate_request(Some(&make(999))).is_ok());
    assert!(limits::validate_request(Some(&make(1000))).is_ok());
    let violation = limits::validate_request(Some(&make(1001))).unwrap_err();
    assert_eq!(violation.limit, "block_elements");
}

#[test]
fn rich_text_url_email_and_array_limits_are_enforced() {
    let text = |content: &str| {
        serde_json::json!({
            "rich_text": [{
                "type": "text",
                "text": { "content": content },
                "annotations": {},
                "plain_text": content
            }]
        })
        .to_string()
    };
    assert!(limits::validate_request(Some(&text(&"a".repeat(2000)))).is_ok());
    assert_eq!(
        limits::validate_request(Some(&text(&"a".repeat(2001))))
            .unwrap_err()
            .limit,
        "rich_text_content"
    );

    let url = |value: &str| serde_json::json!({ "url": value }).to_string();
    assert!(limits::validate_request(Some(&url(&"h".repeat(2000)))).is_ok());
    assert_eq!(
        limits::validate_request(Some(&url(&"h".repeat(2001))))
            .unwrap_err()
            .limit,
        "url"
    );

    let email = serde_json::json!({ "email": "a".repeat(201) }).to_string();
    assert_eq!(
        limits::validate_request(Some(&email)).unwrap_err().limit,
        "email"
    );

    let phone = serde_json::json!({ "phone_number": "1".repeat(201) }).to_string();
    assert_eq!(
        limits::validate_request(Some(&phone)).unwrap_err().limit,
        "phone"
    );

    let multi_select = serde_json::json!({
        "multi_select": (0..101).map(|i| serde_json::json!({ "name": format!("o{i}") })).collect::<Vec<_>>()
    })
    .to_string();
    assert_eq!(
        limits::validate_request(Some(&multi_select))
            .unwrap_err()
            .limit,
        "array_items"
    );
}

#[test]
fn free_multi_member_workspace_hits_the_block_limit() {
    let mut server = server();
    server.set_block_count(1000);
    let response = server.handle(&v2_request(
        Method::Post,
        "/api/v2/pages",
        Some(common::page_draft_json("Blocked", None)),
    ));
    assert_eq!(response.status, 403);
    let error = error(&response);
    assert_eq!(error.code, "restricted_resource");
    assert_eq!(
        error.additional_data.as_ref().unwrap()["block_limit"],
        "block_creation"
    );

    let page_id = server.workspace.pages[0].id.clone();
    assert_eq!(
        server
            .handle(&v2_request(
                Method::Get,
                &format!("/api/v2/pages/{page_id}"),
                None
            ))
            .status,
        200
    );
    assert_eq!(
        server
            .handle(&v2_request(
                Method::Delete,
                &format!("/api/v2/pages/{page_id}"),
                None
            ))
            .status,
        200
    );
}

#[test]
fn the_write_that_reaches_the_block_limit_succeeds() {
    let mut server = server();
    server.set_block_count(999);
    let first = server.handle(&v2_request(
        Method::Post,
        "/api/v2/pages",
        Some(common::page_draft_json("At limit", None)),
    ));
    assert_eq!(first.status, 201);
    let second = server.handle(&v2_request(
        Method::Post,
        "/api/v2/pages",
        Some(common::page_draft_json("Over limit", None)),
    ));
    assert_eq!(second.status, 403);
}

#[test]
fn paid_plans_ignore_the_block_limit() {
    let mut server = server();
    server.set_plan(ion_of_notes::server::Plan::Business);
    server.set_block_count(5000);
    let response = server.handle(&v2_request(
        Method::Post,
        "/api/v2/pages",
        Some(common::page_draft_json("Unlimited", None)),
    ));
    assert_eq!(response.status, 201);
}

#[test]
fn timed_out_write_returns_503_with_retry_guidance() {
    let mut server = server();
    server.simulate_write_timeout(true);
    let response = server.handle(&v2_request(
        Method::Post,
        "/api/v2/pages",
        Some(common::page_draft_json("Saved but timed out", None)),
    ));
    assert_eq!(response.status, 503);
    let error = error(&response);
    assert_eq!(error.code, "service_unavailable");
    let additional = error.additional_data.as_ref().unwrap();
    assert!(
        additional["retry_guidance"]
            .as_array()
            .unwrap()
            .iter()
            .any(|line| line
                .as_str()
                .unwrap_or_default()
                .contains("Do not repeat the write"))
    );
    let committed = additional["committed_resource_id"].as_str().unwrap();
    let page: Page = data(&server.handle(&v2_request(
        Method::Get,
        &format!("/api/v2/pages/{committed}"),
        None,
    )));
    assert_eq!(page.title, "Saved but timed out");
}

#[test]
fn stale_if_match_returns_409_conflict() {
    let mut server = server();
    let mut page = server.workspace.pages[0].clone();
    page.title = "Renamed".to_string();
    let body = serde_json::to_string(&page).unwrap();

    let stale = v2_request(
        Method::Patch,
        &format!("/api/v2/pages/{}", page.id),
        Some(body.clone()),
    )
    .with_header("if-match", "999999");
    let response = server.handle(&stale);
    assert_eq!(response.status, 409);
    let error = error(&response);
    assert_eq!(error.code, "conflict_error");
    assert_eq!(
        error.additional_data.as_ref().unwrap()["current_version"],
        server.workspace.version
    );

    let current = v2_request(
        Method::Patch,
        &format!("/api/v2/pages/{}", page.id),
        Some(body),
    )
    .with_header("if-match", server.workspace.version.to_string());
    assert_eq!(server.handle(&current).status, 200);
}

#[test]
fn request_budget_derives_from_plan_members_and_blocks() {
    let free = ion_of_notes::server::RequestBudget {
        plan: ion_of_notes::server::Plan::Free,
        members: 2,
        blocks: 1000,
    };
    assert!(free.block_limit_reached(1));
    assert_eq!(free.per_connection_limit(), 180);

    let single = ion_of_notes::server::RequestBudget { members: 1, ..free };
    assert!(!single.block_limit_reached(1));

    let business = ion_of_notes::server::RequestBudget {
        plan: ion_of_notes::server::Plan::Business,
        ..free
    };
    assert!(!business.block_limit_reached(1));
    assert_eq!(business.per_connection_limit(), 600);
}
