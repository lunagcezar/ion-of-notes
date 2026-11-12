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
use super::realtime::{ClientMessage, PresenceMember, RealtimeHub, ServerMessage};
use super::{
    ApiRequest, ApiResponse, Connection, Method, RequestTransport, WorkspaceServer, parse_bearer,
};
use crate::collab::Change;

mod http;
mod operations;
mod websocket;

use axum::Router;
use axum::body::Body;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{DefaultBodyLimit, Path, Query, Request, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use http::handle;
use operations::{health, metrics, ready};
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;
use websocket::ws_handler;

/// Transport-level body cap. The v2 contract still enforces its own 500 KB
/// limit below this, so clients see the documented `validation_error`.
const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;
/// Upper bound for one HTTP request (the synchronous server runs in
/// `spawn_blocking`, so this protects the proxy from hung requests).
const REQUEST_TIMEOUT_SECS: u64 = 30;

#[derive(Clone)]
struct AppState {
    server: Arc<Mutex<WorkspaceServer>>,
    /// Immutable workspace snapshot for read-only v1 routes (spec-111).
    snapshot: Arc<RwLock<Option<Arc<crate::model::Workspace>>>>,
    started: Instant,
    requests: Arc<AtomicU64>,
}

/// Build the Axum router around one server instance.
///
/// The router is intentionally a fallback: the existing `WorkspaceServer`
/// already owns routing, versioning, auth and capabilities. The adapter only
/// translates the transport.
pub fn router(server: WorkspaceServer) -> Router {
    router_with_server(Arc::new(Mutex::new(server)))
}

/// Build the router around an already shared server (tests, embedding).
pub fn router_with_server(server: Arc<Mutex<WorkspaceServer>>) -> Router {
    let state = AppState {
        server,
        snapshot: Arc::new(RwLock::new(None)),
        started: Instant::now(),
        requests: Arc::new(AtomicU64::new(0)),
    };
    Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/metrics", get(metrics))
        .route("/ws/workspace/{id}", get(ws_handler))
        .fallback(handle)
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(REQUEST_TIMEOUT_SECS),
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Liveness probe. Kept separate from `/ready` so healthchecks do not count as
/// API traffic in `/metrics`.
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
        let shared = Arc::new(Mutex::new(server));
        spawn_webhook_worker(Arc::clone(&shared));
        axum::serve(listener, router_with_server(shared))
            .with_graceful_shutdown(shutdown_signal())
            .await
    })
}

/// Periodically flush aggregated webhooks and retry failed deliveries.
fn spawn_webhook_worker(server: Arc<Mutex<WorkspaceServer>>) {
    let interval = std::env::var("ION_OF_NOTES_WEBHOOK_INTERVAL_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(30)
        .max(1);
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(interval));
        ticker.tick().await;
        loop {
            ticker.tick().await;
            let server = Arc::clone(&server);
            let _ = tokio::task::spawn_blocking(move || {
                if let Ok(mut server) = server.lock() {
                    server.flush_webhooks();
                    server.process_webhook_deliveries();
                }
            })
            .await;
        }
    });
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tower_http=info"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let mut terminate = match signal(SignalKind::terminate()) {
            Ok(signal) => signal,
            Err(error) => {
                eprintln!("failed to listen for SIGTERM: {error}");
                let _ = tokio::signal::ctrl_c().await;
                return;
            }
        };
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = terminate.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        if let Err(error) = tokio::signal::ctrl_c().await {
            eprintln!("failed to listen for Ctrl+C: {error}");
        }
    }
}
