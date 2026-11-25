use super::*;
use crate::model::{Page, PageSummary};
use crate::server::contract::{CODE_NOT_FOUND, CODE_RESTRICTED_RESOURCE};
use crate::server::pagination::PageRequest;
use crate::server::response::{
    forbidden, json, not_found, query_value, service_unavailable, unauthorized, v2_error,
    v2_list_response,
};
use crate::server::{Connection, Envelope, V2ReadContext, WorkspaceSnapshot, new_request_id};
use axum::extract::ConnectInfo;
use std::net::{IpAddr, SocketAddr};
use std::sync::{OnceLock, RwLock};

pub(super) async fn handle(State(state): State<AppState>, request: Request) -> Response {
    state.requests.fetch_add(1, Ordering::Relaxed);
    let (parts, body) = request.into_parts();
    let peer = parts
        .extensions
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(addr)| *addr);
    let body = match axum::body::to_bytes(body, MAX_BODY_BYTES).await {
        Ok(bytes) => (!bytes.is_empty()).then(|| String::from_utf8_lossy(&bytes).into_owned()),
        Err(_) => {
            return plain_error(StatusCode::PAYLOAD_TOO_LARGE, "request body is too large");
        }
    };
    let Some(method) = api_method(parts.method.as_str()) else {
        return plain_error(StatusCode::METHOD_NOT_ALLOWED, "unsupported method");
    };
    let path = parts
        .uri
        .path_and_query()
        .map(|path| path.as_str())
        .unwrap_or("/")
        .to_string();
    let headers: BTreeMap<String, String> = parts
        .headers
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.as_str().to_string(), value.to_string()))
        })
        .collect();
    let client_ip = trusted_client_ip(peer, &headers);
    let api_request = ApiRequest {
        method,
        path,
        body,
        headers,
        client_ip,
    };

    if api_request.method == Method::Get
        && let Some(response) = snapshot_v2_read(&state, &api_request).await
    {
        return to_http(response);
    }
    if api_request.method == Method::Get
        && let Some(response) = snapshot_read(&state, &api_request).await
    {
        return to_http(response);
    }

    let server = Arc::clone(&state.server);
    let response = match tokio::task::spawn_blocking(move || {
        let mut guard = server
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.handle(&api_request)
    })
    .await
    {
        Ok(response) => response,
        Err(error) => {
            tracing::error!(%error, "server task failed");
            return plain_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "the request could not be completed",
            );
        }
    };
    // The snapshot cache is keyed by `workspace.version`, which every write
    // bumps, so there is no need to invalidate it by hand (spec-141).
    to_http(response)
}

/// Serve the expensive read-only v1 routes from an immutable workspace snapshot
/// without holding the server mutex during serialization (spec-111).
async fn snapshot_read(state: &AppState, request: &ApiRequest) -> Option<ApiResponse> {
    let path = request.path.split('?').next().unwrap_or("");
    let page_id = path.strip_prefix("/api/pages/");
    let is_page = page_id.is_some_and(|id| !id.is_empty() && !id.contains('/'));
    if path != "/api/workspace" && path != "/api/pages" && !is_page {
        return None;
    }
    let server = Arc::clone(&state.server);
    let snapshot = Arc::clone(&state.snapshot);
    let request = request.clone();
    tokio::task::spawn_blocking(move || read_snapshot(&server, &snapshot, &request))
        .await
        .ok()
        .flatten()
}

