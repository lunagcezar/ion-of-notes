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
    ApiRequest, ApiResponse, Connection, Method, WorkspaceApi, WorkspaceServer, parse_bearer,
};
use crate::collab::Change;
use axum::Router;
use axum::body::Body;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{DefaultBodyLimit, Path, Query, Request, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
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
    router_with_server(Arc::new(Mutex::new(server)))
}

/// Build the router around an already shared server (tests, embedding).
pub fn router_with_server(server: Arc<Mutex<WorkspaceServer>>) -> Router {
    let state = AppState { server };
    Router::new()
        .route("/ws/workspace/{id}", get(ws_handler))
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

async fn ws_handler(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    let token = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(parse_bearer)
        .map(str::to_string)
        .or_else(|| query.get("token").cloned());
    let Some(token) = token else {
        return plain_error(StatusCode::UNAUTHORIZED, "missing token");
    };
    let since = query
        .get("since")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);
    let (connection, hub, backlog, seq, member_name) = {
        let server = state
            .server
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if server.workspace.settings.id != workspace_id {
            return plain_error(StatusCode::NOT_FOUND, "workspace not found");
        }
        let Some(connection) = server.connection_for_token(&token) else {
            return plain_error(StatusCode::UNAUTHORIZED, "invalid token");
        };
        let member_name = server
            .collab
            .members
            .iter()
            .find(|member| member.id == connection.member_id)
            .map(|member| member.name.clone())
            .unwrap_or_else(|| connection.member_id.clone());
        let backlog = server
            .collab
            .changes
            .iter()
            .filter(|change| change.seq > since)
            .cloned()
            .collect::<Vec<_>>();
        let seq = server.collab.next_seq.saturating_sub(1);
        (connection, server.realtime(), backlog, seq, member_name)
    };
    let server = Arc::clone(&state.server);
    ws.on_upgrade(move |socket| {
        websocket_session(
            socket,
            server,
            connection,
            hub,
            backlog,
            seq,
            workspace_id,
            member_name,
        )
    })
}

#[allow(clippy::too_many_arguments)]
async fn websocket_session(
    mut socket: WebSocket,
    server: Arc<Mutex<WorkspaceServer>>,
    connection: Connection,
    hub: RealtimeHub,
    backlog: Vec<Change>,
    seq: u64,
    workspace_id: String,
    member_name: String,
) {
    let session_id = format!("ws-{}", uuid::Uuid::new_v4());
    let mut receiver = hub.subscribe();
    hub.join(PresenceMember {
        session_id: session_id.clone(),
        member_id: connection.member_id.clone(),
        name: member_name,
        state: "active".to_string(),
    });
    if !send_json(
        &mut socket,
        &ServerMessage::Ready {
            workspace_id,
            session_id: session_id.clone(),
            seq,
        },
    )
    .await
    {
        hub.leave(&session_id);
        return;
    }
    for change in backlog {
        if !send_json(&mut socket, &ServerMessage::Change { change }).await {
            hub.leave(&session_id);
            return;
        }
    }
    loop {
        tokio::select! {
            event = receiver.recv() => match event {
                Ok(message) => {
                    if !send_json(&mut socket, &message).await {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            },
            incoming = socket.recv() => match incoming {
                Some(Ok(Message::Text(text))) => {
                    match serde_json::from_str::<ClientMessage>(text.as_str()) {
                        Ok(ClientMessage::Ping) => {
                            if !send_json(&mut socket, &ServerMessage::Pong).await {
                                break;
                            }
                        }
                        Ok(ClientMessage::CrdtSync {
                            page_id,
                            state_vector,
                        }) => {
                            if !handle_crdt_sync(&mut socket, &server, page_id, state_vector).await {
                                break;
                            }
                        }
                        Ok(ClientMessage::CrdtUpdate { page_id, update }) => {
                            if !handle_crdt_update(
                                &mut socket,
                                &server,
                                &hub,
                                page_id,
                                update,
                                &connection.member_id,
                            )
                            .await
                            {
                                break;
                            }
                        }
                        Ok(ClientMessage::Awareness { mut awareness }) => {
                            awareness.session_id = session_id.clone();
                            hub.update_awareness(awareness);
                        }
                        Err(_) => {}
                    }
                }
                Some(Ok(Message::Ping(payload))) => {
                    if socket.send(Message::Pong(payload)).await.is_err() {
                        break;
                    }
                }
                Some(Ok(Message::Close(_))) | None => break,
                Some(Ok(_)) => {}
                Some(Err(_)) => break,
            },
        }
    }
    hub.leave(&session_id);
}

async fn send_json(socket: &mut WebSocket, message: &ServerMessage) -> bool {
    let Ok(text) = serde_json::to_string(message) else {
        return false;
    };
    socket.send(Message::Text(text.into())).await.is_ok()
}

async fn send_error(socket: &mut WebSocket, code: &str, message: impl Into<String>) -> bool {
    send_json(
        socket,
        &ServerMessage::Error {
            code: code.to_string(),
            message: message.into(),
        },
    )
    .await
}

async fn handle_crdt_sync(
    socket: &mut WebSocket,
    server: &Arc<Mutex<WorkspaceServer>>,
    page_id: String,
    state_vector: Option<String>,
) -> bool {
    let state_vector = match state_vector {
        Some(value) => match BASE64.decode(value) {
            Ok(bytes) => Some(bytes),
            Err(error) => return send_error(socket, "crdt_error", error.to_string()).await,
        },
        None => None,
    };
    let result = {
        let mut server = server
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        server.crdt_sync(&page_id, state_vector.as_deref())
    };
    match result {
        Ok((update, vector)) => {
            send_json(
                socket,
                &ServerMessage::CrdtSync {
                    page_id,
                    update: BASE64.encode(update),
                    state_vector: BASE64.encode(vector),
                },
            )
            .await
        }
        Err(error) => send_error(socket, "crdt_error", error).await,
    }
}

async fn handle_crdt_update(
    socket: &mut WebSocket,
    server: &Arc<Mutex<WorkspaceServer>>,
    hub: &RealtimeHub,
    page_id: String,
    update: String,
    actor: &str,
) -> bool {
    let update = match BASE64.decode(update) {
        Ok(bytes) => bytes,
        Err(error) => return send_error(socket, "crdt_error", error.to_string()).await,
    };
    let result = {
        let mut server = server
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        server.apply_crdt_update(&page_id, &update, actor)
    };
    match result {
        Ok(merged) => {
            hub.publish_crdt_update(&page_id, &merged);
            true
        }
        Err(error) => send_error(socket, "crdt_error", error).await,
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
