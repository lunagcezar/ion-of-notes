//! Remote `WorkspaceApi` adapter: blocking REST + background WebSocket (spec-044).
//!
//! REST calls use ureq so the egui thread stays synchronous. A background
//! thread owns the WebSocket, reconnects with `?since=` and forwards typed
//! `ServerMessage`s to the UI through a Tokio broadcast channel.

use crate::auth::{LoginInput, SessionResponse};
use crate::model::Workspace;
use crate::server::realtime::{Awareness, ClientMessage, ServerMessage};
use crate::server::{
    API_VERSION, AUTH_HEADER, ApiRequest, ApiResponse, CrdtSyncInput, CrdtSyncOutput,
    CrdtUpdateInput, CrdtUpdateOutput, Envelope, Method, VERSION_HEADER, WorkspaceApi, url_encode,
};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use futures_util::{SinkExt, StreamExt};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use tokio_tungstenite::tungstenite::Message;

enum WsCommand {
    Awareness(Awareness),
}

/// A persisted remote login. Only the session token is stored; the password is
/// never written to disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteSession {
    pub base_url: String,
    pub email: String,
    pub token: String,
}

/// Default location of the remembered remote session.
pub fn session_path() -> PathBuf {
    crate::store::data_path().with_file_name("remote-session.json")
}

/// Persist a session at an explicit path (tests, alternate profiles).
pub fn save_session_at(path: &Path, session: &RemoteSession) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(session)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    std::fs::write(path, text)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

/// Load a session for `base_url`, ignoring missing/malformed/other-server files.
pub fn load_session_at(path: &Path, base_url: &str) -> Option<RemoteSession> {
    let text = std::fs::read_to_string(path).ok()?;
    let session: RemoteSession = serde_json::from_str(&text).ok()?;
    let expected = base_url.trim_end_matches('/');
    (session.base_url.trim_end_matches('/') == expected).then_some(session)
}

/// Remove a saved session. Missing files are fine.
pub fn clear_session_at(path: &Path) -> io::Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

/// Persist the remembered session in the default profile location.
pub fn save_session(base_url: &str, email: &str, token: &str) -> io::Result<()> {
    save_session_at(
        &session_path(),
        &RemoteSession {
            base_url: base_url.to_string(),
            email: email.to_string(),
            token: token.to_string(),
        },
    )
}

/// Load the remembered session for `base_url`.
pub fn load_session(base_url: &str) -> Option<RemoteSession> {
    load_session_at(&session_path(), base_url)
}

/// Forget the remembered session.
pub fn clear_session() -> io::Result<()> {
    clear_session_at(&session_path())
}

/// Log in to a remote server and return the session token.
pub fn login(base_url: &str, email: &str, password: &str) -> Result<SessionResponse, String> {
    let base = base_url.trim_end_matches('/');
    let body = serde_json::to_string(&LoginInput {
        email: email.to_string(),
        password: password.to_string(),
    })
    .map_err(|error| error.to_string())?;
    let mut response = agent()
        .post(format!("{base}/api/v2/auth/login"))
        .header(VERSION_HEADER, API_VERSION)
        .header("Content-Type", "application/json")
        .send(&body)
        .map_err(|error| error.to_string())?;
    let status = response.status().as_u16();
    let text = response
        .body_mut()
        .read_to_string()
        .map_err(|error| error.to_string())?;
    if !(200..300).contains(&status) {
        return Err(text);
    }
    let envelope: Envelope<SessionResponse> =
        serde_json::from_str(&text).map_err(|error| error.to_string())?;
    envelope.data.ok_or_else(|| {
        envelope
            .error
            .unwrap_or_else(|| "empty response".to_string())
    })
}

/// Remote transport used by the desktop when `--server` is set.
pub struct RemoteApi {
    base: String,
    token: String,
    agent: ureq::Agent,
    realtime_rx: Arc<Mutex<Option<broadcast::Receiver<ServerMessage>>>>,
    ws_tx: mpsc::UnboundedSender<WsCommand>,
}

impl RemoteApi {
    /// Connect the REST client and start the background WebSocket.
    pub fn connect(base: &str, token: &str) -> Result<Self, String> {
        let base = base.trim_end_matches('/').to_string();
        let agent = agent();
        let workspace: Workspace = get_json(&agent, &format!("{base}/api/workspace"), None)?;
        let workspace_id = workspace.settings.id;
        let (realtime_tx, realtime_rx) = broadcast::channel(256);
        let (ws_tx, ws_rx) = mpsc::unbounded_channel();
        spawn_ws(
            base.clone(),
            token.to_string(),
            workspace_id,
            realtime_tx.clone(),
            ws_rx,
        );
        Ok(Self {
            base,
            token: token.to_string(),
            agent,
            realtime_rx: Arc::new(Mutex::new(Some(realtime_rx))),
            ws_tx,
        })
    }

