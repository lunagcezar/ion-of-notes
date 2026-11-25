use super::http::plain_error;
use super::*;
use crate::server::realtime::Awareness;

/// Upper bound for one WebSocket client text frame. A CRDT update larger than
/// this is rejected before JSON parsing or base64 decoding.
const MAX_WS_MESSAGE_BYTES: usize = 1024 * 1024;

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
    let (connection, hub, backlog, seq, member_name, reset_required) = {
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
            .filter(|change| server.change_visible(&connection, &change.page_id))
            .cloned()
            .collect::<Vec<_>>();
        let seq = server.collab.next_seq.saturating_sub(1);
        let reset_required = server
            .collab
            .changes
            .front()
            .is_some_and(|change| since.saturating_add(1) < change.seq);
        let backlog = if reset_required { Vec::new() } else { backlog };
        (
            connection,
            server.realtime(),
            backlog,
            seq,
            member_name,
            reset_required,
        )
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
            reset_required,
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
    reset_required: bool,
) {
    let session_id = format!("ws-{}", uuid::Uuid::new_v4());
    let mut budget = crate::server::MessageBudget::new(120, std::time::Duration::from_secs(10));
    let mut receiver = hub.subscribe();
    let presence_name = member_name.clone();
    hub.join(PresenceMember {
        session_id: session_id.clone(),
        member_id: connection.member_id.clone(),
        name: presence_name,
        state: "active".to_string(),
    });
    if !send_json(
        &mut socket,
        &ServerMessage::Ready {
            workspace_id,
            session_id: session_id.clone(),
            seq,
            reset_required,
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
                    let Some(message) = message_visible(&server, &connection, message).await else {
                        continue;
                    };
                    if !send_json(&mut socket, &message).await {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    let _ = send_json(
                        &mut socket,
                        &ServerMessage::Resync {
                            reason: format!("{skipped} messages were dropped; reload the workspace"),
                        },
                    )
                    .await;
                    break;
                }
                Err(broadcast::error::RecvError::Closed) => break,
            },
            incoming = socket.recv() => match incoming {
                Some(Ok(Message::Text(text))) => {
                    if !budget.allow(std::time::Instant::now()) {
                        let _ = send_error(
                            &mut socket,
                            "rate_limited",
                            "too many websocket messages; reconnect after a pause",
                        )
                        .await;
                        break;
                    }
                    if text.len() > MAX_WS_MESSAGE_BYTES {
                        if !send_error(&mut socket, "message_too_large", "websocket message is too large").await {
                            break;
                        }
                        continue;
                    }
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
                            if !handle_crdt_sync(
                                &mut socket,
                                &server,
                                &connection,
                                page_id,
                                state_vector,
                            )
                            .await
                            {
                                break;
                            }
                        }
                        Ok(ClientMessage::CrdtUpdate { page_id, update }) => {
                            if !handle_crdt_update(
                                &mut socket,
                                &server,
                                &hub,
                                &connection,
                                page_id,
                                update,
                            )
                            .await
                            {
                                break;
                            }
                        }
                        Ok(ClientMessage::Awareness { mut awareness }) => {
                            awareness.session_id = session_id.clone();
                            // Identity always comes from the authenticated
                            // session, never from the client payload
                            // (spec-140).
                            awareness.member_id = connection.member_id.clone();
                            awareness.name = member_name.clone();
                            match validate_awareness(&server, &connection, &awareness).await {
                                Ok(()) => hub.update_awareness(awareness),
                                Err(reason) => {
                                    if !send_error(&mut socket, "awareness_rejected", reason).await {
                                        break;
                                    }
                                }
                            }
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
    connection: &Connection,
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
        if !server.can_read_page(connection, &page_id) {
            Err("read access to this page is not allowed".to_string())
        } else {
            server.crdt_sync(&page_id, state_vector.as_deref())
        }
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
    connection: &Connection,
    page_id: String,
    update: String,
) -> bool {
    let update = match BASE64.decode(update) {
        Ok(bytes) => bytes,
        Err(error) => return send_error(socket, "crdt_error", error.to_string()).await,
    };
    let result = {
        let mut server = server
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !server.can_write_page(connection, &page_id) {
            Err("write access to this page is not allowed".to_string())
        } else {
            server.apply_crdt_update(&page_id, &update, &connection.member_id)
        }
    };
    match result {
        Ok(merged) => {
            hub.publish_crdt_update(&page_id, &merged);
            true
        }
        Err(error) => send_error(socket, "crdt_error", error).await,
    }
}

/// One broadcast message projected for one connection.
///
/// Awareness is filtered entry by entry so one unreadable cursor cannot drop
/// the whole batch, and no entry leaks a page the member cannot read
/// (spec-140).
async fn message_visible(
    server: &Arc<Mutex<WorkspaceServer>>,
    connection: &Connection,
    message: ServerMessage,
) -> Option<ServerMessage> {
    let page_id = match &message {
        ServerMessage::Change { change } => Some(change.page_id.as_str()),
        ServerMessage::CrdtUpdate { page_id, .. } | ServerMessage::CrdtSync { page_id, .. } => {
            Some(page_id.as_str())
        }
        ServerMessage::Awareness { .. } => {
            let guard = server
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let ServerMessage::Awareness { members } = message else {
                return None;
            };
            let members: Vec<_> = members
                .into_iter()
                .filter(|awareness| guard.can_read_page(connection, &awareness.page_id))
                .collect();
            return (!members.is_empty()).then_some(ServerMessage::Awareness { members });
        }
        ServerMessage::Ready { .. }
        | ServerMessage::Resync { .. }
        | ServerMessage::Presence { .. }
        | ServerMessage::Pong
        | ServerMessage::Error { .. } => return Some(message),
    };
    let Some(page_id) = page_id else {
        return Some(message);
    };
    let guard = server
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    guard.change_visible(connection, page_id).then_some(message)
}

/// Validate one awareness update against the authenticated session and the
/// page tree before it reaches the hub (spec-140).
async fn validate_awareness(
    server: &Arc<Mutex<WorkspaceServer>>,
    connection: &Connection,
    awareness: &Awareness,
) -> Result<(), String> {
    let guard = server
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if !guard.can_read_page(connection, &awareness.page_id) {
        return Err("read access to this page is not allowed".to_string());
    }
    if guard.workspace.page(&awareness.page_id).is_none() {
        return Err("page not found".to_string());
    }
    if guard
        .workspace
        .page_id_for_block(&awareness.block_id)
        .as_deref()
        != Some(awareness.page_id.as_str())
    {
        return Err("block does not belong to the page".to_string());
    }
    let Some(block) = guard.workspace.block(&awareness.block_id) else {
        return Err("block not found".to_string());
    };
    let length = block.text.chars().count();
    if awareness.index > length {
        return Err("cursor index is outside the block".to_string());
    }
    if let Some(end) = awareness.selection_end
        && end > length
    {
        return Err("selection end is outside the block".to_string());
    }
    Ok(())
}
