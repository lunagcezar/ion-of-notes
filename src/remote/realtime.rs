use super::*;

pub(super) fn spawn_ws(
    base: String,
    token: String,
    workspace_id: String,
    realtime_tx: broadcast::Sender<ServerMessage>,
    mut ws_rx: mpsc::UnboundedReceiver<WsCommand>,
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
                let url = format!(
                    "{}?token={}&since={since}",
                    websocket_url(&base, &workspace_id),
                    url_encode(&token)
                );
                if let Ok((mut socket, _)) = tokio_tungstenite::connect_async(url.as_str()).await {
                    backoff = 1;
                    loop {
                        tokio::select! {
                            message = socket.next() => match message {
                                Some(Ok(Message::Text(text))) => {
                                    if let Ok(message) = serde_json::from_str::<ServerMessage>(text.as_str()) {
                                        match &message {
                                            ServerMessage::Ready { seq, .. } => since = since.max(*seq),
                                            ServerMessage::Change { change } => since = since.max(change.seq),
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
                                Some(WsCommand::Awareness(awareness)) => {
                                    if let Ok(text) = serde_json::to_string(&ClientMessage::Awareness { awareness }) {
                                        let _ = socket.send(Message::Text(text.into())).await;
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
