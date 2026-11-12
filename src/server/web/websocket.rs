use super::http::plain_error;
use super::*;

pub(super) async fn ws_handler(
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
    let snapshot = Arc::clone(&state.snapshot);
    ws.on_upgrade(move |socket| {
        websocket_session(
            socket,
            server,
            snapshot,
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
    snapshot: Arc<RwLock<Option<Arc<crate::model::Workspace>>>>,
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
                            if let Ok(mut snapshot) = snapshot.write() {
                                *snapshot = None;
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
