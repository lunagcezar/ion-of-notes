//! v2 request pipeline: version, auth, limits, capability, then the shared
//! resource handlers.

use super::super::contract::{
    AUTH_HEADER, CODE_CONFLICT, CODE_MISSING_VERSION, CODE_NOT_FOUND, CODE_RATE_LIMITED,
    CODE_RESTRICTED_RESOURCE, CODE_SERVICE_OVERLOAD, CODE_UNAUTHORIZED, CODE_UNSUPPORTED_VERSION,
    CODE_VALIDATION_ERROR, RequestLogEntry, SUPPORTED_VERSIONS, VERSION_HEADER, new_request_id,
    parse_bearer, version_supported,
};
use super::super::limits;
use super::super::rate_limit::REASON_CONNECTION;
use super::super::response::{map_v1_error, parse_body, v2_error, write_timeout_response};
use super::super::{ApiRequest, ApiResponse, Method, WorkspaceServer};
use super::capabilities::required_capability;
use crate::model::{BlockChildrenRequest, Page};
use std::collections::BTreeMap;
use std::time::Instant;

impl WorkspaceServer {
    /// Reject a mutating request whose `If-Match` version is stale.
    fn conflict_response(&self, request: &ApiRequest, request_id: &str) -> Option<ApiResponse> {
        if matches!(request.method, Method::Get) {
            return None;
        }
        let expected = request
            .header("if-match")?
            .trim()
            .trim_matches('"')
            .parse::<u64>()
            .ok()?;
        if expected == self.workspace.version {
            return None;
        }
        Some(v2_error(
            409,
            CODE_CONFLICT,
            "the workspace changed since the request was prepared",
            request_id,
            Some(serde_json::json!({ "current_version": self.workspace.version })),
        ))
    }

