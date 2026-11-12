//! Cross-cutting stages of the v2 request pipeline.
//!
//! The stages are small functions with one return type so the ordering is
//! explicit and each rule can be tested in isolation.

use super::super::contract::{
    AUTH_HEADER, CODE_MISSING_VERSION, CODE_NOT_FOUND, CODE_RATE_LIMITED, CODE_RESTRICTED_RESOURCE,
    CODE_SERVICE_OVERLOAD, CODE_UNAUTHORIZED, CODE_UNSUPPORTED_VERSION, CODE_VALIDATION_ERROR,
    SUPPORTED_VERSIONS, VERSION_HEADER, parse_bearer, version_supported,
};
use super::super::limits;
use super::super::rate_limit::{REASON_CONNECTION, RateDecision};
use super::super::response::{parse_body, v2_error};
use super::super::{ApiRequest, ApiResponse, Connection, Method, WorkspaceServer};
use super::capabilities::{BlockLimit, route_policy};
use crate::auth::LoginInput;
use crate::model::{BlockChildrenRequest, Page};
use std::time::Instant;

pub(super) fn check_version(request: &ApiRequest, request_id: &str) -> Result<(), ApiResponse> {
    let Some(version) = request.header(VERSION_HEADER) else {
        return Err(v2_error(
            400,
            CODE_MISSING_VERSION,
            format!("{VERSION_HEADER} header is required"),
            request_id,
            None,
        ));
    };
    if !version_supported(version) {
        return Err(v2_error(
            400,
            CODE_UNSUPPORTED_VERSION,
            format!("unsupported version: {version}"),
            request_id,
            Some(serde_json::json!({ "supported_versions": SUPPORTED_VERSIONS })),
        ));
    }
    Ok(())
}

