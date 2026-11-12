//! Remote `WorkspaceApi` adapter: blocking REST + background WebSocket (spec-044).
//!
//! REST calls use ureq so the egui thread stays synchronous. A background
//! thread owns the WebSocket, reconnects with `?since=` and forwards typed
//! `ServerMessage`s to the UI through a Tokio broadcast channel.

use crate::auth::{LoginInput, SessionResponse};
use crate::model::Workspace;
use crate::server::realtime::{Awareness, ClientMessage, ServerMessage};
use crate::server::{
    API_VERSION, AUTH_HEADER, ApiRequest, ApiResponse, CollabTransport, CrdtSyncInput,
    CrdtSyncOutput, CrdtUpdateInput, CrdtUpdateOutput, Envelope, Method, RequestTransport,
    VERSION_HEADER, url_encode,
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

mod crdt;
mod http;
mod realtime;
mod session;

use http::{agent, execute, get_json};
use realtime::spawn_ws;
pub use session::{
    RemoteSession, clear_session, clear_session_at, load_session, load_session_at, save_session,
    save_session_at, session_path,
};

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

/// REST-only remote transport, safe to move into a background worker.
///
/// It deliberately has no real-time state: the UI keeps the WebSocket in
/// [`RemoteApi`], while blocking REST/CRDT calls run on the worker thread
/// (spec-108).
#[derive(Clone)]
pub struct RemoteRest {
    base: String,
    token: String,
    agent: ureq::Agent,
}

impl RemoteRest {
    /// Build a REST client for an existing base URL and token.
    pub fn connect(base: &str, token: &str) -> Self {
        Self {
            base: base.trim_end_matches('/').to_string(),
            token: token.to_string(),
            agent: agent(),
        }
    }

    /// The bearer token this client sends.
    pub fn token(&self) -> &str {
        &self.token
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

impl RequestTransport for RemoteRest {
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
}

/// Remote transport used by the desktop when `--server` is set.
pub struct RemoteApi {
    rest: RemoteRest,
    realtime_rx: Arc<Mutex<Option<broadcast::Receiver<ServerMessage>>>>,
    ws_tx: mpsc::Sender<WsCommand>,
}

impl RemoteApi {
    /// Connect the REST client and start the background WebSocket.
    pub fn connect(base: &str, token: &str) -> Result<Self, String> {
        let rest = RemoteRest::connect(base, token);
        let workspace: Workspace =
            get_json(&rest.agent, &format!("{}/api/workspace", rest.base), None)?;
        let workspace_id = workspace.settings.id;
        let (realtime_tx, realtime_rx) = broadcast::channel(256);
        let (ws_tx, ws_rx) = mpsc::channel(64);
        spawn_ws(
            rest.base.clone(),
            rest.token.clone(),
            workspace_id,
            realtime_tx.clone(),
            ws_rx,
        );
        Ok(Self {
            rest,
            realtime_rx: Arc::new(Mutex::new(Some(realtime_rx))),
            ws_tx,
        })
    }

    /// A REST-only clone for the background request worker.
    pub fn rest(&self) -> RemoteRest {
        self.rest.clone()
    }
}

impl RequestTransport for RemoteApi {
    fn handle(&mut self, request: &ApiRequest) -> ApiResponse {
        self.rest.handle(request)
    }
}