    fn post_json<T: Serialize, R: DeserializeOwned>(
        &self,
        path: &str,
        value: &T,
    ) -> Result<R, String> {
        let body = serde_json::to_string(value).map_err(|error| error.to_string())?;
        let mut response = self
            .agent
            .post(format!("{}{}", self.base, path))
            .header(VERSION_HEADER, API_VERSION)
            .header(AUTH_HEADER, format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .send(&body)
            .map_err(|error| error.to_string())?;
        let status = response.status().as_u16();
        let text = response
            .body_mut()
            .read_to_string()
            .map_err(|error| error.to_string())?;
        if !(200..300).contains(&status) {
            return Err(text);
        }
        let envelope: Envelope<R> =
            serde_json::from_str(&text).map_err(|error| error.to_string())?;
        envelope.data.ok_or_else(|| {
            envelope
                .error
                .unwrap_or_else(|| "empty response".to_string())
        })
    }
}

impl WorkspaceApi for RemoteApi {
    fn handle(&mut self, request: &ApiRequest) -> ApiResponse {
        let url = format!("{}{}", self.base, request.path);
        match execute(
            &self.agent,
            request.method,
            &url,
            &request.headers,
            request.body.as_deref(),
        ) {
            Ok((status, request_id, body)) => ApiResponse {
                status,
                body,
                request_id,
            },
            Err(error) => ApiResponse {
                status: 503,
                body: serde_json::to_string(&Envelope::<()>::err(error)).unwrap_or_default(),
                request_id: None,
            },
        }
    }

    fn crdt_sync(
        &mut self,
        page_id: &str,
        state_vector: Option<&[u8]>,
    ) -> Result<(Vec<u8>, Vec<u8>), String> {
        let output: CrdtSyncOutput = self.post_json(
            "/api/v2/crdt/sync",
            &CrdtSyncInput {
                page_id: page_id.to_string(),
                state_vector: state_vector.map(|bytes| BASE64.encode(bytes)),
            },
        )?;
        let update = BASE64
            .decode(output.update)
            .map_err(|error| error.to_string())?;
        let vector = BASE64
            .decode(output.state_vector)
            .map_err(|error| error.to_string())?;
        Ok((update, vector))
    }

    fn apply_crdt_update(
        &mut self,
        page_id: &str,
        update: &[u8],
        actor: &str,
    ) -> Result<Vec<u8>, String> {
        let output: CrdtUpdateOutput = self.post_json(
            "/api/v2/crdt/update",
            &CrdtUpdateInput {
                page_id: page_id.to_string(),
                update: BASE64.encode(update),
                actor: Some(actor.to_string()),
            },
        )?;
        BASE64
            .decode(output.update)
            .map_err(|error| error.to_string())
    }

    fn subscribe_realtime(&self) -> Option<broadcast::Receiver<ServerMessage>> {
        self.realtime_rx
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
    }

    fn publish_awareness(&mut self, awareness: Awareness) -> Result<(), String> {
        self.ws_tx
            .send(WsCommand::Awareness(awareness))
            .map_err(|error| error.to_string())
    }
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .http_status_as_error(false)
        .max_redirects(2)
        .build()
        .into()
}

fn get_json<T: DeserializeOwned>(
    agent: &ureq::Agent,
    url: &str,
    token: Option<&str>,
) -> Result<T, String> {
    let mut request = agent.get(url);
    if let Some(token) = token {
        request = request.header(AUTH_HEADER, format!("Bearer {token}"));
    }
    let mut response = request.call().map_err(|error| error.to_string())?;
    let status = response.status().as_u16();
    let text = response
        .body_mut()
        .read_to_string()
        .map_err(|error| error.to_string())?;
    if !(200..300).contains(&status) {
        return Err(text);
    }
    let envelope: Envelope<T> = serde_json::from_str(&text).map_err(|error| error.to_string())?;
    envelope.data.ok_or_else(|| {
        envelope
            .error
            .unwrap_or_else(|| "empty response".to_string())
    })
}

fn execute(
    agent: &ureq::Agent,
    method: Method,
    url: &str,
    headers: &BTreeMap<String, String>,
    body: Option<&str>,
) -> Result<(u16, Option<String>, String), String> {
    let mut response = match method {
        Method::Get => {
            let mut builder = agent.get(url);
            for (name, value) in headers {
                builder = builder.header(name, value);
            }
            builder.call()
        }
        Method::Delete => {
            let mut builder = agent.delete(url);
            for (name, value) in headers {
                builder = builder.header(name, value);
            }
            builder.call()
        }
        Method::Post => {
            let mut builder = agent.post(url);
            for (name, value) in headers {
                builder = builder.header(name, value);
            }
            builder.send(body.unwrap_or(""))
        }
        Method::Put => {
            let mut builder = agent.put(url);
            for (name, value) in headers {
                builder = builder.header(name, value);
            }
            builder.send(body.unwrap_or(""))
        }
        Method::Patch => {
            let mut builder = agent.patch(url);
            for (name, value) in headers {
                builder = builder.header(name, value);
            }
            builder.send(body.unwrap_or(""))
        }
    }
    .map_err(|error| error.to_string())?;
    let status = response.status().as_u16();
    let request_id = response
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let body = response
        .body_mut()
        .read_to_string()
        .map_err(|error| error.to_string())?;
    Ok((status, request_id, body))
}

fn spawn_ws(
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
