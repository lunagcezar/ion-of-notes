use super::*;
use crate::model::{Page, PageSummary, Workspace};
use crate::server::contract::{AUTH_HEADER, OWNER_TOKEN, parse_bearer};
use crate::server::response::{forbidden, json, not_found, unauthorized};
use crate::server::{Connection, Envelope};
use std::sync::RwLock;

pub(super) async fn handle(State(state): State<AppState>, request: Request) -> Response {
    state.requests.fetch_add(1, Ordering::Relaxed);
    let (parts, body) = request.into_parts();
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
    let client_ip = headers
        .get("x-forwarded-for")
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| headers.get("x-real-ip").cloned());
    let api_request = ApiRequest {
        method,
        path,
        body,
        headers,
        client_ip,
    };

    if api_request.method == Method::Get
        && let Some(response) = snapshot_read(&state, &api_request).await
    {
        return to_http(response);
    }

    let is_read = api_request.method == Method::Get;
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
            return plain_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("server task failed: {error}"),
            );
        }
    };
    if !is_read && let Ok(mut snapshot) = state.snapshot.write() {
        *snapshot = None;
    }
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
    cache: &RwLock<Option<Arc<Workspace>>>,
    request: &ApiRequest,
) -> Option<ApiResponse> {
    let path = request.path.split('?').next().unwrap_or("");
    let guard = server
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let connection = match authenticate(&guard, request) {
        Ok(connection) => connection,
        Err(response) => return Some(response),
    };
    if path == "/api/workspace" {
        let workspace = snapshot_for(&guard, cache);
        drop(guard);
        return Some(json(200, Envelope::ok(workspace.as_ref())));
    }
    if path == "/api/pages" {
        let workspace = snapshot_for(&guard, cache);
        drop(guard);
        let pages: Vec<PageSummary> = workspace.pages.iter().map(Page::summary).collect();
        return Some(json(200, Envelope::ok(pages)));
    }
    let page_id = path.strip_prefix("/api/pages/")?;
    if page_id.is_empty() || page_id.contains('/') {
        return None;
    }
    if !guard.can_read_page(&connection, page_id) {
        drop(guard);
        return Some(forbidden("read access to this page is not allowed"));
    }
    let workspace = snapshot_for(&guard, cache);
    drop(guard);
    match workspace.page(page_id) {
        Some(page) => Some(json(200, Envelope::ok(page.clone()))),
        None => Some(not_found("page not found")),
    }
}

fn authenticate(server: &WorkspaceServer, request: &ApiRequest) -> Result<Connection, ApiResponse> {
    if server.requires_auth() {
        let Some(token) = request.header(AUTH_HEADER).and_then(parse_bearer) else {
            return Err(unauthorized("missing or malformed bearer token"));
        };
        server
            .connection_for_token(token)
            .ok_or_else(|| unauthorized("invalid bearer token"))
    } else {
        Ok(server.connection_for_token(OWNER_TOKEN).unwrap_or_else(|| {
            Connection::owner(
                server.collab.owner_id(),
                server.workspace.settings.id.clone(),
            )
        }))
    }
}

fn snapshot_for(
    server: &WorkspaceServer,
    cache: &RwLock<Option<Arc<Workspace>>>,
) -> Arc<Workspace> {
    let version = server.workspace.version;
    if let Ok(guard) = cache.read()
        && let Some(cached) = guard.as_ref()
        && cached.version == version
    {
        return Arc::clone(cached);
    }
    let workspace = Arc::new(server.workspace.clone());
    if let Ok(mut guard) = cache.write() {
        *guard = Some(Arc::clone(&workspace));
    }
    workspace
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