fn read_snapshot(
    server: &std::sync::Mutex<WorkspaceServer>,
    cache: &RwLock<Option<WorkspaceSnapshot>>,
    request: &ApiRequest,
) -> Option<ApiResponse> {
    let path = request.path.split('?').next().unwrap_or("");
    // Hold the lock only long enough to authenticate and take the snapshot;
    // projection and JSON serialization run after it is released (spec-143).
    let (connection, snapshot) = {
        let guard = server
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let connection = match authenticate(&guard, request) {
            Ok(connection) => connection,
            Err(response) => return Some(response),
        };
        let snapshot = workspace_snapshot(&guard, cache);
        (connection, snapshot)
    };
    let view = snapshot.view();
    let workspace = &snapshot.workspace;
    if path == "/api/workspace" {
        let projected = if view.is_owner(&connection) {
            (**workspace).clone()
        } else {
            view.project_workspace(&connection)
        };
        return Some(json(200, Envelope::ok(projected)));
    }
    if path == "/api/pages" {
        let pages: Vec<PageSummary> = if view.is_owner(&connection) {
            workspace.pages.iter().map(Page::summary).collect()
        } else {
            view.visible_page_summaries(&connection)
        };
        return Some(json(200, Envelope::ok(pages)));
    }
    let page_id = path.strip_prefix("/api/pages/")?;
    if page_id.is_empty() || page_id.contains('/') {
        return None;
    }
    if !view.can_read_page(&connection, page_id) {
        return Some(forbidden("read access to this page is not allowed"));
    }
    match workspace.page(page_id) {
        Some(page) => Some(json(200, Envelope::ok(page.clone()))),
        None => Some(not_found("page not found")),
    }
}

fn authenticate(server: &WorkspaceServer, request: &ApiRequest) -> Result<Connection, ApiResponse> {
    server
        .authenticate_local_or_session(request)
        .map_err(|error| match error.status() {
            503 => service_unavailable(error.message()),
            _ => unauthorized(error.message()),
        })
}

/// Serve the heavy v2 GET routes from an immutable snapshot: version, auth,
/// rate-limit and capability checks run under the lock, then projection and
/// serialization run after it is released (spec-150).
async fn snapshot_v2_read(state: &AppState, request: &ApiRequest) -> Option<ApiResponse> {
    let path = request.path.split('?').next().unwrap_or("");
    let is_page = path
        .strip_prefix("/api/v2/pages/")
        .is_some_and(|id| !id.is_empty() && !id.contains('/'));
    if path != "/api/v2/workspace"
        && path != "/api/v2/pages"
        && path != "/api/v2/search"
        && !is_page
    {
        return None;
    }
    let server = Arc::clone(&state.server);
    let cache = Arc::clone(&state.snapshot);
    let prepare_request = request.clone();
    let request_id = new_request_id();
    let prepared = tokio::task::spawn_blocking(move || {
        let mut guard = server
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match guard.prepare_v2_read(&prepare_request, &request_id, &cache) {
            Ok(context) => Ok(context),
            Err(mut response) => {
                let status = response.status;
                response.request_id = Some(request_id.clone());
                guard.record_request(&prepare_request, status, &request_id);
                Err(response)
            }
        }
    })
    .await
    .ok()?;
    let context = match prepared {
        Ok(context) => context,
        Err(response) => return Some(response),
    };
    let response = build_v2_read(&context);
    let status = response.status;
    let log_id = context.request_id.clone();
    let server = Arc::clone(&state.server);
    let log_request = request.clone();
    tokio::task::spawn_blocking(move || {
        if let Ok(mut guard) = server.lock() {
            guard.record_request(&log_request, status, &log_id);
        }
    })
    .await
    .ok()?;
    Some(ApiResponse {
        request_id: Some(context.request_id),
        ..response
    })
}

