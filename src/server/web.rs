//! Axum HTTP adapter for the self-hosted API (spec-038).
//!
//! This is an infrastructure module: it translates HTTP requests into the
//! transport-neutral [`ApiRequest`](super::ApiRequest), calls the existing
//! `WorkspaceServer` through the `WorkspaceApi` seam, and translates the
//! response back. No Axum type leaks into the domain or application core.
//!
//! The synchronous server runs in `spawn_blocking` behind a mutex, so the
//! Tokio reactor is never blocked by workspace work.

use super::http::retry_after_seconds;
use super::{ApiRequest, ApiResponse, Method, WorkspaceApi, WorkspaceServer};
use axum::Router;
use axum::body::Body;
use axum::extract::{DefaultBodyLimit, Request, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

/// Transport-level body cap. The v2 contract still enforces its own 500 KB
/// limit below this, so clients see the documented `validation_error`.
const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone)]
struct AppState {
    server: Arc<Mutex<WorkspaceServer>>,
}

/// Build the Axum router around one server instance.
///
/// The router is intentionally a fallback: the existing `WorkspaceServer`
/// already owns routing, versioning, auth and capabilities. The adapter only
/// translates the transport.
pub fn router(server: WorkspaceServer) -> Router {
    let state = AppState {
        server: Arc::new(Mutex::new(server)),
    };
    Router::new()
        .fallback(handle)
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Run the Axum server until Ctrl+C.
pub fn serve_axum(addr: &str, server: WorkspaceServer) -> std::io::Result<()> {
    init_tracing();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(std::io::Error::other)?;
    runtime.block_on(async move {
        let listener = tokio::net::TcpListener::bind(addr).await?;
        let local = listener.local_addr()?;
        println!("ion-of-notes API listening on http://{local}");
        axum::serve(listener, router(server))
            .with_graceful_shutdown(shutdown_signal())
            .await
    })
}

async fn handle(State(state): State<AppState>, request: Request) -> Response {
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
    let api_request = ApiRequest {
        method,
        path,
        body,
        headers,
    };

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
    to_http(response)
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

fn plain_error(status: StatusCode, message: &str) -> Response {
    (status, message.to_string()).into_response()
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tower_http=info"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

async fn shutdown_signal() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        eprintln!("failed to listen for Ctrl+C: {error}");
    }
}
