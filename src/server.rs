//! Server side of the application.
//!
//! `WorkspaceServer` owns all state and answers `ApiRequest`s through the
//! `WorkspaceApi` seam. `serve_once` / `serve_forever` expose the very same
//! contract over loopback HTTP/1.1 for the optional `--serve` mode.
//!
//! This root file is the module facade and the state owner. Route adapters
//! live in `server::routes`; one-resource use cases live in
//! `server::handlers`; shared response shaping lives in `server::response`.

use crate::auth::{self, AuthStore, NoAuthStore};
use crate::collab::{ChangeOp, CollabState};
use crate::model::Workspace;
use crate::store::{
    CollabStore, JsonCollabStore, JsonStore, NoCollabStore, NoStore, WorkspaceStore,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

mod contract;
pub mod crdt;
mod handlers;
mod http;
pub mod limits;
mod pagination;
mod rate_limit;
pub mod realtime;
mod response;
mod routes;
mod url;
mod usecases;
pub mod web;
pub mod webhook_sender;

pub use contract::{
    API_VERSION, AUTH_HEADER, ApiError, CODE_CONFLICT, CODE_INTERNAL, CODE_INVALID_CURSOR,
    CODE_INVALID_JSON, CODE_MISSING_VERSION, CODE_NOT_FOUND, CODE_RATE_LIMITED,
    CODE_RESTRICTED_RESOURCE, CODE_SERVICE_OVERLOAD, CODE_SERVICE_UNAVAILABLE, CODE_UNAUTHORIZED,
    CODE_UNSUPPORTED_VERSION, CODE_VALIDATION_ERROR, Capability, Connection, ListResponse,
    OWNER_TOKEN, RequestLogEntry, SUPPORTED_VERSIONS, VERSION_HEADER, new_request_id, parse_bearer,
    version_supported,
};
pub use crdt::CrdtStore;
pub use http::{format_http_response, parse_http_request, serve_forever, serve_on, serve_once};
pub use limits::{
    FREE_MULTI_MEMBER_BLOCK_LIMIT, LimitViolation, MAX_ARRAY_ITEMS, MAX_BLOCK_ELEMENTS,
    MAX_EMAIL_CHARS, MAX_EQUATION_CHARS, MAX_PHONE_CHARS, MAX_REQUEST_BYTES, MAX_RICH_TEXT_CHARS,
    MAX_URL_CHARS, Plan, RequestBudget, validate_request,
};
pub use pagination::{PageRequest, PaginationError, paginate};
pub use rate_limit::{
    REASON_CONNECTION, REASON_WORKSPACE, RateDecision, RateLimitPolicy, RateLimiter,
    SqliteRateLimiter,
};
pub use routes::capabilities::route_capability;
pub use url::url_encode;
pub use web::serve_axum;
pub use webhook_sender::{UreqWebhookSender, WebhookSender};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl Method {
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_uppercase().as_str() {
            "GET" => Some(Method::Get),
            "POST" => Some(Method::Post),
            "PUT" => Some(Method::Put),
            "PATCH" => Some(Method::Patch),
            "DELETE" => Some(Method::Delete),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Patch => "PATCH",
            Method::Delete => "DELETE",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiRequest {
    pub method: Method,
    pub path: String,
    pub body: Option<String>,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
}

impl ApiRequest {
    pub fn new(method: Method, path: impl Into<String>, body: Option<String>) -> Self {
        Self {
            method,
            path: path.into(),
            body,
            headers: BTreeMap::new(),
        }
    }

    /// Add one header, preserving the caller's spelling but looking it up
    /// case-insensitively.
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    /// Case-insensitive header lookup.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiResponse {
    pub status: u16,
    pub body: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

/// REST CRDT sync request (remote clients).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtSyncInput {
    pub page_id: String,
    #[serde(default)]
    pub state_vector: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtSyncOutput {
    pub update: String,
    pub state_vector: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtUpdateInput {
    pub page_id: String,
    pub update: String,
    #[serde(default)]
    pub actor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtUpdateOutput {
    pub update: String,
}

/// The JSON envelope used for every response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Envelope<T> {
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> Envelope<T> {
    pub fn ok(data: T) -> Self {
        Self {
            data: Some(data),
            error: None,
        }
    }

    pub fn err(message: impl Into<String>) -> Self {
        Self {
            data: None,
            error: Some(message.into()),
        }
    }
}

/// The seam between client and server.
pub trait WorkspaceApi {
    fn handle(&mut self, request: &ApiRequest) -> ApiResponse;

    /// CRDT sync for the in-process desktop client. Remote transports override
    /// this through their own adapter.
    fn crdt_sync(
        &mut self,
        _page_id: &str,
        _state_vector: Option<&[u8]>,
    ) -> Result<(Vec<u8>, Vec<u8>), String> {
        Err("CRDT is not available on this transport".to_string())
    }

    /// Apply one CRDT update.
    fn apply_crdt_update(
        &mut self,
        _page_id: &str,
        _update: &[u8],
        _actor: &str,
    ) -> Result<Vec<u8>, String> {
        Err("CRDT is not available on this transport".to_string())
    }

    /// Subscribe to the in-process real-time hub, when available.
    fn subscribe_realtime(
        &self,
    ) -> Option<tokio::sync::broadcast::Receiver<realtime::ServerMessage>> {
        None
    }

    /// Publish cursor/selection awareness, when available.
    fn publish_awareness(&mut self, _awareness: realtime::Awareness) -> Result<(), String> {
        Err("awareness is not available on this transport".to_string())
    }
}

/// Owns all workspace state. Persistence is injected through the
/// `WorkspaceStore` and `CollabStore` ports, so the core never depends on a
/// concrete backend. Collaboration state (accounts, permissions, sync log,
/// versions) is kept beside the workspace and exposed through the decided API.
pub struct WorkspaceServer {
    pub workspace: Workspace,
    pub collab: CollabState,
    persistence: Persistence,
    runtime: Runtime,
    webhooks: WebhookRuntime,
    realtime: realtime::RealtimeHub,
    crdt: crdt::CrdtStore,
}

/// The three injected persistence ports.
struct Persistence {
    workspace_store: Box<dyn WorkspaceStore>,
    collab_store: Box<dyn CollabStore>,
    auth_store: Box<dyn AuthStore>,
}

/// Per-process connections, request metrics and operational limits.
struct Runtime {
    connections: Vec<Connection>,
    request_log: Vec<RequestLogEntry>,
    plan: Plan,
    block_count_override: Option<u64>,
    block_count_baseline: u64,
    overloaded: bool,
    simulate_write_timeout: bool,
    rate_limiter: Box<dyn RateLimitPolicy>,
    shared_rate_store: Option<Arc<dyn crate::store::RateLimitStore>>,
}

/// Webhook delivery state, separate from the collaboration records.
struct WebhookRuntime {
    allow_insecure: bool,
    queue: Vec<crate::webhooks::WebhookEvent>,
    sender: Box<dyn WebhookSender>,
}

impl WorkspaceServer {
    /// An in-memory server with no persistence (the null-object adapters).
    pub fn new(workspace: Workspace) -> Self {
        Self::with_stores(workspace, CollabState::new(), NoStore, NoCollabStore)
    }

    /// A server that persists workspace mutations to a JSON file.
    ///
    /// Collaboration state stays in memory; use [`Self::with_paths`] when both
    /// documents should be durable.
    pub fn with_path(workspace: Workspace, path: PathBuf) -> Self {
        Self::with_stores(
            workspace,
            CollabState::new(),
            JsonStore::new(path),
            NoCollabStore,
        )
    }

    /// A server that persists both the workspace and collaboration state.
    pub fn with_paths(workspace: Workspace, workspace_path: PathBuf, collab_path: PathBuf) -> Self {
        let collab = crate::store::load_collab_or_seed(&collab_path);
        Self::with_stores(
            workspace,
            collab,
            JsonStore::new(workspace_path),
            JsonCollabStore::new(collab_path),
        )
    }

    /// Inject a workspace persistence adapter (DIP/OCP).
    pub fn with_store<S: WorkspaceStore + 'static>(workspace: Workspace, store: S) -> Self {
        Self::with_stores(workspace, CollabState::new(), store, NoCollabStore)
    }

    /// Inject both persistence adapters without changing server logic.
    pub fn with_stores<S, C>(
        workspace: Workspace,
        collab: CollabState,
        store: S,
        collab_store: C,
    ) -> Self
    where
        S: WorkspaceStore + 'static,
        C: CollabStore + 'static,
    {
        Self::with_stores_and_auth(workspace, collab, store, collab_store, NoAuthStore)
    }

    /// Inject workspace, collaboration and auth adapters.
    pub fn with_stores_and_auth<S, C, A>(
        workspace: Workspace,
        collab: CollabState,
        store: S,
        collab_store: C,
        auth_store: A,
    ) -> Self
    where
        S: WorkspaceStore + 'static,
        C: CollabStore + 'static,
        A: AuthStore + 'static,
    {
        let owner = collab.owner_id();
        let workspace_id = workspace.settings.id.clone();
        Self {
            workspace,
            collab,
            persistence: Persistence {
                workspace_store: Box::new(store),
                collab_store: Box::new(collab_store),
                auth_store: Box::new(auth_store),
            },
            runtime: Runtime {
                connections: vec![Connection::owner(owner, workspace_id)],
                request_log: Vec::new(),
                plan: Plan::Free,
                block_count_override: None,
                block_count_baseline: 0,
                overloaded: false,
                simulate_write_timeout: false,
                rate_limiter: Box::new(RateLimiter::for_plan(Plan::Free, Duration::from_secs(60))),
                shared_rate_store: None,
            },
            webhooks: WebhookRuntime {
                allow_insecure: false,
                queue: Vec::new(),
                sender: Box::new(UreqWebhookSender::new()),
            },
            realtime: realtime::RealtimeHub::new(256),
            crdt: crdt::CrdtStore::new(),
        }
    }

    pub fn snapshot(&self) -> &Workspace {
        &self.workspace
    }

    /// Register an additional connection (used by tests and, later, by the
    /// self-hosted session layer).
    pub fn register_connection(&mut self, connection: Connection) {
        self.runtime.connections.push(connection);
    }

    /// Clone the real-time hub (the server keeps ownership).
    pub fn realtime(&self) -> realtime::RealtimeHub {
        self.realtime.clone()
    }

    pub fn connection_for_token(&self, token: &str) -> Option<Connection> {
        if let Some(connection) = self.runtime.connections.iter().find(|c| c.token == token) {
            return Some(connection.clone());
        }
        let token_hash = auth::hash_token(token);
        let session = self
            .persistence
            .auth_store
            .find_session(&token_hash)
            .ok()
            .flatten()?;
        if auth::session_is_expired(&session) {
            let _ = self.persistence.auth_store.delete_session(&token_hash);
            return None;
        }
        let member = self
            .collab
            .members
            .iter()
            .find(|member| member.id == session.member_id)?;
        Some(Connection::new(
            format!("session-{}", member.id),
            token,
            member.id.clone(),
            self.workspace.settings.id.clone(),
            Capability::all(),
        ))
    }

    /// In-memory request log for `/api/v2`; the same IDs are written to the
    /// HTTP response and (in a production adapter) to structured logs.
    pub fn request_log(&self) -> &[RequestLogEntry] {
        &self.runtime.request_log
    }

    /// Allow loopback HTTP webhook URLs for local development and tests.
    pub fn allow_insecure_webhooks(&mut self, allow: bool) {
        self.webhooks.allow_insecure = allow;
    }

    /// The workspace plan used for operational limits (not billing).
    pub fn plan(&self) -> Plan {
        self.runtime.plan
    }

    /// Change the plan and refresh the default rate budgets.
    pub fn set_plan(&mut self, plan: Plan) {
        let window = self.runtime.rate_limiter.window();
        self.runtime.plan = plan;
        self.runtime.rate_limiter = match &self.runtime.shared_rate_store {
            Some(store) => Box::new(SqliteRateLimiter::for_plan(store.clone(), plan, window)),
            None => Box::new(RateLimiter::for_plan(plan, window)),
        };
    }

    /// Override the derived block count (tests and operational tooling). Later
    /// writes still advance the count from this baseline.
    pub fn set_block_count(&mut self, blocks: u64) {
        self.runtime.block_count_baseline = self.workspace.block_count();
        self.runtime.block_count_override = Some(blocks);
    }

    /// The current workspace block count (pages count as one block each).
    pub fn block_count(&self) -> u64 {
        let actual = self.workspace.block_count();
        self.runtime
            .block_count_override
            .map(|base| {
                base.saturating_add(actual.saturating_sub(self.runtime.block_count_baseline))
            })
            .unwrap_or(actual)
    }

    /// The policy inputs for this request.
    pub fn request_budget(&self) -> RequestBudget {
        RequestBudget {
            plan: self.runtime.plan,
            members: self.collab.members.len(),
            blocks: self.block_count(),
        }
    }

    /// Replace the rate budgets (tests and deployment configuration).
    pub fn configure_rate_limits(
        &mut self,
        per_connection: u32,
        per_workspace: u32,
        window: Duration,
    ) {
        self.runtime.shared_rate_store = None;
        self.runtime.rate_limiter =
            Box::new(RateLimiter::new(per_connection, per_workspace, window));
    }

    /// Use a shared rate-limit store with the current plan defaults.
    pub fn use_shared_rate_limits<S>(&mut self, store: Arc<S>)
    where
        S: crate::store::RateLimitStore + 'static,
    {
        let window = self.runtime.rate_limiter.window();
        let store: Arc<dyn crate::store::RateLimitStore> = store;
        self.runtime.shared_rate_store = Some(store.clone());
        self.runtime.rate_limiter = Box::new(SqliteRateLimiter::for_plan(
            store,
            self.runtime.plan,
            window,
        ));
    }

    /// Use a shared rate-limit store with explicit budgets (tests/deployment).
    pub fn configure_shared_rate_limits<S>(
        &mut self,
        store: Arc<S>,
        per_connection: u32,
        per_workspace: u32,
        window: Duration,
    ) where
        S: crate::store::RateLimitStore + 'static,
    {
        let store: Arc<dyn crate::store::RateLimitStore> = store;
        self.runtime.shared_rate_store = Some(store.clone());
        self.runtime.rate_limiter = Box::new(SqliteRateLimiter::new(
            store,
            per_connection,
            per_workspace,
            window,
        ));
    }

    /// Inject a custom webhook sender (tests, alternate TLS adapters).
    pub fn set_webhook_sender(&mut self, sender: impl WebhookSender + 'static) {
        self.webhooks.sender = Box::new(sender);
    }

    /// Simulate a temporarily overloaded service (`529 service_overload`).
    pub fn set_overloaded(&mut self, overloaded: bool) {
        self.runtime.overloaded = overloaded;
    }

    /// Fault injection: make the next successful write report a timeout after
    /// the change was saved (`503 service_unavailable` with retry guidance).
    pub fn simulate_write_timeout(&mut self, enabled: bool) {
        self.runtime.simulate_write_timeout = enabled;
    }
}

impl WorkspaceServer {
    /// Read-only access to accounts, permissions and the sync log.
    pub fn collab(&self) -> &CollabState {
        &self.collab
    }

    /// Persist both documents.
    pub fn persist(&self) -> std::io::Result<()> {
        self.persistence.workspace_store.persist(&self.workspace)?;
        self.persistence.collab_store.persist(&self.collab)
    }

    /// Persist only the collaboration document.
    pub fn persist_collab(&self) -> std::io::Result<()> {
        self.persistence.collab_store.persist(&self.collab)
    }

    /// Single write path (DRY): bump the version, append one sync-log entry,
    /// and persist both documents. Every mutation goes through here and reports
    /// a storage failure to the caller instead of hiding it (spec-015).
    fn write(&mut self, op: ChangeOp, page_id: &str) -> std::io::Result<()> {
        self.workspace.bump_version();
        let actor = self.collab.owner_id();
        let change = self.collab.record(op, page_id, &actor);
        self.persist()?;
        // A REST write is the structure source of truth; re-seed the CRDT text
        // from the new snapshot instead of keeping a divergent document.
        if page_id == "*" {
            self.crdt.reset_all();
        } else {
            self.crdt.reset_page(page_id);
        }
        self.realtime.publish_change(change);
        Ok(())
    }

    /// Sync one page's CRDT text after an optional state vector.
    pub fn crdt_sync(
        &mut self,
        page_id: &str,
        state_vector: Option<&[u8]>,
    ) -> Result<(Vec<u8>, Vec<u8>), String> {
        self.crdt.ensure_page(&self.workspace, page_id)?;
        self.crdt.sync(&self.workspace, page_id, state_vector)
    }

    /// Apply a CRDT update, project the merged text and persist the snapshot.
    pub fn apply_crdt_update(
        &mut self,
        page_id: &str,
        update: &[u8],
        actor: &str,
    ) -> Result<Vec<u8>, String> {
        self.crdt.ensure_page(&self.workspace, page_id)?;
        let merged = self.crdt.apply_update(&self.workspace, page_id, update)?;
        self.crdt.project(&mut self.workspace, page_id)?;
        self.workspace.bump_version();
        self.collab.record(ChangeOp::Update, page_id, actor);
        self.persist().map_err(|error| error.to_string())?;
        Ok(merged)
    }
}
