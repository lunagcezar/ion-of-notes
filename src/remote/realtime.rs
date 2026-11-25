use super::*;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

pub(super) fn spawn_ws(
    base: String,
    token: String,
    workspace_id: String,
    realtime_tx: broadcast::Sender<ServerMessage>,
    mut ws_rx: mpsc::Receiver<WsCommand>,
    pending_awareness: Arc<PendingAwareness>,
) {
    std::thread::spawn(move || {
        let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        else {
            return;
        };
        runtime.block_on(async move {
            let mut since = 0u64;
            let mut backoff = 1u64;
            loop {
                let url = format!("{}?since={since}", websocket_url(&base, &workspace_id));
                let mut request = match url.as_str().into_client_request() {
                    Ok(request) => request,
                    Err(_) => return,
                };
                if let Ok(value) = HeaderValue::from_str(&format!("Bearer {token}")) {
                    request.headers_mut().insert("authorization", value);
                }
                if let Ok((mut socket, _)) = tokio_tungstenite::connect_async(request).await {
                    backoff = 1;
                    if !flush_awareness(&mut socket, &pending_awareness).await {
                        continue;
                    }
                    loop {
                        tokio::select! {
                            message = socket.next() => match message {
                                Some(Ok(Message::Text(text))) => {
                                    if let Ok(message) = serde_json::from_str::<ServerMessage>(text.as_str()) {
                                        match &message {
                                            ServerMessage::Ready { seq, .. } => since = since.max(*seq),
                                            ServerMessage::Change { change } => since = since.max(change.seq),
                                            ServerMessage::Resync { .. } => since = 0,
                                            _ => {}
                                        }
                                        let _ = realtime_tx.send(message);
                                    }
                                }
                                Some(Ok(Message::Ping(payload))) => {
                                    let _ = socket.send(Message::Pong(payload)).await;
                                }
                                Some(Ok(Message::Close(_))) | None => break,
                                Some(Err(_)) => break,
                                _ => {}
                            },
                            command = ws_rx.recv() => match command {
                                Some(WsCommand::Awareness) => {
                                    if !flush_awareness(&mut socket, &pending_awareness).await {
                                        break;
                                    }
                                }
                                None => return,
                            },
                        }
                    }
                }
                tokio::time::sleep(Duration::from_secs(backoff)).await;
                backoff = (backoff * 2).min(30);
            }
        });
    });
}

fn websocket_url(base: &str, workspace_id: &str) -> String {
    let base = base
        .replacen("https://", "wss://", 1)
        .replacen("http://", "ws://", 1);
    format!("{base}/ws/workspace/{workspace_id}")
}

/// Send the latest pending cursor, if any. Returns false when the socket is
/// gone so the caller can fall back to the reconnect loop.
async fn flush_awareness(
    socket: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    pending: &Arc<PendingAwareness>,
) -> bool {
    let Some(awareness) = pending.snapshot() else {
        return true;
    };
    let Ok(text) = serde_json::to_string(&ClientMessage::Awareness {
        awareness: awareness.clone(),
    }) else {
        return true;
    };
    if socket.send(Message::Text(text.into())).await.is_err() {
        return false;
    }
    pending.clear_if(&awareness);
    true
}