fn build_v2_read(context: &V2ReadContext) -> ApiResponse {
    let view = context.snapshot.view();
    let workspace = &context.snapshot.workspace;
    let request_id = &context.request_id;
    if context.path == "/api/v2/workspace" {
        let projected = if view.is_owner(&context.connection) {
            (**workspace).clone()
        } else {
            view.project_workspace(&context.connection)
        };
        return json(200, Envelope::ok(projected));
    }
    if context.path == "/api/v2/pages" {
        let page = match PageRequest::from_query(&context.query) {
            Ok(page) => page,
            Err(error) => {
                return v2_error(400, error.code(), error.message(), request_id, None);
            }
        };
        let items: Vec<PageSummary> = if view.is_owner(&context.connection) {
            workspace.pages.iter().map(Page::summary).collect()
        } else {
            view.visible_page_summaries(&context.connection)
        };
        return v2_list_response(items, &page, "pages", "page", request_id);
    }
    if context.path == "/api/v2/search" {
        let page = match PageRequest::from_query(&context.query) {
            Ok(page) => page,
            Err(error) => {
                return v2_error(400, error.code(), error.message(), request_id, None);
            }
        };
        let q = query_value(&context.query, "q");
        let results = view.visible_search(&context.connection, &q);
        let sort = format!("search:{q}");
        return v2_list_response(results, &page, &sort, "search_result", request_id);
    }
    let Some(page_id) = context.path.strip_prefix("/api/v2/pages/") else {
        return v2_error(404, CODE_NOT_FOUND, "unknown route", request_id, None);
    };
    if !view.can_read_page(&context.connection, page_id) {
        return v2_error(
            403,
            CODE_RESTRICTED_RESOURCE,
            "read access to this page is not allowed",
            request_id,
            Some(serde_json::json!({ "required_access": "read" })),
        );
    }
    match workspace.page(page_id) {
        Some(page) => json(200, Envelope::ok(page.clone())),
        None => v2_error(404, CODE_NOT_FOUND, "page not found", request_id, None),
    }
}

fn api_method(method: &str) -> Option<Method> {
    match method {
        "GET" => Some(Method::Get),
        "POST" => Some(Method::Post),
        "PUT" => Some(Method::Put),
        "PATCH" => Some(Method::Patch),
        "DELETE" => Some(Method::Delete),
        _ => None,
    }
}

fn to_http(response: ApiResponse) -> Response {
    let status = StatusCode::from_u16(response.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let mut builder = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json; charset=utf-8");
    if let Some(request_id) = &response.request_id {
        builder = builder.header("x-request-id", request_id);
    }
    if let Some(seconds) = retry_after_seconds(&response) {
        builder = builder.header(header::RETRY_AFTER, seconds.to_string());
    }
    builder.body(Body::from(response.body)).unwrap_or_else(|_| {
        plain_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "could not build the response",
        )
    })
}

pub(super) fn plain_error(status: StatusCode, message: &str) -> Response {
    (status, message.to_string()).into_response()
}

/// Resolve the client address used by login rate limiting.
///
/// `X-Forwarded-For` and `X-Real-IP` are honored only when the direct peer is a
/// configured trusted proxy (loopback by default). Otherwise the socket peer is
/// used, so a public client cannot rotate the login bucket with a header.
fn trusted_client_ip(
    peer: Option<SocketAddr>,
    headers: &BTreeMap<String, String>,
) -> Option<String> {
    let peer_ip = peer.map(|addr| addr.ip());
    let trusted = peer_ip.is_some_and(is_trusted_proxy);
    if !trusted {
        return peer_ip.map(|ip| ip.to_string());
    }
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| headers.get("x-real-ip").cloned())
        .or_else(|| peer_ip.map(|ip| ip.to_string()))
}

/// Trusted reverse proxies: loopback always, plus `ION_OF_NOTES_TRUSTED_PROXIES`
/// (comma-separated IPs or `*`). The list is read once per process.
fn is_trusted_proxy(ip: IpAddr) -> bool {
    if ip.is_loopback() {
        return true;
    }
    trusted_proxy_list().iter().any(|entry| match entry {
        TrustedProxy::Any => true,
        TrustedProxy::Address(address) => *address == ip,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrustedProxy {
    Any,
    Address(IpAddr),
}

fn trusted_proxy_list() -> &'static [TrustedProxy] {
    static LIST: OnceLock<Vec<TrustedProxy>> = OnceLock::new();
    LIST.get_or_init(|| {
        std::env::var("ION_OF_NOTES_TRUSTED_PROXIES")
            .unwrap_or_default()
            .split(',')
            .filter_map(|entry| {
                let entry = entry.trim();
                if entry == "*" {
                    Some(TrustedProxy::Any)
                } else {
                    entry.parse::<IpAddr>().ok().map(TrustedProxy::Address)
                }
            })
            .collect()
    })
}