    /// Reject a write that would create blocks beyond the Free workspace limit.
    fn block_limit_response(
        &self,
        request: &ApiRequest,
        path: &str,
        request_id: &str,
    ) -> Option<ApiResponse> {
        let additional = self.blocks_created_by(request, path)?;
        if !self.request_budget().block_limit_reached(additional) {
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

    /// How many new blocks a request would add, or `None` when it cannot
    /// create blocks. The page itself counts as one block.
    fn blocks_created_by(&self, request: &ApiRequest, path: &str) -> Option<u64> {
        match (request.method, path) {
            (Method::Post, "/api/v2/pages") | (Method::Post, "/api/v2/databases") => Some(1),
            (Method::Patch, p) if p.starts_with("/api/v2/blocks/") && p.ends_with("/children") => {
                parse_body::<BlockChildrenRequest>(request)
                    .ok()
                    .map(|children| children.children.len() as u64)
            }
            (Method::Patch, p) => {
                let rest = p.strip_prefix("/api/v2/pages/")?;
                if rest.is_empty() || rest.contains('/') {
                    return None;
                }
                let page = parse_body::<Page>(request).ok()?;
                let previous = self
                    .workspace
                    .page(&page.id)
                    .map(Page::block_count)
                    .unwrap_or(0);
                Some(page.block_count().saturating_sub(previous))
            }
            _ => None,
        }
    }

    /// Dispatch a v2 request: version, auth, capability, then the shared v1
    /// use case. The v2 layer changes the envelope/error shape and adds a
    /// request id; it does not duplicate the business logic.
    pub(super) fn handle_v2(&mut self, request: &ApiRequest) -> ApiResponse {
        let request_id = new_request_id();
        let (path, query) = match request.path.split_once('?') {
            Some((p, q)) => (p, q),
            None => (request.path.as_str(), ""),
        };
        let is_write = !matches!(request.method, Method::Get);
        let timed_out = is_write && self.simulate_write_timeout;
        if timed_out {
            self.simulate_write_timeout = false;
        }
        let response = self.handle_v2_inner(request, path, query, &request_id);
        let response = if timed_out && (200..300).contains(&response.status) {
            write_timeout_response(&response, &request_id)
        } else {
            response
        };
        let status = response.status;
        let response = ApiResponse {
            request_id: Some(request_id.clone()),
            ..response
        };
        self.request_log.push(RequestLogEntry {
            request_id,
            method: request.method.as_str().to_string(),
            path: request.path.clone(),
            status,
        });
        response
    }

    pub(super) fn handle_v2_inner(
        &mut self,
        request: &ApiRequest,
        path: &str,
        query: &str,
        request_id: &str,
    ) -> ApiResponse {
        let Some(version) = request.header(VERSION_HEADER) else {
            return v2_error(
                400,
                CODE_MISSING_VERSION,
                format!("{VERSION_HEADER} header is required"),
                request_id,
                None,
            );
        };
        if !version_supported(version) {
            return v2_error(
                400,
                CODE_UNSUPPORTED_VERSION,
                format!("unsupported version: {version}"),
                request_id,
                Some(serde_json::json!({ "supported_versions": SUPPORTED_VERSIONS })),
            );
        }

        if request.method == Method::Post && path == "/api/v2/auth/login" {
            let decision =
                self.rate_limiter
                    .check("auth-login", &self.workspace.settings.id, Instant::now());
            if !decision.allowed {
                let reason = decision.reason.unwrap_or(REASON_CONNECTION);
                return v2_error(
                    429,
                    CODE_RATE_LIMITED,
                    format!("rate limit exceeded: {reason}"),
                    request_id,
                    Some(serde_json::json!({
                        "rate_limit_reason": reason,
                        "retry_after": decision.retry_after.to_string(),
                    })),
                );
            }
            return self.login(request, request_id);
        }

        let Some(token) = request.header(AUTH_HEADER).and_then(parse_bearer) else {
            return v2_error(
                401,
                CODE_UNAUTHORIZED,
                "missing or malformed bearer token",
                request_id,
                None,
            );
        };
        let Some(connection) = self.connection_for_token(token) else {
            return v2_error(
                401,
                CODE_UNAUTHORIZED,
                "invalid bearer token",
                request_id,
                None,
            );
        };

        if self.overloaded {
            return v2_error(
                529,
                CODE_SERVICE_OVERLOAD,
                "the service is temporarily overloaded; retry after the wait",
                request_id,
                Some(serde_json::json!({ "retry_after": "5" })),
            );
        }
        let decision =
            self.rate_limiter
                .check(&connection.id, &connection.workspace_id, Instant::now());
        if !decision.allowed {
            let reason = decision.reason.unwrap_or(REASON_CONNECTION);
            return v2_error(
                429,
                CODE_RATE_LIMITED,
                format!("rate limit exceeded: {reason}"),
                request_id,
                Some(serde_json::json!({
                    "rate_limit_reason": reason,
                    "retry_after": decision.retry_after.to_string(),
                })),
            );
        }
        if !matches!(request.method, Method::Get)
            && let Err(violation) = limits::validate_request(request.body.as_deref())
        {
            return v2_error(
                400,
                CODE_VALIDATION_ERROR,
                violation.message,
                request_id,
                Some(serde_json::json!({ "limit": violation.limit })),
            );
        }
        if let Some(response) = self.conflict_response(request, request_id) {
            return response;
        }
        if let Some(response) = self.block_limit_response(request, path, request_id) {
            return response;
        }

        let Some(required) = required_capability(request.method, path) else {
            return v2_error(404, CODE_NOT_FOUND, "unknown route", request_id, None);
        };
        if !connection.has(required) {
            return v2_error(
                403,
                CODE_RESTRICTED_RESOURCE,
                format!("missing capability: {}", required.as_str()),
                request_id,
                Some(serde_json::json!({ "required_capability": required.as_str() })),
            );
        }

        if let Some(response) = self.handle_v2_crdt(request, path, request_id, &connection) {
            return response;
        }
        if let Some(response) = self.handle_v2_auth(request, path, request_id, &connection) {
            return response;
        }

        self.workspace.ensure_data_sources();
        self.workspace.ensure_views();
        if let Some(response) = self.handle_v2_webhook(request, path, request_id) {
            return response;
        }
        if let Some(response) =
            self.handle_v2_comment_user_file(request, path, query, request_id, &connection)
        {
            return response;
        }
        if let Some(response) = self.handle_v2_view(request, path, query, request_id) {
            return response;
        }
        if let Some(response) = self.handle_v2_database(request, path, query, request_id) {
            return response;
        }
        if let Some(response) = self.handle_v2_block(request, path, query, request_id) {
            return response;
        }
        if let Some(response) = self.handle_v2_list(request, path, query, request_id) {
            return response;
        }

        let v1_request = ApiRequest {
            method: request.method,
            path: request.path.replacen("/api/v2", "/api", 1),
            body: request.body.clone(),
            headers: BTreeMap::new(),
        };
        let v1 = self.handle_v1(&v1_request);
        if v1.status >= 400 {
            return map_v1_error(v1, request_id);
        }
        ApiResponse {
            status: v1.status,
            body: v1.body,
            request_id: None,
        }
    }
}