pub(super) fn check_login_rate(
    server: &mut WorkspaceServer,
    request: &ApiRequest,
    request_id: &str,
) -> Result<(), ApiResponse> {
    let email = request
        .body
        .as_deref()
        .and_then(|body| serde_json::from_str::<LoginInput>(body).ok())
        .map(|input| input.email.trim().to_lowercase())
        .filter(|email| !email.is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    let client = request.client_ip.as_deref().unwrap_or("local");
    let now = Instant::now();
    let workspace_key = format!("login:{}", server.workspace.settings.id);
    let client_decision =
        server
            .runtime
            .login_limiter
            .check(&format!("login-ip:{client}"), &workspace_key, now);
    if !client_decision.allowed {
        return Err(rate_limit_response(client_decision, request_id));
    }
    let account_decision =
        server
            .runtime
            .login_limiter
            .check(&format!("login-account:{email}"), &workspace_key, now);
    if !account_decision.allowed {
        return Err(rate_limit_response(account_decision, request_id));
    }
    Ok(())
}

pub(super) fn authenticate(
    server: &WorkspaceServer,
    request: &ApiRequest,
    request_id: &str,
) -> Result<Connection, ApiResponse> {
    let Some(token) = request.header(AUTH_HEADER).and_then(parse_bearer) else {
        return Err(v2_error(
            401,
            CODE_UNAUTHORIZED,
            "missing or malformed bearer token",
            request_id,
            None,
        ));
    };
    server.connection_for_token(token).ok_or_else(|| {
        v2_error(
            401,
            CODE_UNAUTHORIZED,
            "invalid bearer token",
            request_id,
            None,
        )
    })
}

pub(super) fn check_runtime(
    server: &mut WorkspaceServer,
    connection: &Connection,
    request: &ApiRequest,
    path: &str,
    request_id: &str,
) -> Result<(), ApiResponse> {
    if server.runtime.overloaded {
        return Err(v2_error(
            529,
            CODE_SERVICE_OVERLOAD,
            "the service is temporarily overloaded; retry after the wait",
            request_id,
            Some(serde_json::json!({ "retry_after": "5" })),
        ));
    }
    let decision =
        server
            .runtime
            .rate_limiter
            .check(&connection.id, &connection.workspace_id, Instant::now());
    if !decision.allowed {
        return Err(rate_limit_response(decision, request_id));
    }
    if !matches!(request.method, Method::Get)
        && let Err(violation) = limits::validate_request(request.body.as_deref())
    {
        return Err(v2_error(
            400,
            CODE_VALIDATION_ERROR,
            violation.message,
            request_id,
            Some(serde_json::json!({ "limit": violation.limit })),
        ));
    }
    if let Some(response) = conflict_response(server, request, request_id) {
        return Err(response);
    }
    if let Some(response) = block_limit_response(server, request, path, request_id) {
        return Err(response);
    }
    Ok(())
}

pub(super) fn check_capability(
    connection: &Connection,
    request: &ApiRequest,
    path: &str,
    request_id: &str,
) -> Result<(), ApiResponse> {
    let Some(required) = super::capabilities::route_capability(request.method, path) else {
        return Err(v2_error(
            404,
            CODE_NOT_FOUND,
            "unknown route",
            request_id,
            None,
        ));
    };
    if !connection.has(required) {
        return Err(v2_error(
            403,
            CODE_RESTRICTED_RESOURCE,
            format!("missing capability: {}", required.as_str()),
            request_id,
            Some(serde_json::json!({ "required_capability": required.as_str() })),
        ));
    }
    Ok(())
}

pub(super) fn check_authorization(
    server: &WorkspaceServer,
    connection: &Connection,
    request: &ApiRequest,
    path: &str,
    request_id: &str,
) -> Result<(), ApiResponse> {
    let Some(policy) = route_policy(request.method, path) else {
        return Ok(());
    };
    if policy.owner_only && !server.is_owner(connection) {
        return Err(v2_error(
            403,
            CODE_RESTRICTED_RESOURCE,
            "owner access required",
            request_id,
            Some(serde_json::json!({ "required_role": "owner" })),
        ));
    }
    if policy.page_scoped
        && let Some(page_id) = page_id(path)
    {
        let write = !matches!(request.method, Method::Get);
        if write && !server.can_write_page(connection, page_id) {
            return Err(v2_error(
                403,
                CODE_RESTRICTED_RESOURCE,
                "write access to this page is not allowed",
                request_id,
                Some(serde_json::json!({
                    "page_id": page_id,
                    "required_access": "write",
                })),
            ));
        }
        if !write && !server.can_read_page(connection, page_id) {
            return Err(v2_error(
                403,
                CODE_RESTRICTED_RESOURCE,
                "read access to this page is not allowed",
                request_id,
                Some(serde_json::json!({
                    "page_id": page_id,
                    "required_access": "read",
                })),
            ));
        }
    }
    Ok(())
}

pub(super) fn rate_limit_response(decision: RateDecision, request_id: &str) -> ApiResponse {
    let reason = decision.reason.unwrap_or(REASON_CONNECTION);
    v2_error(
        429,
        CODE_RATE_LIMITED,
        format!("rate limit exceeded: {reason}"),
        request_id,
        Some(serde_json::json!({
            "rate_limit_reason": reason,
            "retry_after": decision.retry_after.to_string(),
        })),
    )
}

fn conflict_response(
    server: &WorkspaceServer,
    request: &ApiRequest,
    request_id: &str,
) -> Option<ApiResponse> {
    if matches!(request.method, Method::Get) {
        return None;
    }
    let expected = request
        .header("if-match")?
        .trim()
        .trim_matches('"')
        .parse::<u64>()
        .ok()?;
    if expected == server.workspace.version {
        return None;
    }
    Some(v2_error(
        409,
        super::super::contract::CODE_CONFLICT,
        "the workspace changed since the request was prepared",
        request_id,
        Some(serde_json::json!({ "current_version": server.workspace.version })),
    ))
}

fn block_limit_response(
    server: &WorkspaceServer,
    request: &ApiRequest,
    path: &str,
    request_id: &str,
) -> Option<ApiResponse> {
    let additional = blocks_created_by(server, request, path)?;
    if !server.request_budget().block_limit_reached(additional) {
        return None;
    }
    Some(v2_error(
        403,
        CODE_RESTRICTED_RESOURCE,
        "This workspace has used all of its free blocks. Upgrade its plan to create more content.",
        request_id,
        Some(serde_json::json!({ "block_limit": "block_creation" })),
    ))
}

fn blocks_created_by(server: &WorkspaceServer, request: &ApiRequest, path: &str) -> Option<u64> {
    let policy = route_policy(request.method, path)?;
    match policy.block_limit {
        BlockLimit::None => None,
        BlockLimit::PageCreate | BlockLimit::DatabaseCreate => Some(1),
        BlockLimit::BlockChildren => parse_body::<BlockChildrenRequest>(request)
            .ok()
            .map(|children| children.children.len() as u64),
        BlockLimit::PageReplace => {
            let page = parse_body::<Page>(request).ok()?;
            let previous = server
                .workspace
                .page(&page.id)
                .map(Page::block_count)
                .unwrap_or(0);
            Some(page.block_count().saturating_sub(previous))
        }
    }
}

fn page_id(path: &str) -> Option<&str> {
    let rest = path.strip_prefix("/api/v2/pages/")?;
    let id = rest.strip_suffix("/versions").unwrap_or(rest);
    if id.is_empty() || id.contains('/') {
        return None;
    }
    Some(id)
}
