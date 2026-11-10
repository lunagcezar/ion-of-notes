//! Server side of the application.
//!
//! `WorkspaceServer` owns all state and answers `ApiRequest`s through the
//! `WorkspaceApi` seam. `serve_once` / `serve_forever` expose the very same
//! contract over loopback HTTP/1.1 for the optional `--serve` mode.

use crate::auth::{
    self, AuthStore, LoginInput, NoAuthStore, Session, SessionInfo, SessionResponse,
};
use crate::collab::{
    ChangeOp, CollabState, Member, MemberDraft, Share, export_tree, realtime_contract,
};
use crate::model::{
    AttachmentInput, Block, BlockChildrenRequest, BlockDto, BlockInput, BlockKind,
    CommentAttachment, CommentDisplayName, CommentDisplayNameDto, CommentDto, CommentInput,
    DataSource, DataSourceInput, DataSourceRef, Database, DatabaseInput, FileObject, FileUpload,
    FileUploadInput, Page, PageDraft, PagePropertiesInput, PageSummary, ParentDto, Person,
    RichText, SearchResult, UploadStatus, UserDto, UserRef, UserType, View, ViewConfig, ViewInput,
    ViewQueryInput, ViewQueryResult, Workspace, now_iso, now_label,
};
use crate::store::{
    CollabStore, JsonCollabStore, JsonStore, NoCollabStore, NoStore, WorkspaceStore,
};
use crate::webhooks::{
    EntityRef, SubscriptionStatus, WebhookDelivery, WebhookEvent, WebhookEventType,
    WebhookSubscription, WebhookSubscriptionInput, WebhookVerificationInput, aggregate_events,
    is_allowed_url, sign,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

mod contract;
mod http;
pub mod limits;
mod pagination;
mod rate_limit;
pub mod realtime;
mod url;
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
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use url::url_decode;
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
}

/// Owns all workspace state. Persistence is injected through the
/// `WorkspaceStore` and `CollabStore` ports, so the core never depends on a
/// concrete backend. Collaboration state (accounts, permissions, sync log,
/// versions) is kept beside the workspace and exposed through the decided API.
pub struct WorkspaceServer {
    pub workspace: Workspace,
    pub collab: CollabState,
    store: Box<dyn WorkspaceStore>,
    collab_store: Box<dyn CollabStore>,
    auth_store: Box<dyn AuthStore>,
    connections: Vec<Connection>,
    request_log: Vec<RequestLogEntry>,
    allow_insecure_webhooks: bool,
    webhook_queue: Vec<crate::webhooks::WebhookEvent>,
    plan: Plan,
    block_count_override: Option<u64>,
    block_count_baseline: u64,
    overloaded: bool,
    simulate_write_timeout: bool,
    rate_limiter: Box<dyn RateLimitPolicy>,
    shared_rate_store: Option<Arc<dyn crate::store::RateLimitStore>>,
    realtime: realtime::RealtimeHub,
    webhook_sender: Box<dyn WebhookSender>,
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
            store: Box::new(store),
            collab_store: Box::new(collab_store),
            auth_store: Box::new(auth_store),
            connections: vec![Connection::owner(owner, workspace_id)],
            request_log: Vec::new(),
            allow_insecure_webhooks: false,
            webhook_queue: Vec::new(),
            plan: Plan::Free,
            block_count_override: None,
            block_count_baseline: 0,
            overloaded: false,
            simulate_write_timeout: false,
            rate_limiter: Box::new(RateLimiter::for_plan(Plan::Free, Duration::from_secs(60))),
            shared_rate_store: None,
            realtime: realtime::RealtimeHub::new(256),
            webhook_sender: Box::new(UreqWebhookSender::new()),
        }
    }

    pub fn snapshot(&self) -> &Workspace {
        &self.workspace
    }

    /// Register an additional connection (used by tests and, later, by the
    /// self-hosted session layer).
    pub fn register_connection(&mut self, connection: Connection) {
        self.connections.push(connection);
    }

    /// Clone the real-time hub (the server keeps ownership).
    pub fn realtime(&self) -> realtime::RealtimeHub {
        self.realtime.clone()
    }

    pub fn connection_for_token(&self, token: &str) -> Option<Connection> {
        if let Some(connection) = self.connections.iter().find(|c| c.token == token) {
            return Some(connection.clone());
        }
        let token_hash = auth::hash_token(token);
        let session = self.auth_store.find_session(&token_hash).ok().flatten()?;
        if auth::session_is_expired(&session) {
            let _ = self.auth_store.delete_session(&token_hash);
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
        &self.request_log
    }

    /// Allow loopback HTTP webhook URLs for local development and tests.
    pub fn allow_insecure_webhooks(&mut self, allow: bool) {
        self.allow_insecure_webhooks = allow;
    }

    /// The workspace plan used for operational limits (not billing).
    pub fn plan(&self) -> Plan {
        self.plan
    }

    /// Change the plan and refresh the default rate budgets.
    pub fn set_plan(&mut self, plan: Plan) {
        let window = self.rate_limiter.window();
        self.plan = plan;
        self.rate_limiter = match &self.shared_rate_store {
            Some(store) => Box::new(SqliteRateLimiter::for_plan(store.clone(), plan, window)),
            None => Box::new(RateLimiter::for_plan(plan, window)),
        };
    }

    /// Override the derived block count (tests and operational tooling). Later
    /// writes still advance the count from this baseline.
    pub fn set_block_count(&mut self, blocks: u64) {
        self.block_count_baseline = self.workspace.block_count();
        self.block_count_override = Some(blocks);
    }

    /// The current workspace block count (pages count as one block each).
    pub fn block_count(&self) -> u64 {
        let actual = self.workspace.block_count();
        self.block_count_override
            .map(|base| base.saturating_add(actual.saturating_sub(self.block_count_baseline)))
            .unwrap_or(actual)
    }

    /// The policy inputs for this request.
    pub fn request_budget(&self) -> RequestBudget {
        RequestBudget {
            plan: self.plan,
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
        self.shared_rate_store = None;
        self.rate_limiter = Box::new(RateLimiter::new(per_connection, per_workspace, window));
    }

    /// Use a shared rate-limit store with the current plan defaults.
    pub fn use_shared_rate_limits<S>(&mut self, store: Arc<S>)
    where
        S: crate::store::RateLimitStore + 'static,
    {
        let window = self.rate_limiter.window();
        let store: Arc<dyn crate::store::RateLimitStore> = store;
        self.shared_rate_store = Some(store.clone());
        self.rate_limiter = Box::new(SqliteRateLimiter::for_plan(store, self.plan, window));
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
        self.shared_rate_store = Some(store.clone());
        self.rate_limiter = Box::new(SqliteRateLimiter::new(
            store,
            per_connection,
            per_workspace,
            window,
        ));
    }

    /// Inject a custom webhook sender (tests, alternate TLS adapters).
    pub fn set_webhook_sender(&mut self, sender: impl WebhookSender + 'static) {
        self.webhook_sender = Box::new(sender);
    }

    /// Simulate a temporarily overloaded service (`529 service_overload`).
    pub fn set_overloaded(&mut self, overloaded: bool) {
        self.overloaded = overloaded;
    }

    /// Fault injection: make the next successful write report a timeout after
    /// the change was saved (`503 service_unavailable` with retry guidance).
    pub fn simulate_write_timeout(&mut self, enabled: bool) {
        self.simulate_write_timeout = enabled;
    }

    /// `POST /api/v2/auth/login`: verify an account and create a session.
    fn login(&mut self, request: &ApiRequest, request_id: &str) -> ApiResponse {
        let input = match parse_body::<LoginInput>(request) {
            Ok(input) => input,
            Err(message) => {
                return v2_error(400, CODE_INVALID_JSON, message, request_id, None);
            }
        };
        if input.email.trim().is_empty() || input.password.is_empty() {
            return v2_error(
                400,
                CODE_VALIDATION_ERROR,
                "email and password are required",
                request_id,
                None,
            );
        }
        let email = input.email.trim().to_lowercase();
        let _ = self.auth_store.prune_sessions(&now_iso());
        let account = match self.auth_store.find_account(&email) {
            Ok(Some(account)) => account,
            Ok(None) => {
                return v2_error(
                    401,
                    CODE_UNAUTHORIZED,
                    "invalid email or password",
                    request_id,
                    None,
                );
            }
            Err(error) => {
                return v2_error(500, CODE_INTERNAL, error.to_string(), request_id, None);
            }
        };
        if !auth::verify_password(&account.password_hash, &input.password) {
            return v2_error(
                401,
                CODE_UNAUTHORIZED,
                "invalid email or password",
                request_id,
                None,
            );
        }
        let Some(member) = self
            .collab
            .members
            .iter()
            .find(|member| member.id == account.member_id)
            .cloned()
        else {
            return v2_error(
                401,
                CODE_UNAUTHORIZED,
                "account has no workspace member",
                request_id,
                None,
            );
        };
        let token = auth::new_session_token();
        let session = Session {
            token_hash: auth::hash_token(&token),
            member_id: member.id.clone(),
            created_at: now_iso(),
            expires_at: auth::session_expiry(),
        };
        if let Err(error) = self.auth_store.save_session(&session) {
            return v2_error(500, CODE_INTERNAL, error.to_string(), request_id, None);
        }
        self.connections.push(Connection::new(
            format!("session-{}", member.id),
            token.clone(),
            member.id.clone(),
            self.workspace.settings.id.clone(),
            Capability::all(),
        ));
        json(
            200,
            Envelope::ok(SessionResponse {
                token,
                expires_at: session.expires_at,
                member_id: member.id,
                email: member.email,
                name: member.name,
            }),
        )
    }

    /// Session routes that need an already-resolved connection.
    fn handle_v2_auth(
        &mut self,
        request: &ApiRequest,
        path: &str,
        request_id: &str,
        connection: &Connection,
    ) -> Option<ApiResponse> {
        if path == "/api/v2/auth/logout" && request.method == Method::Post {
            let token_hash = auth::hash_token(&connection.token);
            if let Err(error) = self.auth_store.delete_session(&token_hash) {
                return Some(v2_error(
                    500,
                    CODE_INTERNAL,
                    error.to_string(),
                    request_id,
                    None,
                ));
            }
            // The demo owner token is not a persisted session; keep it usable.
            if connection.id.starts_with("session-") {
                self.connections
                    .retain(|existing| existing.token != connection.token);
            }
            return Some(json(200, Envelope::ok("logged_out".to_string())));
        }
        if path == "/api/v2/auth/session" && request.method == Method::Get {
            let token_hash = auth::hash_token(&connection.token);
            let session = match self.auth_store.find_session(&token_hash) {
                Ok(Some(session)) => session,
                Ok(None) => {
                    return Some(v2_error(
                        401,
                        CODE_UNAUTHORIZED,
                        "session not found",
                        request_id,
                        None,
                    ));
                }
                Err(error) => {
                    return Some(v2_error(
                        500,
                        CODE_INTERNAL,
                        error.to_string(),
                        request_id,
                        None,
                    ));
                }
            };
            if auth::session_is_expired(&session) {
                let _ = self.auth_store.delete_session(&token_hash);
                return Some(v2_error(
                    401,
                    CODE_UNAUTHORIZED,
                    "session expired",
                    request_id,
                    None,
                ));
            }
            let Some(member) = self
                .collab
                .members
                .iter()
                .find(|member| member.id == session.member_id)
            else {
                return Some(v2_error(
                    401,
                    CODE_UNAUTHORIZED,
                    "session member not found",
                    request_id,
                    None,
                ));
            };
            return Some(json(
                200,
                Envelope::ok(SessionInfo {
                    member_id: member.id.clone(),
                    email: member.email.clone(),
                    name: member.name.clone(),
                    expires_at: session.expires_at,
                }),
            ));
        }
        None
    }

    /// Reject a mutating request whose `If-Match` version is stale.
    fn conflict_response(&self, request: &ApiRequest, request_id: &str) -> Option<ApiResponse> {
        if matches!(request.method, Method::Get) {
            return None;
        }
        let expected = request
            .header("if-match")?
            .trim()
            .trim_matches('"')
            .parse::<u64>()
            .ok()?;
        if expected == self.workspace.version {
            return None;
        }
        Some(v2_error(
            409,
            CODE_CONFLICT,
            "the workspace changed since the request was prepared",
            request_id,
            Some(serde_json::json!({ "current_version": self.workspace.version })),
        ))
    }

    /// Reject a write that would create blocks beyond the Free workspace limit.
    fn block_limit_response(
        &self,
        request: &ApiRequest,
        path: &str,
        request_id: &str,
    ) -> Option<ApiResponse> {
        let additional = self.blocks_created_by(request, path)?;
        if !self.request_budget().block_limit_reached(additional) {
            return None;
        }
        Some(v2_error(
            403,
            CODE_RESTRICTED_RESOURCE,
            "This workspace has used all of its free blocks. Upgrade its plan to create more content.",
            request_id,
            Some(serde_json::json!({ "block_limit": "block_creation" })),
        ))
    }

    /// How many new blocks a request would add, or `None` when it cannot
    /// create blocks. The page itself counts as one block.
    fn blocks_created_by(&self, request: &ApiRequest, path: &str) -> Option<u64> {
        match (request.method, path) {
            (Method::Post, "/api/v2/pages") | (Method::Post, "/api/v2/databases") => Some(1),
            (Method::Patch, p) if p.starts_with("/api/v2/blocks/") && p.ends_with("/children") => {
                parse_body::<BlockChildrenRequest>(request)
                    .ok()
                    .map(|children| children.children.len() as u64)
            }
            (Method::Patch, p) => {
                let rest = p.strip_prefix("/api/v2/pages/")?;
                if rest.is_empty() || rest.contains('/') {
                    return None;
                }
                let page = parse_body::<Page>(request).ok()?;
                let previous = self
                    .workspace
                    .page(&page.id)
                    .map(Page::block_count)
                    .unwrap_or(0);
                Some(page.block_count().saturating_sub(previous))
            }
            _ => None,
        }
    }

    /// Read-only access to accounts, permissions and the sync log.
    pub fn collab(&self) -> &CollabState {
        &self.collab
    }

    /// Persist both documents.
    pub fn persist(&self) -> std::io::Result<()> {
        self.store.persist(&self.workspace)?;
        self.collab_store.persist(&self.collab)
    }

    /// Persist only the collaboration document.
    pub fn persist_collab(&self) -> std::io::Result<()> {
        self.collab_store.persist(&self.collab)
    }

    /// Single write path (DRY): bump the version, append one sync-log entry,
    /// and persist both documents. Every mutation goes through here and reports
    /// a storage failure to the caller instead of hiding it (spec-015).
    fn write(&mut self, op: ChangeOp, page_id: &str) -> std::io::Result<()> {
        self.workspace.bump_version();
        let actor = self.collab.owner_id();
        let change = self.collab.record(op, page_id, &actor);
        self.persist()?;
        self.realtime.publish_change(change);
        Ok(())
    }
}

fn json<T: Serialize>(status: u16, envelope: Envelope<T>) -> ApiResponse {
    ApiResponse {
        status,
        body: serde_json::to_string(&envelope)
            .unwrap_or_else(|_| r#"{"data":null,"error":"serialize"}"#.to_string()),
        request_id: None,
    }
}

fn bad(message: &str) -> ApiResponse {
    json::<()>(400, Envelope::err(message))
}

fn not_found(message: &str) -> ApiResponse {
    json::<()>(404, Envelope::err(message))
}

fn server_error(message: &str) -> ApiResponse {
    json::<()>(500, Envelope::err(message))
}

fn parse_body<T: DeserializeOwned>(request: &ApiRequest) -> Result<T, String> {
    let body = request
        .body
        .as_deref()
        .ok_or_else(|| "missing request body".to_string())?;
    serde_json::from_str(body).map_err(|e| format!("invalid JSON: {e}"))
}

fn query_value(query: &str, key: &str) -> String {
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=')
            && k == key
        {
            return url_decode(v);
        }
    }
    String::new()
}

fn v2_error(
    status: u16,
    code: &str,
    message: impl Into<String>,
    request_id: &str,
    additional_data: Option<serde_json::Value>,
) -> ApiResponse {
    let mut error = ApiError::new(status, code, message, request_id);
    if let Some(data) = additional_data {
        error = error.with_data(data);
    }
    ApiResponse {
        status,
        body: serde_json::to_string(&error).unwrap_or_else(|_| {
            r#"{"object":"error","status":500,"code":"internal_server_error"}"#.to_string()
        }),
        request_id: None,
    }
}

/// A saved write whose response could not be built: tell the client to read
/// the object instead of repeating the write (spec-037).
fn write_timeout_response(response: &ApiResponse, request_id: &str) -> ApiResponse {
    let committed = serde_json::from_str::<serde_json::Value>(&response.body)
        .ok()
        .and_then(|body| body.get("data")?.get("id")?.as_str().map(str::to_string));
    let mut data = serde_json::json!({
        "retry_after": "1",
        "retry_guidance": [
            "Read the object again to confirm the saved change.",
            "Do not repeat the write."
        ]
    });
    if let Some(id) = committed {
        data["committed_resource_id"] = serde_json::json!(id);
    }
    v2_error(
        503,
        CODE_SERVICE_UNAVAILABLE,
        "The change was saved, but the response could not be built in time. Read the object again instead of repeating the write.",
        request_id,
        Some(data),
    )
}

fn map_v1_error(v1: ApiResponse, request_id: &str) -> ApiResponse {
    let message = serde_json::from_str::<Envelope<serde_json::Value>>(&v1.body)
        .ok()
        .and_then(|envelope| envelope.error)
        .unwrap_or_else(|| v1.body.clone());
    let code = match v1.status {
        404 => CODE_NOT_FOUND,
        500 => CODE_INTERNAL,
        _ if message.contains("invalid JSON") || message.contains("missing request body") => {
            CODE_INVALID_JSON
        }
        _ => CODE_VALIDATION_ERROR,
    };
    v2_error(v1.status, code, message, request_id, None)
}

fn required_capability(method: Method, path: &str) -> Option<Capability> {
    match (method, path) {
        (Method::Get, "/api/v2/workspace") => Some(Capability::ReadContent),
        (Method::Put, "/api/v2/workspace") => Some(Capability::UpdateContent),
        (Method::Get, p) if p.starts_with("/api/v2/blocks/") => Some(Capability::ReadContent),
        (Method::Patch | Method::Delete, p) if p.starts_with("/api/v2/blocks/") => {
            Some(Capability::UpdateContent)
        }
        (Method::Get, "/api/v2/databases") => Some(Capability::ReadContent),
        (Method::Post, "/api/v2/databases") => Some(Capability::InsertContent),
        (Method::Get, p) if p.starts_with("/api/v2/databases/") => Some(Capability::ReadContent),
        (Method::Patch | Method::Delete, p) if p.starts_with("/api/v2/databases/") => {
            Some(Capability::UpdateContent)
        }
        (Method::Get, "/api/v2/auth/session") => Some(Capability::ReadContent),
        (Method::Post, "/api/v2/auth/logout") => Some(Capability::ReadContent),
        (Method::Get, "/api/v2/webhooks") => Some(Capability::ReadContent),
        (Method::Post, "/api/v2/webhooks") => Some(Capability::InsertContent),
        (Method::Get, "/api/v2/webhook_deliveries") => Some(Capability::ReadContent),
        (Method::Post, "/api/v2/webhook_deliveries/retry") => Some(Capability::InsertContent),
        (Method::Delete, p) if p.starts_with("/api/v2/webhooks/") => {
            Some(Capability::UpdateContent)
        }
        (Method::Post, p) if p.starts_with("/api/v2/webhooks/") => Some(Capability::InsertContent),
        (Method::Get, "/api/v2/comments") => Some(Capability::ReadComments),
        (Method::Post, "/api/v2/comments") => Some(Capability::InsertComments),
        (Method::Get, p) if p.starts_with("/api/v2/comments/") => Some(Capability::ReadComments),
        (Method::Patch | Method::Delete, p) if p.starts_with("/api/v2/comments/") => {
            Some(Capability::InsertComments)
        }
        (Method::Get, p) if p.starts_with("/api/v2/users") => Some(Capability::ReadContent),
        (Method::Get, "/api/v2/file_uploads") => Some(Capability::ReadContent),
        (Method::Post, "/api/v2/file_uploads") => Some(Capability::InsertContent),
        (Method::Get, p) if p.starts_with("/api/v2/file_uploads/") => Some(Capability::ReadContent),
        (Method::Post, p) if p.starts_with("/api/v2/file_uploads/") => {
            Some(Capability::InsertContent)
        }
        (Method::Get, "/api/v2/views") => Some(Capability::ReadContent),
        (Method::Post, "/api/v2/views") => Some(Capability::InsertContent),
        (Method::Get, p) if p.starts_with("/api/v2/views/") => Some(Capability::ReadContent),
        (Method::Patch | Method::Delete, p) if p.starts_with("/api/v2/views/") => {
            Some(Capability::UpdateContent)
        }
        (Method::Post, p) if p.starts_with("/api/v2/views/") => Some(Capability::ReadContent),
        (Method::Get, p) if p.starts_with("/api/v2/data_sources/") => Some(Capability::ReadContent),
        (Method::Patch, p) if p.starts_with("/api/v2/data_sources/") => {
            Some(Capability::UpdateContent)
        }
        (Method::Post, p) if p.ends_with("/query") && p.starts_with("/api/v2/data_sources/") => {
            Some(Capability::ReadContent)
        }
        (Method::Post, p)
            if p.ends_with("/data_sources") && p.starts_with("/api/v2/databases/") =>
        {
            Some(Capability::InsertContent)
        }
        (Method::Get, "/api/v2/pages") => Some(Capability::ReadContent),
        (Method::Post, "/api/v2/pages") => Some(Capability::InsertContent),
        (Method::Get, p) if p.starts_with("/api/v2/pages/") => Some(Capability::ReadContent),
        (Method::Patch | Method::Delete, p) if p.starts_with("/api/v2/pages/") => {
            Some(Capability::UpdateContent)
        }
        (Method::Get, "/api/v2/search") => Some(Capability::ReadContent),
        (Method::Get, "/api/v2/members") => Some(Capability::ReadContent),
        (Method::Post, "/api/v2/members") => Some(Capability::InsertContent),
        (Method::Get, "/api/v2/shares") => Some(Capability::ReadContent),
        (Method::Put, "/api/v2/shares") => Some(Capability::UpdateContent),
        (Method::Get, "/api/v2/sync") => Some(Capability::ReadContent),
        (Method::Get, "/api/v2/export") => Some(Capability::ReadContent),
        (Method::Get, "/api/v2/realtime") => Some(Capability::ReadContent),
        _ => None,
    }
}

impl WorkspaceServer {
    /// Dispatch a v2 request: version, auth, capability, then the shared v1
    /// use case. The v2 layer changes the envelope/error shape and adds a
    /// request id; it does not duplicate the business logic.
    fn handle_v2(&mut self, request: &ApiRequest) -> ApiResponse {
        let request_id = new_request_id();
        let (path, query) = match request.path.split_once('?') {
            Some((p, q)) => (p, q),
            None => (request.path.as_str(), ""),
        };
        let is_write = !matches!(request.method, Method::Get);
        let timed_out = is_write && self.simulate_write_timeout;
        if timed_out {
            self.simulate_write_timeout = false;
        }
        let response = self.handle_v2_inner(request, path, query, &request_id);
        let response = if timed_out && (200..300).contains(&response.status) {
            write_timeout_response(&response, &request_id)
        } else {
            response
        };
        let status = response.status;
        let response = ApiResponse {
            request_id: Some(request_id.clone()),
            ..response
        };
        self.request_log.push(RequestLogEntry {
            request_id,
            method: request.method.as_str().to_string(),
            path: request.path.clone(),
            status,
        });
        response
    }

    fn handle_v2_inner(
        &mut self,
        request: &ApiRequest,
        path: &str,
        query: &str,
        request_id: &str,
    ) -> ApiResponse {
        let Some(version) = request.header(VERSION_HEADER) else {
            return v2_error(
                400,
                CODE_MISSING_VERSION,
                format!("{VERSION_HEADER} header is required"),
                request_id,
                None,
            );
        };
        if !version_supported(version) {
            return v2_error(
                400,
                CODE_UNSUPPORTED_VERSION,
                format!("unsupported version: {version}"),
                request_id,
                Some(serde_json::json!({ "supported_versions": SUPPORTED_VERSIONS })),
            );
        }

        if request.method == Method::Post && path == "/api/v2/auth/login" {
            let decision =
                self.rate_limiter
                    .check("auth-login", &self.workspace.settings.id, Instant::now());
            if !decision.allowed {
                let reason = decision.reason.unwrap_or(REASON_CONNECTION);
                return v2_error(
                    429,
                    CODE_RATE_LIMITED,
                    format!("rate limit exceeded: {reason}"),
                    request_id,
                    Some(serde_json::json!({
                        "rate_limit_reason": reason,
                        "retry_after": decision.retry_after.to_string(),
                    })),
                );
            }
            return self.login(request, request_id);
        }

        let Some(token) = request.header(AUTH_HEADER).and_then(parse_bearer) else {
            return v2_error(
                401,
                CODE_UNAUTHORIZED,
                "missing or malformed bearer token",
                request_id,
                None,
            );
        };
        let Some(connection) = self.connection_for_token(token) else {
            return v2_error(
                401,
                CODE_UNAUTHORIZED,
                "invalid bearer token",
                request_id,
                None,
            );
        };

        if self.overloaded {
            return v2_error(
                529,
                CODE_SERVICE_OVERLOAD,
                "the service is temporarily overloaded; retry after the wait",
                request_id,
                Some(serde_json::json!({ "retry_after": "5" })),
            );
        }
        let decision =
            self.rate_limiter
                .check(&connection.id, &connection.workspace_id, Instant::now());
        if !decision.allowed {
            let reason = decision.reason.unwrap_or(REASON_CONNECTION);
            return v2_error(
                429,
                CODE_RATE_LIMITED,
                format!("rate limit exceeded: {reason}"),
                request_id,
                Some(serde_json::json!({
                    "rate_limit_reason": reason,
                    "retry_after": decision.retry_after.to_string(),
                })),
            );
        }
        if !matches!(request.method, Method::Get)
            && let Err(violation) = limits::validate_request(request.body.as_deref())
        {
            return v2_error(
                400,
                CODE_VALIDATION_ERROR,
                violation.message,
                request_id,
                Some(serde_json::json!({ "limit": violation.limit })),
            );
        }
        if let Some(response) = self.conflict_response(request, request_id) {
            return response;
        }
        if let Some(response) = self.block_limit_response(request, path, request_id) {
            return response;
        }

        let Some(required) = required_capability(request.method, path) else {
            return v2_error(404, CODE_NOT_FOUND, "unknown route", request_id, None);
        };
        if !connection.has(required) {
            return v2_error(
                403,
                CODE_RESTRICTED_RESOURCE,
                format!("missing capability: {}", required.as_str()),
                request_id,
                Some(serde_json::json!({ "required_capability": required.as_str() })),
            );
        }

        if let Some(response) = self.handle_v2_auth(request, path, request_id, &connection) {
            return response;
        }

        self.workspace.ensure_data_sources();
        self.workspace.ensure_views();
        if let Some(response) = self.handle_v2_webhook(request, path, request_id) {
            return response;
        }
        if let Some(response) =
            self.handle_v2_comment_user_file(request, path, query, request_id, &connection)
        {
            return response;
        }
        if let Some(response) = self.handle_v2_view(request, path, query, request_id) {
            return response;
        }
        if let Some(response) = self.handle_v2_database(request, path, query, request_id) {
            return response;
        }
        if let Some(response) = self.handle_v2_block(request, path, query, request_id) {
            return response;
        }
        if let Some(response) = self.handle_v2_list(request, path, query, request_id) {
            return response;
        }

        let v1_request = ApiRequest {
            method: request.method,
            path: request.path.replacen("/api/v2", "/api", 1),
            body: request.body.clone(),
            headers: BTreeMap::new(),
        };
        let v1 = self.handle_v1(&v1_request);
        if v1.status >= 400 {
            return map_v1_error(v1, request_id);
        }
        ApiResponse {
            status: v1.status,
            body: v1.body,
            request_id: None,
        }
    }

    pub fn list_webhooks(&self) -> &[WebhookSubscription] {
        &self.collab.webhooks
    }

    pub fn create_webhook(
        &mut self,
        input: WebhookSubscriptionInput,
    ) -> Result<WebhookSubscription, String> {
        if input.url.trim().is_empty() || input.events.is_empty() {
            return Err("url and at least one event are required".to_string());
        }
        if !is_allowed_url(&input.url, self.allow_insecure_webhooks) {
            return Err(
                "webhook URL must be HTTPS (loopback HTTP requires the development override)"
                    .to_string(),
            );
        }
        let subscription = WebhookSubscription {
            id: format!("wh-{}", uuid::Uuid::new_v4()),
            url: input.url,
            events: input.events,
            verification_token: format!("secret_{}", uuid::Uuid::new_v4().simple()),
            status: SubscriptionStatus::Pending,
            created_at: now_iso(),
        };
        self.collab.webhooks.push(subscription.clone());
        self.persist_collab().map_err(|error| error.to_string())?;
        Ok(subscription)
    }

    pub fn verify_webhook(&mut self, id: &str, token: &str) -> Result<(), String> {
        let Some(subscription) = self
            .collab
            .webhooks
            .iter_mut()
            .find(|subscription| subscription.id == id)
        else {
            return Err("webhook not found".to_string());
        };
        if subscription.verification_token != token {
            return Err("verification token does not match".to_string());
        }
        subscription.status = SubscriptionStatus::Active;
        self.persist_collab().map_err(|error| error.to_string())
    }

    pub fn delete_webhook(&mut self, id: &str) -> Result<(), String> {
        let before = self.collab.webhooks.len();
        self.collab
            .webhooks
            .retain(|subscription| subscription.id != id);
        if self.collab.webhooks.len() == before {
            return Err("webhook not found".to_string());
        }
        self.persist_collab().map_err(|error| error.to_string())
    }

    pub fn webhook_deliveries(&self) -> &[WebhookDelivery] {
        &self.collab.webhook_deliveries
    }

    /// Queue or deliver an event to active subscriptions.
    pub fn emit_webhook(
        &mut self,
        event_type: WebhookEventType,
        entity_type: &str,
        entity_id: &str,
        data: impl Into<String>,
    ) {
        let data = data.into();
        let actor = self.collab.owner_id();
        let workspace_id = self.workspace.settings.id.clone();
        let subscriptions: Vec<WebhookSubscription> = self
            .collab
            .webhooks
            .iter()
            .filter(|subscription| {
                subscription.status == SubscriptionStatus::Active
                    && subscription.events.contains(&event_type)
            })
            .cloned()
            .collect();
        for subscription in subscriptions {
            let event = WebhookEvent::new(
                workspace_id.clone(),
                subscription.id.clone(),
                event_type,
                EntityRef {
                    id: entity_id.to_string(),
                    r#type: entity_type.to_string(),
                },
                actor.clone(),
                data.clone(),
            );
            if event_type.is_aggregated() {
                if let Some(existing) = self.webhook_queue.iter_mut().find(|existing| {
                    existing.subscription_id == event.subscription_id
                        && existing.entity.id == event.entity.id
                        && existing.r#type == event.r#type
                }) {
                    *existing = event;
                } else {
                    self.webhook_queue.push(event);
                }
            } else {
                self.deliver_event(&subscription, event);
            }
        }
    }

    /// Flush aggregated events (the production worker calls this on a timer).
    pub fn flush_webhooks(&mut self) {
        let queued = std::mem::take(&mut self.webhook_queue);
        for event in aggregate_events(queued) {
            if let Some(subscription) = self
                .collab
                .webhooks
                .iter()
                .find(|subscription| subscription.id == event.subscription_id)
                .cloned()
            {
                self.deliver_event(&subscription, event);
            }
        }
    }

    /// Retry failed deliveries with a small exponential backoff.
    pub fn process_webhook_deliveries(&mut self) {
        let subscriptions: Vec<WebhookSubscription> = self.collab.webhooks.clone();
        for delivery in &mut self.collab.webhook_deliveries {
            if delivery.delivered || delivery.attempts >= 8 {
                continue;
            }
            let Some(subscription) = subscriptions
                .iter()
                .find(|subscription| subscription.id == delivery.subscription_id)
            else {
                continue;
            };
            let signature = sign(&subscription.verification_token, &delivery.payload);
            let result = self
                .webhook_sender
                .send(&subscription.url, &delivery.payload, &signature);
            delivery.attempts = delivery.attempts.saturating_add(1);
            match result {
                Ok(status) => {
                    delivery.last_status = Some(status);
                    delivery.last_error = None;
                    delivery.delivered = (200..300).contains(&status);
                    delivery.next_attempt_at = if delivery.delivered {
                        None
                    } else {
                        Some(backoff_time(delivery.attempts))
                    };
                }
                Err(error) => {
                    delivery.last_status = None;
                    delivery.last_error = Some(error);
                    delivery.next_attempt_at = Some(backoff_time(delivery.attempts));
                }
            }
        }
        let _ = self.persist_collab();
    }

    fn deliver_event(&mut self, subscription: &WebhookSubscription, mut event: WebhookEvent) {
        event.attempt_number = 1;
        let payload = serde_json::to_string(&event).unwrap_or_default();
        let signature = sign(&subscription.verification_token, &payload);
        let result = self
            .webhook_sender
            .send(&subscription.url, &payload, &signature);
        let delivered = result
            .as_ref()
            .is_ok_and(|status| (200..300).contains(status));
        self.collab.webhook_deliveries.push(WebhookDelivery {
            id: format!("delivery-{}", uuid::Uuid::new_v4()),
            subscription_id: subscription.id.clone(),
            event_id: event.id.clone(),
            payload,
            attempts: 1,
            last_status: result.as_ref().ok().copied(),
            last_error: result.err(),
            delivered,
            next_attempt_at: if delivered {
                None
            } else {
                Some(backoff_time(1))
            },
        });
        let _ = self.persist_collab();
    }

    fn handle_v2_webhook(
        &mut self,
        request: &ApiRequest,
        path: &str,
        request_id: &str,
    ) -> Option<ApiResponse> {
        if path == "/api/v2/webhooks" && request.method == Method::Get {
            return Some(json(200, Envelope::ok(self.collab.webhooks.clone())));
        }
        if path == "/api/v2/webhooks" && request.method == Method::Post {
            let input = match parse_body::<WebhookSubscriptionInput>(request) {
                Ok(input) => input,
                Err(message) => {
                    return Some(v2_error(400, CODE_INVALID_JSON, message, request_id, None));
                }
            };
            return match self.create_webhook(input) {
                Ok(subscription) => Some(json(201, Envelope::ok(subscription))),
                Err(message) => Some(v2_error(
                    400,
                    CODE_VALIDATION_ERROR,
                    message,
                    request_id,
                    None,
                )),
            };
        }
        if path == "/api/v2/webhook_deliveries" && request.method == Method::Get {
            return Some(json(
                200,
                Envelope::ok(self.collab.webhook_deliveries.clone()),
            ));
        }
        if path == "/api/v2/webhook_deliveries/retry" && request.method == Method::Post {
            self.process_webhook_deliveries();
            return Some(json(
                200,
                Envelope::ok(self.collab.webhook_deliveries.clone()),
            ));
        }
        let rest = path.strip_prefix("/api/v2/webhooks/")?;
        if let Some(id) = rest.strip_suffix("/verify") {
            if request.method != Method::Post || id.is_empty() {
                return Some(v2_error(
                    404,
                    CODE_NOT_FOUND,
                    "webhook not found",
                    request_id,
                    None,
                ));
            }
            let input = match parse_body::<WebhookVerificationInput>(request) {
                Ok(input) => input,
                Err(message) => {
                    return Some(v2_error(400, CODE_INVALID_JSON, message, request_id, None));
                }
            };
            return match self.verify_webhook(id, &input.verification_token) {
                Ok(()) => Some(json(200, Envelope::ok("verified".to_string()))),
                Err(message) => Some(v2_error(
                    400,
                    CODE_VALIDATION_ERROR,
                    message,
                    request_id,
                    None,
                )),
            };
        }
        if request.method == Method::Delete && !rest.is_empty() && !rest.contains('/') {
            return match self.delete_webhook(rest) {
                Ok(()) => Some(json(200, Envelope::ok("deleted".to_string()))),
                Err(message) => Some(v2_error(404, CODE_NOT_FOUND, message, request_id, None)),
            };
        }
        None
    }

    fn handle_v2_comment_user_file(
        &mut self,
        request: &ApiRequest,
        path: &str,
        query: &str,
        request_id: &str,
        connection: &Connection,
    ) -> Option<ApiResponse> {
        if path == "/api/v2/comments" && request.method == Method::Get {
            let page_filter = query_value(query, "page");
            let block_filter = query_value(query, "block");
            let mut comments: Vec<CommentDto> = self
                .workspace
                .comments
                .iter()
                .filter(|comment| {
                    if !page_filter.is_empty() {
                        return matches!(
                            &comment.parent,
                            ParentDto::Page { page_id } if page_id == &page_filter
                        );
                    }
                    if !block_filter.is_empty() {
                        return matches!(
                            &comment.parent,
                            ParentDto::Block { block_id } if block_id == &block_filter
                        );
                    }
                    true
                })
                .cloned()
                .collect();
            comments.sort_by(|a, b| a.created_time.cmp(&b.created_time));
            let page = match PageRequest::from_query(query) {
                Ok(page) => page,
                Err(error) => {
                    return Some(v2_error(
                        400,
                        error.code(),
                        error.message(),
                        request_id,
                        None,
                    ));
                }
            };
            let sort = format!("comments:{page_filter}:{block_filter}");
            return Some(v2_list_response(
                comments, &page, &sort, "comment", request_id,
            ));
        }
        if path == "/api/v2/comments" && request.method == Method::Post {
            let input = match parse_body::<CommentInput>(request) {
                Ok(input) => input,
                Err(message) => {
                    return Some(v2_error(400, CODE_INVALID_JSON, message, request_id, None));
                }
            };
            if input.attachments.len() > 3 {
                return Some(v2_error(
                    400,
                    CODE_VALIDATION_ERROR,
                    "a comment supports at most 3 attachments",
                    request_id,
                    None,
                ));
            }
            let page_id = match &input.parent {
                ParentDto::Page { page_id } if self.workspace.page_or_row(page_id).is_some() => {
                    page_id.clone()
                }
                ParentDto::Block { block_id } if self.workspace.block(block_id).is_some() => self
                    .workspace
                    .page_id_for_block(block_id)
                    .unwrap_or_else(|| "__comment__".to_string()),
                _ => {
                    return Some(v2_error(
                        400,
                        CODE_VALIDATION_ERROR,
                        "comment parent page or block not found",
                        request_id,
                        None,
                    ));
                }
            };
            let mut attachments = Vec::new();
            for attachment in &input.attachments {
                match self.comment_attachment(attachment) {
                    Ok(attachment) => attachments.push(attachment),
                    Err(message) => {
                        return Some(v2_error(
                            400,
                            CODE_VALIDATION_ERROR,
                            message,
                            request_id,
                            None,
                        ));
                    }
                }
            }
            if input.rich_text.is_empty() && attachments.is_empty() {
                return Some(v2_error(
                    400,
                    CODE_VALIDATION_ERROR,
                    "a comment needs text or an attachment",
                    request_id,
                    None,
                ));
            }
            let now = now_iso();
            let comment = CommentDto {
                object: "comment".to_string(),
                id: format!("comment-{}", uuid::Uuid::new_v4()),
                parent: input.parent,
                discussion_id: input
                    .discussion_id
                    .unwrap_or_else(|| format!("discussion-{}", uuid::Uuid::new_v4())),
                property_id: input.property_id,
                created_time: now.clone(),
                last_edited_time: now,
                created_by: UserRef::new(self.collab.owner_id()),
                rich_text: input.rich_text,
                attachments,
                display_name: input
                    .display_name
                    .as_ref()
                    .map(|display| self.resolve_display_name(display)),
                has_pending_attachments: false,
            };
            self.workspace.comments.push(comment.clone());
            if let Err(error) = self.write(ChangeOp::Create, &page_id) {
                return Some(v2_error(
                    500,
                    CODE_INTERNAL,
                    error.to_string(),
                    request_id,
                    None,
                ));
            }
            self.emit_webhook(
                WebhookEventType::CommentCreated,
                "comment",
                &comment.id,
                serde_json::json!({ "page_id": page_id.clone(), "parent": comment.parent.clone() })
                    .to_string(),
            );
            return Some(json(201, Envelope::ok(comment)));
        }

        if let Some(id) = path.strip_prefix("/api/v2/comments/") {
            if id.is_empty() || id.contains('/') {
                return Some(v2_error(
                    404,
                    CODE_NOT_FOUND,
                    "comment not found",
                    request_id,
                    None,
                ));
            }
            match request.method {
                Method::Get => match self.workspace.comments.iter().find(|c| c.id == id) {
                    Some(comment) => Some(json(200, Envelope::ok(comment.clone()))),
                    None => Some(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "comment not found",
                        request_id,
                        None,
                    )),
                },
                Method::Patch => {
                    let input = match parse_body::<CommentInput>(request) {
                        Ok(input) => input,
                        Err(message) => {
                            return Some(v2_error(
                                400,
                                CODE_INVALID_JSON,
                                message,
                                request_id,
                                None,
                            ));
                        }
                    };
                    if input.attachments.len() > 3 {
                        return Some(v2_error(
                            400,
                            CODE_VALIDATION_ERROR,
                            "a comment supports at most 3 attachments",
                            request_id,
                            None,
                        ));
                    }
                    let mut attachments = Vec::new();
                    for attachment in &input.attachments {
                        match self.comment_attachment(attachment) {
                            Ok(attachment) => attachments.push(attachment),
                            Err(message) => {
                                return Some(v2_error(
                                    400,
                                    CODE_VALIDATION_ERROR,
                                    message,
                                    request_id,
                                    None,
                                ));
                            }
                        }
                    }
                    let display_name = input
                        .display_name
                        .as_ref()
                        .map(|display| self.resolve_display_name(display));
                    let dto = {
                        let Some(comment) = self.workspace.comments.iter_mut().find(|c| c.id == id)
                        else {
                            return Some(v2_error(
                                404,
                                CODE_NOT_FOUND,
                                "comment not found",
                                request_id,
                                None,
                            ));
                        };
                        comment.rich_text = input.rich_text;
                        comment.attachments = attachments;
                        comment.property_id = input.property_id;
                        comment.display_name = display_name;
                        comment.last_edited_time = now_iso();
                        comment.clone()
                    };
                    if let Err(error) = self.write(ChangeOp::Update, "__comment__") {
                        return Some(v2_error(
                            500,
                            CODE_INTERNAL,
                            error.to_string(),
                            request_id,
                            None,
                        ));
                    }
                    self.emit_webhook(
                        WebhookEventType::CommentUpdated,
                        "comment",
                        &dto.id,
                        serde_json::json!({ "parent": dto.parent.clone() }).to_string(),
                    );
                    Some(json(200, Envelope::ok(dto)))
                }
                Method::Delete => {
                    let Some(index) = self.workspace.comments.iter().position(|c| c.id == id)
                    else {
                        return Some(v2_error(
                            404,
                            CODE_NOT_FOUND,
                            "comment not found",
                            request_id,
                            None,
                        ));
                    };
                    let comment = self.workspace.comments.remove(index);
                    if let Err(error) = self.write(ChangeOp::Delete, "__comment__") {
                        return Some(v2_error(
                            500,
                            CODE_INTERNAL,
                            error.to_string(),
                            request_id,
                            None,
                        ));
                    }
                    self.emit_webhook(
                        WebhookEventType::CommentDeleted,
                        "comment",
                        &comment.id,
                        serde_json::json!({ "parent": comment.parent.clone() }).to_string(),
                    );
                    Some(json(200, Envelope::ok(comment)))
                }
                _ => Some(v2_error(
                    404,
                    CODE_NOT_FOUND,
                    "unknown route",
                    request_id,
                    None,
                )),
            }
        } else if path == "/api/v2/users" && request.method == Method::Get {
            let level = user_info_level(connection);
            let mut users: Vec<UserDto> = self
                .collab
                .members
                .iter()
                .map(|member| user_dto(member, level))
                .collect();
            users.push(UserDto::bot(
                connection.id.clone(),
                connection.workspace_id.clone(),
                Some(self.workspace.name.clone()),
            ));
            Some(json(200, Envelope::ok(users)))
        } else if let Some(id) = path.strip_prefix("/api/v2/users/") {
            if request.method != Method::Get || id.is_empty() || id.contains('/') {
                return Some(v2_error(
                    404,
                    CODE_NOT_FOUND,
                    "user not found",
                    request_id,
                    None,
                ));
            }
            if let Some(member) = self.collab.members.iter().find(|member| member.id == id) {
                return Some(json(
                    200,
                    Envelope::ok(user_dto(member, user_info_level(connection))),
                ));
            }
            if id == connection.id {
                return Some(json(
                    200,
                    Envelope::ok(UserDto::bot(
                        connection.id.clone(),
                        connection.workspace_id.clone(),
                        Some(self.workspace.name.clone()),
                    )),
                ));
            }
            Some(v2_error(
                404,
                CODE_NOT_FOUND,
                "user not found",
                request_id,
                None,
            ))
        } else if path == "/api/v2/file_uploads" && request.method == Method::Get {
            let page = match PageRequest::from_query(query) {
                Ok(page) => page,
                Err(error) => {
                    return Some(v2_error(
                        400,
                        error.code(),
                        error.message(),
                        request_id,
                        None,
                    ));
                }
            };
            Some(v2_list_response(
                self.workspace.file_uploads.clone(),
                &page,
                "file_uploads",
                "file_upload",
                request_id,
            ))
        } else if path == "/api/v2/file_uploads" && request.method == Method::Post {
            let input = match parse_body::<FileUploadInput>(request) {
                Ok(input) => input,
                Err(message) => {
                    return Some(v2_error(400, CODE_INVALID_JSON, message, request_id, None));
                }
            };
            let now = now_iso();
            let id = format!("upload-{}", uuid::Uuid::new_v4());
            let upload = FileUpload {
                object: "file_upload".to_string(),
                id: id.clone(),
                created_time: now.clone(),
                last_edited_time: now,
                expiry_time: Some((chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339()),
                status: UploadStatus::Pending,
                filename: input.filename,
                content_type: input.content_type,
                content_length: input.content_length,
                upload_url: Some(format!("local://file_uploads/{id}/send")),
                complete_url: None,
            };
            self.workspace.file_uploads.push(upload.clone());
            if let Err(error) = self.write(ChangeOp::Create, "__file_upload__") {
                return Some(v2_error(
                    500,
                    CODE_INTERNAL,
                    error.to_string(),
                    request_id,
                    None,
                ));
            }
            self.emit_webhook(
                WebhookEventType::FileUploadCreated,
                "file_upload",
                &upload.id,
                serde_json::json!({ "status": "pending" }).to_string(),
            );
            Some(json(201, Envelope::ok(upload)))
        } else if let Some(rest) = path.strip_prefix("/api/v2/file_uploads/") {
            if let Some(id) = rest.strip_suffix("/send") {
                if request.method != Method::Post {
                    return Some(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "unknown route",
                        request_id,
                        None,
                    ));
                }
                let Some(content) = request.body.as_ref() else {
                    return Some(v2_error(
                        400,
                        CODE_VALIDATION_ERROR,
                        "file content is required",
                        request_id,
                        None,
                    ));
                };
                let dto = {
                    let Some(upload) = self
                        .workspace
                        .file_uploads
                        .iter_mut()
                        .find(|upload| upload.id == id)
                    else {
                        return Some(v2_error(
                            404,
                            CODE_NOT_FOUND,
                            "file upload not found",
                            request_id,
                            None,
                        ));
                    };
                    upload.status = UploadStatus::Uploaded;
                    upload.content_length = Some(content.len() as u64);
                    upload.upload_url = None;
                    upload.expiry_time = None;
                    upload.last_edited_time = now_iso();
                    upload.clone()
                };
                if let Err(error) = self.write(ChangeOp::Update, "__file_upload__") {
                    return Some(v2_error(
                        500,
                        CODE_INTERNAL,
                        error.to_string(),
                        request_id,
                        None,
                    ));
                }
                self.emit_webhook(
                    WebhookEventType::FileUploadCompleted,
                    "file_upload",
                    &dto.id,
                    serde_json::json!({ "status": "uploaded" }).to_string(),
                );
                return Some(json(200, Envelope::ok(dto)));
            }
            if let Some(id) = rest.strip_suffix("/complete") {
                if request.method != Method::Post {
                    return Some(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "unknown route",
                        request_id,
                        None,
                    ));
                }
                let dto = {
                    let Some(upload) = self
                        .workspace
                        .file_uploads
                        .iter_mut()
                        .find(|upload| upload.id == id)
                    else {
                        return Some(v2_error(
                            404,
                            CODE_NOT_FOUND,
                            "file upload not found",
                            request_id,
                            None,
                        ));
                    };
                    if upload.status != UploadStatus::Pending {
                        return Some(v2_error(
                            400,
                            CODE_VALIDATION_ERROR,
                            "file upload is not pending",
                            request_id,
                            None,
                        ));
                    }
                    upload.status = UploadStatus::Uploaded;
                    upload.upload_url = None;
                    upload.complete_url = None;
                    upload.expiry_time = None;
                    upload.last_edited_time = now_iso();
                    upload.clone()
                };
                if let Err(error) = self.write(ChangeOp::Update, "__file_upload__") {
                    return Some(v2_error(
                        500,
                        CODE_INTERNAL,
                        error.to_string(),
                        request_id,
                        None,
                    ));
                }
                self.emit_webhook(
                    WebhookEventType::FileUploadCompleted,
                    "file_upload",
                    &dto.id,
                    serde_json::json!({ "status": "uploaded" }).to_string(),
                );
                return Some(json(200, Envelope::ok(dto)));
            }
            if request.method == Method::Get && !rest.is_empty() && !rest.contains('/') {
                return match self
                    .workspace
                    .file_uploads
                    .iter()
                    .find(|upload| upload.id == rest)
                {
                    Some(upload) => Some(json(200, Envelope::ok(upload.clone()))),
                    None => Some(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "file upload not found",
                        request_id,
                        None,
                    )),
                };
            }
            None
        } else {
            None
        }
    }

    fn comment_attachment(&self, input: &AttachmentInput) -> Result<CommentAttachment, String> {
        let upload = self
            .workspace
            .file_uploads
            .iter()
            .find(|upload| upload.id == input.file_upload_id)
            .ok_or_else(|| "file upload not found".to_string())?;
        if upload.status != UploadStatus::Uploaded {
            return Err("file upload is not uploaded".to_string());
        }
        Ok(CommentAttachment {
            category: category_for_filename(upload.filename.as_deref()),
            file: FileObject::File {
                url: format!("local://file_uploads/{}", upload.id),
                expiry_time: None,
            },
        })
    }

    fn resolve_display_name(&self, display: &CommentDisplayName) -> CommentDisplayNameDto {
        match display {
            CommentDisplayName::Connection => CommentDisplayNameDto {
                r#type: "connection".to_string(),
                resolved_name: "Connection".to_string(),
            },
            CommentDisplayName::User => {
                let name = self
                    .collab
                    .members
                    .iter()
                    .find(|member| member.id == self.collab.owner_id())
                    .map(|member| member.name.clone())
                    .unwrap_or_else(|| "User".to_string());
                CommentDisplayNameDto {
                    r#type: "user".to_string(),
                    resolved_name: name,
                }
            }
            CommentDisplayName::Custom { name } => CommentDisplayNameDto {
                r#type: "custom".to_string(),
                resolved_name: name.clone(),
            },
        }
    }

    fn handle_v2_view(
        &mut self,
        request: &ApiRequest,
        path: &str,
        query: &str,
        request_id: &str,
    ) -> Option<ApiResponse> {
        if path == "/api/v2/views" && request.method == Method::Get {
            let data_source = query_value(query, "data_source");
            let views: Vec<View> = self
                .workspace
                .views
                .iter()
                .filter(|view| {
                    data_source.is_empty()
                        || view.data_source_id.as_deref() == Some(data_source.as_str())
                })
                .cloned()
                .collect();
            return Some(json(200, Envelope::ok(views)));
        }
        if path == "/api/v2/views" && request.method == Method::Post {
            let input = match parse_body::<ViewInput>(request) {
                Ok(input) => input,
                Err(message) => {
                    return Some(v2_error(400, CODE_INVALID_JSON, message, request_id, None));
                }
            };
            let Some(data_source_id) = input.data_source_id.clone() else {
                return Some(v2_error(
                    400,
                    CODE_VALIDATION_ERROR,
                    "data_source_id is required",
                    request_id,
                    None,
                ));
            };
            let Some(data_source) = self.workspace.data_source(&data_source_id) else {
                return Some(v2_error(
                    400,
                    CODE_VALIDATION_ERROR,
                    "data source not found",
                    request_id,
                    None,
                ));
            };
            let now = now_iso();
            let view = View {
                id: format!("view-{}", uuid::Uuid::new_v4()),
                parent: data_source.parent.clone(),
                data_source_id: Some(data_source_id),
                name: if input.name.trim().is_empty() {
                    "View".to_string()
                } else {
                    input.name
                },
                r#type: input.r#type,
                filter: input.filter,
                sorts: input.sorts,
                configuration: input.configuration.or(Some(ViewConfig::Table)),
                created_time: now.clone(),
                created_by: None,
                last_edited_time: now,
                last_edited_by: None,
                url: String::new(),
            };
            self.workspace.views.push(view.clone());
            if let Err(error) = self.write(ChangeOp::Create, "__view__") {
                return Some(v2_error(
                    500,
                    CODE_INTERNAL,
                    error.to_string(),
                    request_id,
                    None,
                ));
            }
            self.emit_webhook(
                WebhookEventType::ViewCreated,
                "view",
                &view.id,
                serde_json::json!({ "data_source_id": view.data_source_id }).to_string(),
            );
            return Some(json(201, Envelope::ok(view)));
        }

        let rest = path.strip_prefix("/api/v2/views/")?;
        if rest.is_empty() {
            return Some(v2_error(
                404,
                CODE_NOT_FOUND,
                "view not found",
                request_id,
                None,
            ));
        }
        if let Some(id) = rest.strip_suffix("/query")
            && request.method == Method::Post
        {
            let Some(view) = self.workspace.view(id) else {
                return Some(v2_error(
                    404,
                    CODE_NOT_FOUND,
                    "view not found",
                    request_id,
                    None,
                ));
            };
            let input = if request.body.is_some() {
                match parse_body::<ViewQueryInput>(request) {
                    Ok(input) => input,
                    Err(message) => {
                        return Some(v2_error(400, CODE_INVALID_JSON, message, request_id, None));
                    }
                }
            } else {
                ViewQueryInput::default()
            };
            let filter = input.filter.as_ref().or(view.filter.as_ref());
            let sorts = input.sorts.as_deref().or(view.sorts.as_deref());
            let rows = self.workspace.query_view_rows(view, filter, sorts);
            let page_request = match PageRequest::from_query(query) {
                Ok(page) => page,
                Err(error) => {
                    return Some(v2_error(
                        400,
                        error.code(),
                        error.message(),
                        request_id,
                        None,
                    ));
                }
            };
            let sort = format!("view_query:{}", view.id);
            let page = match paginate(&rows, &page_request, &sort, "page") {
                Ok(page) => page,
                Err(error) => {
                    return Some(v2_error(
                        400,
                        error.code(),
                        error.message(),
                        request_id,
                        None,
                    ));
                }
            };
            let groups = self.workspace.group_view_rows(view, &rows);
            let result =
                ViewQueryResult::new(view, groups, page.results, page.has_more, page.next_cursor);
            return Some(json(200, Envelope::ok(result)));
        }

        match request.method {
            Method::Get => match self.workspace.view(rest) {
                Some(view) => Some(json(200, Envelope::ok(view.clone()))),
                None => Some(v2_error(
                    404,
                    CODE_NOT_FOUND,
                    "view not found",
                    request_id,
                    None,
                )),
            },
            Method::Patch => {
                let input = match parse_body::<ViewInput>(request) {
                    Ok(input) => input,
                    Err(message) => {
                        return Some(v2_error(400, CODE_INVALID_JSON, message, request_id, None));
                    }
                };
                let new_parent = input.data_source_id.as_ref().and_then(|data_source_id| {
                    self.workspace
                        .data_source(data_source_id)
                        .map(|data_source| (data_source_id.clone(), data_source.parent.clone()))
                });
                let dto = {
                    let Some(view) = self.workspace.view_mut(rest) else {
                        return Some(v2_error(
                            404,
                            CODE_NOT_FOUND,
                            "view not found",
                            request_id,
                            None,
                        ));
                    };
                    if !input.name.trim().is_empty() {
                        view.name = input.name;
                    }
                    view.r#type = input.r#type;
                    view.filter = input.filter;
                    view.sorts = input.sorts;
                    if input.configuration.is_some() {
                        view.configuration = input.configuration;
                    }
                    if let Some((data_source_id, parent)) = new_parent {
                        view.data_source_id = Some(data_source_id);
                        view.parent = parent;
                    }
                    view.last_edited_time = now_iso();
                    view.clone()
                };
                if let Err(error) = self.write(ChangeOp::Update, "__view__") {
                    return Some(v2_error(
                        500,
                        CODE_INTERNAL,
                        error.to_string(),
                        request_id,
                        None,
                    ));
                }
                self.emit_webhook(
                    WebhookEventType::ViewUpdated,
                    "view",
                    &dto.id,
                    serde_json::json!({ "data_source_id": dto.data_source_id }).to_string(),
                );
                Some(json(200, Envelope::ok(dto)))
            }
            Method::Delete => {
                if self.workspace.view(rest).is_none() {
                    return Some(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "view not found",
                        request_id,
                        None,
                    ));
                }
                let deleted_id = rest.to_string();
                self.workspace.views.retain(|view| view.id != rest);
                if let Err(error) = self.write(ChangeOp::Delete, "__view__") {
                    return Some(v2_error(
                        500,
                        CODE_INTERNAL,
                        error.to_string(),
                        request_id,
                        None,
                    ));
                }
                self.emit_webhook(
                    WebhookEventType::ViewDeleted,
                    "view",
                    &deleted_id,
                    serde_json::json!({}).to_string(),
                );
                Some(json(200, Envelope::ok(deleted_id)))
            }
            _ => Some(v2_error(
                404,
                CODE_NOT_FOUND,
                "unknown route",
                request_id,
                None,
            )),
        }
    }

    fn handle_v2_database(
        &mut self,
        request: &ApiRequest,
        path: &str,
        query: &str,
        request_id: &str,
    ) -> Option<ApiResponse> {
        // Page properties: /api/v2/pages/{id}/properties
        if let Some(page_id) = path
            .strip_prefix("/api/v2/pages/")
            .and_then(|rest| rest.strip_suffix("/properties"))
            && !page_id.is_empty()
            && request.method == Method::Get
        {
            let Some(page) = self.workspace.page_or_row(page_id) else {
                return Some(v2_error(
                    404,
                    CODE_NOT_FOUND,
                    "page not found",
                    request_id,
                    None,
                ));
            };
            return Some(json(200, Envelope::ok(page.properties.clone())));
        }

        // Page property write: /api/v2/pages/{id}/properties
        if let Some(page_id) = path
            .strip_prefix("/api/v2/pages/")
            .and_then(|rest| rest.strip_suffix("/properties"))
            && !page_id.is_empty()
            && request.method == Method::Patch
        {
            let input = match parse_body::<PagePropertiesInput>(request) {
                Ok(input) => input,
                Err(message) => {
                    return Some(v2_error(400, CODE_INVALID_JSON, message, request_id, None));
                }
            };
            let dto = {
                let Some(page) = self.workspace.page_or_row_mut(page_id) else {
                    return Some(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "page not found",
                        request_id,
                        None,
                    ));
                };
                for (key, property) in input.properties {
                    page.properties.insert(key, property);
                }
                page.updated_at = now_label();
                page.clone()
            };
            if let Err(error) = self.write(ChangeOp::Update, page_id) {
                return Some(v2_error(
                    500,
                    CODE_INTERNAL,
                    error.to_string(),
                    request_id,
                    None,
                ));
            }
            return Some(json(200, Envelope::ok(dto)));
        }

        // Page property item: /api/v2/pages/{id}/properties/{property_id}
        if let Some(rest) = path.strip_prefix("/api/v2/pages/")
            && let Some((page_id, property_id)) = rest.split_once("/properties/")
            && !page_id.is_empty()
            && !property_id.is_empty()
            && request.method == Method::Get
        {
            let Some(page) = self.workspace.page_or_row(page_id) else {
                return Some(v2_error(
                    404,
                    CODE_NOT_FOUND,
                    "page not found",
                    request_id,
                    None,
                ));
            };
            let Some(property) = page.properties.get(property_id).or_else(|| {
                page.properties
                    .values()
                    .find(|property| property.id == property_id)
            }) else {
                return Some(v2_error(
                    404,
                    CODE_NOT_FOUND,
                    "property not found",
                    request_id,
                    None,
                ));
            };
            let page_request = match PageRequest::from_query(query) {
                Ok(page) => page,
                Err(error) => {
                    return Some(v2_error(
                        400,
                        error.code(),
                        error.message(),
                        request_id,
                        None,
                    ));
                }
            };
            let items = property.items();
            let sort = format!("page_property:{page_id}:{property_id}");
            return Some(v2_list_response(
                items,
                &page_request,
                &sort,
                "property_item",
                request_id,
            ));
        }

        let database_rest = path.strip_prefix("/api/v2/databases/");
        let data_source_rest = path.strip_prefix("/api/v2/data_sources/");

        match (request.method, path, database_rest, data_source_rest) {
            (Method::Get, "/api/v2/databases", _, _) => {
                Some(json(200, Envelope::ok(self.workspace.databases.clone())))
            }
            (Method::Post, "/api/v2/databases", _, _) => {
                let input = match parse_body::<DatabaseInput>(request) {
                    Ok(input) => input,
                    Err(message) => {
                        return Some(v2_error(400, CODE_INVALID_JSON, message, request_id, None));
                    }
                };
                if input.title.trim().is_empty() {
                    return Some(v2_error(
                        400,
                        CODE_VALIDATION_ERROR,
                        "database title must not be empty",
                        request_id,
                        None,
                    ));
                }
                let now = now_iso();
                let database = Database {
                    id: format!("db-{}", uuid::Uuid::new_v4()),
                    title: vec![RichText::text(input.title)],
                    data_sources: Vec::new(),
                    parent: input
                        .parent_page_id
                        .map_or(ParentDto::Workspace { workspace: true }, |page_id| {
                            ParentDto::Page { page_id }
                        }),
                    is_inline: input.is_inline,
                    created_time: now.clone(),
                    created_by: None,
                    last_edited_time: now,
                    last_edited_by: None,
                    in_trash: false,
                    url: String::new(),
                };
                self.workspace.databases.push(database.clone());
                if let Err(error) = self.write(ChangeOp::Create, "__database__") {
                    return Some(v2_error(
                        500,
                        CODE_INTERNAL,
                        error.to_string(),
                        request_id,
                        None,
                    ));
                }
                Some(json(201, Envelope::ok(database)))
            }
            (Method::Get, _, Some(id), _) if !id.is_empty() && !id.contains('/') => {
                match self.workspace.database(id) {
                    Some(database) => Some(json(200, Envelope::ok(database.clone()))),
                    None => Some(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "database not found",
                        request_id,
                        None,
                    )),
                }
            }
            (Method::Patch, _, Some(id), _) if !id.is_empty() && !id.contains('/') => {
                let input = match parse_body::<DatabaseInput>(request) {
                    Ok(input) => input,
                    Err(message) => {
                        return Some(v2_error(400, CODE_INVALID_JSON, message, request_id, None));
                    }
                };
                let dto = {
                    let Some(database) = self.workspace.database_mut(id) else {
                        return Some(v2_error(
                            404,
                            CODE_NOT_FOUND,
                            "database not found",
                            request_id,
                            None,
                        ));
                    };
                    if !input.title.trim().is_empty() {
                        database.title = vec![RichText::text(input.title)];
                    }
                    database.is_inline = input.is_inline;
                    if let Some(page_id) = input.parent_page_id {
                        database.parent = ParentDto::Page { page_id };
                    }
                    database.last_edited_time = now_iso();
                    database.clone()
                };
                if let Err(error) = self.write(ChangeOp::Update, "__database__") {
                    return Some(v2_error(
                        500,
                        CODE_INTERNAL,
                        error.to_string(),
                        request_id,
                        None,
                    ));
                }
                Some(json(200, Envelope::ok(dto)))
            }
            (Method::Delete, _, Some(id), _) if !id.is_empty() && !id.contains('/') => {
                let dto = {
                    let Some(database) = self.workspace.database_mut(id) else {
                        return Some(v2_error(
                            404,
                            CODE_NOT_FOUND,
                            "database not found",
                            request_id,
                            None,
                        ));
                    };
                    database.in_trash = true;
                    database.last_edited_time = now_iso();
                    database.clone()
                };
                if let Err(error) = self.write(ChangeOp::Delete, "__database__") {
                    return Some(v2_error(
                        500,
                        CODE_INTERNAL,
                        error.to_string(),
                        request_id,
                        None,
                    ));
                }
                Some(json(200, Envelope::ok(dto)))
            }
            (Method::Get, _, Some(id), _) if id.ends_with("/data_sources") => {
                let database_id = id.trim_end_matches("/data_sources");
                let Some(database) = self.workspace.database(database_id) else {
                    return Some(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "database not found",
                        request_id,
                        None,
                    ));
                };
                let sources: Vec<DataSource> = self
                    .workspace
                    .data_sources
                    .iter()
                    .filter(|data_source| {
                        matches!(
                            &data_source.parent,
                            ParentDto::Database { database_id: parent } if parent == database_id
                        )
                    })
                    .cloned()
                    .collect();
                let _ = database;
                Some(json(200, Envelope::ok(sources)))
            }
            (Method::Post, _, Some(id), _) if id.ends_with("/data_sources") => {
                let database_id = id.trim_end_matches("/data_sources");
                let Some(database) = self.workspace.database(database_id).cloned() else {
                    return Some(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "database not found",
                        request_id,
                        None,
                    ));
                };
                let input = match parse_body::<DataSourceInput>(request) {
                    Ok(input) => input,
                    Err(message) => {
                        return Some(v2_error(400, CODE_INVALID_JSON, message, request_id, None));
                    }
                };
                let now = now_iso();
                let title = if input.title.trim().is_empty() {
                    "Data source".to_string()
                } else {
                    input.title
                };
                let data_source = DataSource {
                    id: format!("ds-{}", uuid::Uuid::new_v4()),
                    title: vec![RichText::text(title)],
                    properties: input.properties,
                    parent: ParentDto::Database {
                        database_id: database_id.to_string(),
                    },
                    database_parent: database.parent.clone(),
                    rows: Vec::new(),
                    created_time: now.clone(),
                    created_by: None,
                    last_edited_time: now,
                    last_edited_by: None,
                    in_trash: false,
                };
                self.workspace.data_sources.push(data_source.clone());
                if let Some(database) = self.workspace.database_mut(database_id) {
                    database.data_sources.push(DataSourceRef {
                        id: data_source.id.clone(),
                        name: data_source
                            .title
                            .iter()
                            .map(RichText::plain_text)
                            .collect::<Vec<_>>()
                            .join(""),
                    });
                    database.last_edited_time = now_iso();
                }
                if let Err(error) = self.write(ChangeOp::Create, "__data_source__") {
                    return Some(v2_error(
                        500,
                        CODE_INTERNAL,
                        error.to_string(),
                        request_id,
                        None,
                    ));
                }
                Some(json(201, Envelope::ok(data_source)))
            }
            (Method::Get, _, _, Some(id)) if !id.is_empty() && !id.contains('/') => {
                match self.workspace.data_source(id) {
                    Some(data_source) => Some(json(200, Envelope::ok(data_source.clone()))),
                    None => Some(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "data source not found",
                        request_id,
                        None,
                    )),
                }
            }
            (Method::Patch, _, _, Some(id)) if !id.is_empty() && !id.contains('/') => {
                let input = match parse_body::<DataSourceInput>(request) {
                    Ok(input) => input,
                    Err(message) => {
                        return Some(v2_error(400, CODE_INVALID_JSON, message, request_id, None));
                    }
                };
                let dto = {
                    let Some(data_source) = self.workspace.data_source_mut(id) else {
                        return Some(v2_error(
                            404,
                            CODE_NOT_FOUND,
                            "data source not found",
                            request_id,
                            None,
                        ));
                    };
                    if !input.title.trim().is_empty() {
                        data_source.title = vec![RichText::text(input.title)];
                    }
                    if !input.properties.is_empty() {
                        data_source.properties = input.properties;
                    }
                    data_source.last_edited_time = now_iso();
                    data_source.clone()
                };
                if let Err(error) = self.write(ChangeOp::Update, "__data_source__") {
                    return Some(v2_error(
                        500,
                        CODE_INTERNAL,
                        error.to_string(),
                        request_id,
                        None,
                    ));
                }
                Some(json(200, Envelope::ok(dto)))
            }
            (Method::Post, _, _, Some(id)) if id.ends_with("/query") => {
                let data_source_id = id.trim_end_matches("/query");
                let Some(data_source) = self.workspace.data_source(data_source_id) else {
                    return Some(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "data source not found",
                        request_id,
                        None,
                    ));
                };
                let page_request = match PageRequest::from_query(query) {
                    Ok(page) => page,
                    Err(error) => {
                        return Some(v2_error(
                            400,
                            error.code(),
                            error.message(),
                            request_id,
                            None,
                        ));
                    }
                };
                let rows = data_source.rows.clone();
                let sort = format!("data_source_rows:{data_source_id}");
                Some(v2_list_response(
                    rows,
                    &page_request,
                    &sort,
                    "page",
                    request_id,
                ))
            }
            _ => None,
        }
    }

    fn handle_v2_block(
        &mut self,
        request: &ApiRequest,
        path: &str,
        query: &str,
        request_id: &str,
    ) -> Option<ApiResponse> {
        let rest = path.strip_prefix("/api/v2/blocks/")?;
        let (id, is_children) = match rest.strip_suffix("/children") {
            Some(id) => (id, true),
            None => (rest, false),
        };
        if id.is_empty() {
            return Some(v2_error(
                404,
                CODE_NOT_FOUND,
                "block not found",
                request_id,
                None,
            ));
        }

        match (request.method, is_children) {
            (Method::Get, false) => {
                let Some(page_id) = self.workspace.page_id_for_block(id) else {
                    return Some(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "block not found",
                        request_id,
                        None,
                    ));
                };
                let Some(block) = self.workspace.block(id) else {
                    return Some(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "block not found",
                        request_id,
                        None,
                    ));
                };
                Some(json(200, Envelope::ok(block.to_dto(&page_id))))
            }
            (Method::Get, true) => {
                let page_id = self
                    .workspace
                    .page_id_for_block(id)
                    .or_else(|| self.workspace.page(id).map(|page| page.id.clone()));
                let Some(page_id) = page_id else {
                    return Some(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "block not found",
                        request_id,
                        None,
                    ));
                };
                let Some(children) = self.workspace.block_children(id) else {
                    return Some(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "block not found",
                        request_id,
                        None,
                    ));
                };
                let page = match PageRequest::from_query(query) {
                    Ok(page) => page,
                    Err(error) => {
                        return Some(v2_error(
                            400,
                            error.code(),
                            error.message(),
                            request_id,
                            None,
                        ));
                    }
                };
                let items: Vec<BlockDto> = children
                    .iter()
                    .map(|block| block.to_dto(&page_id))
                    .collect();
                let sort = format!("block_children:{id}");
                Some(v2_list_response(items, &page, &sort, "block", request_id))
            }
            (Method::Patch, true) => {
                let body = match parse_body::<BlockChildrenRequest>(request) {
                    Ok(body) => body,
                    Err(message) => {
                        return Some(v2_error(400, CODE_INVALID_JSON, message, request_id, None));
                    }
                };
                let is_page_parent = self.workspace.page(id).is_some();
                let page_id = self
                    .workspace
                    .page_id_for_block(id)
                    .or_else(|| is_page_parent.then(|| id.to_string()));
                let Some(page_id) = page_id else {
                    return Some(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "block not found",
                        request_id,
                        None,
                    ));
                };
                let parent_block = (!is_page_parent).then(|| id.to_string());
                let actor = self.collab.owner_id();
                let mut created = Vec::new();
                {
                    let Some(children) = self.workspace.block_children_mut(id) else {
                        return Some(v2_error(
                            404,
                            CODE_NOT_FOUND,
                            "block not found",
                            request_id,
                            None,
                        ));
                    };
                    for input in body.children {
                        let child_id = uuid::Uuid::new_v4().to_string();
                        let child = Block::from_input(
                            input,
                            child_id,
                            parent_block.clone(),
                            Some(actor.clone()),
                        );
                        children.push(child.clone());
                        created.push(child);
                    }
                }
                if let Err(error) = self.write(ChangeOp::Update, &page_id) {
                    return Some(v2_error(
                        500,
                        CODE_INTERNAL,
                        error.to_string(),
                        request_id,
                        None,
                    ));
                }
                let items: Vec<BlockDto> =
                    created.iter().map(|block| block.to_dto(&page_id)).collect();
                Some(json(200, Envelope::ok(items)))
            }
            (Method::Patch, false) => {
                let input = match parse_body::<BlockInput>(request) {
                    Ok(input) => input,
                    Err(message) => {
                        return Some(v2_error(400, CODE_INVALID_JSON, message, request_id, None));
                    }
                };
                let Some(page_id) = self.workspace.page_id_for_block(id) else {
                    return Some(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "block not found",
                        request_id,
                        None,
                    ));
                };
                let actor = self.collab.owner_id();
                let dto = {
                    let Some(block) = self.workspace.block_mut(id) else {
                        return Some(v2_error(
                            404,
                            CODE_NOT_FOUND,
                            "block not found",
                            request_id,
                            None,
                        ));
                    };
                    let type_name = if input.r#type.is_empty() {
                        block.kind.as_api_type(block.unsupported_type.as_deref())
                    } else {
                        input.r#type.clone()
                    };
                    block.kind = BlockKind::from_api_type(&type_name);
                    block.unsupported_type =
                        (block.kind == BlockKind::Unsupported).then_some(type_name);
                    if !input.rich_text.is_empty() || input.text.is_some() {
                        block.rich_text = input.rich_text;
                        if block.rich_text.is_empty()
                            && let Some(text) = input.text
                        {
                            block.rich_text = vec![RichText::text(text)];
                        }
                        block.sync_text_from_rich_text();
                    }
                    block.checked = input.checked;
                    block.collapsed = input.collapsed;
                    block.table_width = input.table_width;
                    block.has_column_header = input.has_column_header;
                    block.has_row_header = input.has_row_header;
                    block.cells = input.cells;
                    block.child_page_title = input.child_page_title;
                    block.last_edited_time = now_iso();
                    block.last_edited_by = Some(actor);
                    block.to_dto(&page_id)
                };
                if let Err(error) = self.write(ChangeOp::Update, &page_id) {
                    return Some(v2_error(
                        500,
                        CODE_INTERNAL,
                        error.to_string(),
                        request_id,
                        None,
                    ));
                }
                Some(json(200, Envelope::ok(dto)))
            }
            (Method::Delete, false) => {
                let Some(page_id) = self.workspace.page_id_for_block(id) else {
                    return Some(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "block not found",
                        request_id,
                        None,
                    ));
                };
                let actor = self.collab.owner_id();
                let dto = {
                    let Some(block) = self.workspace.block_mut(id) else {
                        return Some(v2_error(
                            404,
                            CODE_NOT_FOUND,
                            "block not found",
                            request_id,
                            None,
                        ));
                    };
                    block.in_trash = true;
                    block.last_edited_time = now_iso();
                    block.last_edited_by = Some(actor);
                    block.to_dto(&page_id)
                };
                if let Err(error) = self.write(ChangeOp::Update, &page_id) {
                    return Some(v2_error(
                        500,
                        CODE_INTERNAL,
                        error.to_string(),
                        request_id,
                        None,
                    ));
                }
                Some(json(200, Envelope::ok(dto)))
            }
            _ => Some(v2_error(
                404,
                CODE_NOT_FOUND,
                "unknown route",
                request_id,
                None,
            )),
        }
    }

    fn handle_v2_list(
        &mut self,
        request: &ApiRequest,
        path: &str,
        query: &str,
        request_id: &str,
    ) -> Option<ApiResponse> {
        if request.method != Method::Get {
            return None;
        }
        let is_list_path = matches!(
            path,
            "/api/v2/pages"
                | "/api/v2/members"
                | "/api/v2/shares"
                | "/api/v2/sync"
                | "/api/v2/search"
        ) || (path.starts_with("/api/v2/pages/") && path.ends_with("/versions"));
        if !is_list_path {
            return None;
        }
        let page = match PageRequest::from_query(query) {
            Ok(page) => page,
            Err(error) => {
                return Some(v2_error(
                    400,
                    error.code(),
                    error.message(),
                    request_id,
                    None,
                ));
            }
        };

        if path == "/api/v2/pages" {
            let items: Vec<PageSummary> = self
                .workspace
                .pages
                .iter()
                .map(|page| page.summary())
                .collect();
            return Some(v2_list_response(items, &page, "pages", "page", request_id));
        }
        if path == "/api/v2/members" {
            return Some(v2_list_response(
                self.collab.members.clone(),
                &page,
                "members",
                "member",
                request_id,
            ));
        }
        if path == "/api/v2/shares" {
            let page_id = query_value(query, "page");
            let shares = if page_id.is_empty() {
                self.collab.shares.clone()
            } else {
                self.collab.shares_for(&page_id)
            };
            let sort = format!("shares:{page_id}");
            return Some(v2_list_response(shares, &page, &sort, "share", request_id));
        }
        if path == "/api/v2/sync" {
            let since = query_value(query, "since").parse::<u64>().unwrap_or(0);
            let changes = self.collab.changes_since(since).changes;
            let sort = format!("sync:{since}");
            return Some(v2_list_response(
                changes, &page, &sort, "change", request_id,
            ));
        }
        if path == "/api/v2/search" {
            let q = query_value(query, "q");
            let results = self.workspace.search(&q);
            let sort = format!("search:{q}");
            return Some(v2_list_response(
                results,
                &page,
                &sort,
                "search_result",
                request_id,
            ));
        }
        if let Some(id) = path
            .strip_prefix("/api/v2/pages/")
            .and_then(|rest| rest.strip_suffix("/versions"))
        {
            let versions = self.collab.versions_for(id);
            let sort = format!("versions:{id}");
            return Some(v2_list_response(
                versions,
                &page,
                &sort,
                "page_version",
                request_id,
            ));
        }
        None
    }
}

fn v2_list_response<T: Clone + Serialize>(
    items: Vec<T>,
    page: &PageRequest,
    sort: &str,
    type_name: &str,
    request_id: &str,
) -> ApiResponse {
    match paginate(&items, page, sort, type_name) {
        Ok(list) => json(200, Envelope::ok(list)),
        Err(error) => v2_error(400, error.code(), error.message(), request_id, None),
    }
}

impl WorkspaceApi for WorkspaceServer {
    fn handle(&mut self, request: &ApiRequest) -> ApiResponse {
        if request.path.starts_with("/api/v2") {
            self.handle_v2(request)
        } else {
            self.handle_v1(request)
        }
    }
}

impl WorkspaceServer {
    fn handle_v1(&mut self, request: &ApiRequest) -> ApiResponse {
        let (path, query) = match request.path.split_once('?') {
            Some((p, q)) => (p, q),
            None => (request.path.as_str(), ""),
        };

        match (request.method, path) {
            (Method::Get, "/health") => json(200, Envelope::ok("ok".to_string())),

            (Method::Get, "/api/workspace") => json(200, Envelope::ok(self.workspace.clone())),
            (Method::Put, "/api/workspace") => match parse_body::<Workspace>(request) {
                Ok(workspace) => {
                    let actor = self.collab.owner_id();
                    for page in &workspace.pages {
                        let changed = self
                            .workspace
                            .page(&page.id)
                            .map(|old| old != page)
                            .unwrap_or(true);
                        if changed {
                            self.collab.record_version(page, &actor);
                        }
                    }
                    self.workspace = workspace;
                    if let Err(error) = self.write(ChangeOp::Replace, "__workspace__") {
                        return server_error(&error.to_string());
                    }
                    json(200, Envelope::ok(self.workspace.clone()))
                }
                Err(message) => bad(&message),
            },

            (Method::Get, "/api/pages") => {
                let summaries: Vec<PageSummary> =
                    self.workspace.pages.iter().map(|p| p.summary()).collect();
                json(200, Envelope::ok(summaries))
            }
            (Method::Post, "/api/pages") => match parse_body::<PageDraft>(request) {
                Ok(draft) => {
                    let title = draft.title.trim().to_string();
                    if title.is_empty() {
                        return bad("page title must not be empty");
                    }
                    if let Some(parent) = draft.parent.as_deref()
                        && self.workspace.page(parent).is_none()
                    {
                        return bad("unknown parent page");
                    }
                    let icon = if draft.icon.trim().is_empty() {
                        "P".to_string()
                    } else {
                        draft.icon.clone()
                    };
                    let mut page = Page::new(uuid::Uuid::new_v4().to_string(), title, icon);
                    page.parent = draft.parent;
                    if let Err(e) = self.workspace.add_page(page.clone()) {
                        return bad(&e.to_string());
                    }
                    let page_id = page.id.clone();
                    let actor = self.collab.owner_id();
                    self.collab.record_version(&page, &actor);
                    if let Err(error) = self.write(ChangeOp::Create, &page_id) {
                        return server_error(&error.to_string());
                    }
                    self.emit_webhook(
                        WebhookEventType::PageCreated,
                        "page",
                        &page_id,
                        serde_json::json!({ "parent": page.parent }).to_string(),
                    );
                    json(201, Envelope::ok(page))
                }
                Err(message) => bad(&message),
            },

            (Method::Get, "/api/search") => {
                let q = query_value(query, "q");
                let results: Vec<SearchResult> = self.workspace.search(&q);
                json(200, Envelope::ok(results))
            }

            // ---- decided collaborative API (see docs/adr-003-*) ----------
            (Method::Get, "/api/members") => json(200, Envelope::ok(self.collab.members.clone())),
            (Method::Post, "/api/members") => match parse_body::<MemberDraft>(request) {
                Ok(draft) => {
                    if draft.name.trim().is_empty() || draft.email.trim().is_empty() {
                        return bad("member name and email must not be empty");
                    }
                    let member = self.collab.invite(draft);
                    if let Err(error) = self.persist_collab() {
                        return server_error(&error.to_string());
                    }
                    json(201, Envelope::ok(member))
                }
                Err(message) => bad(&message),
            },
            (Method::Get, "/api/shares") => {
                let page = query_value(query, "page");
                let shares = if page.is_empty() {
                    self.collab.shares.clone()
                } else {
                    self.collab.shares_for(&page)
                };
                json(200, Envelope::ok(shares))
            }
            (Method::Put, "/api/shares") => match parse_body::<Share>(request) {
                Ok(share) => {
                    self.collab.set_access(share.clone());
                    if let Err(error) = self.persist_collab() {
                        return server_error(&error.to_string());
                    }
                    json(200, Envelope::ok(share))
                }
                Err(message) => bad(&message),
            },
            (Method::Get, "/api/sync") => {
                let since = query_value(query, "since").parse::<u64>().unwrap_or(0);
                json(200, Envelope::ok(self.collab.changes_since(since)))
            }
            (Method::Get, "/api/export") => json(
                200,
                Envelope::ok(export_tree(&self.workspace, &self.collab.members)),
            ),
            (Method::Get, "/api/realtime") => json(200, Envelope::ok(realtime_contract())),
            // ---------------------------------------------------------------
            (method, page_path) if page_path.starts_with("/api/pages/") => {
                let rest = &page_path["/api/pages/".len()..];
                if rest.is_empty() {
                    return not_found("unknown route");
                }
                if let Some(id) = rest.strip_suffix("/versions") {
                    if id.is_empty() {
                        return not_found("unknown route");
                    }
                    return match method {
                        Method::Get => json(200, Envelope::ok(self.collab.versions_for(id))),
                        _ => not_found("unknown route"),
                    };
                }
                let id = rest;
                match method {
                    Method::Get => match self.workspace.page(id) {
                        Some(page) => json(200, Envelope::ok(page.clone())),
                        None => not_found("page not found"),
                    },
                    Method::Patch => match parse_body::<Page>(request) {
                        Ok(mut page) => {
                            page.id = id.to_string();
                            let was_locked = self
                                .workspace
                                .page(id)
                                .map(|existing| existing.locked)
                                .unwrap_or(false);
                            match self.workspace.page_mut(id) {
                                Some(slot) => {
                                    *slot = page.clone();
                                    let actor = self.collab.owner_id();
                                    self.collab.record_version(&page, &actor);
                                    if let Err(error) = self.write(ChangeOp::Update, id) {
                                        return server_error(&error.to_string());
                                    }
                                    if was_locked != page.locked {
                                        self.emit_webhook(
                                            if page.locked {
                                                WebhookEventType::PageLocked
                                            } else {
                                                WebhookEventType::PageUnlocked
                                            },
                                            "page",
                                            id,
                                            serde_json::json!({}).to_string(),
                                        );
                                    } else {
                                        self.emit_webhook(
                                            WebhookEventType::PageContentUpdated,
                                            "page",
                                            id,
                                            serde_json::json!({}).to_string(),
                                        );
                                    }
                                    json(200, Envelope::ok(page))
                                }
                                None => not_found("page not found"),
                            }
                        }
                        Err(message) => bad(&message),
                    },
                    Method::Delete => match self.workspace.remove_page(id) {
                        Ok(removed) => {
                            if let Err(error) = self.write(ChangeOp::Delete, id) {
                                return server_error(&error.to_string());
                            }
                            self.emit_webhook(
                                WebhookEventType::PageDeleted,
                                "page",
                                id,
                                serde_json::json!({}).to_string(),
                            );
                            json(200, Envelope::ok(removed))
                        }
                        Err(e) => not_found(&e.to_string()),
                    },
                    _ => not_found("unknown route"),
                }
            }

            _ => not_found("unknown route"),
        }
    }
}

fn user_info_level(connection: &Connection) -> u8 {
    if connection.has(Capability::UserInfoWithEmail) {
        2
    } else if connection.has(Capability::UserInfoWithoutEmail) {
        1
    } else {
        0
    }
}

fn user_dto(member: &Member, level: u8) -> UserDto {
    if level == 0 {
        return UserDto {
            object: "user".to_string(),
            id: member.id.clone(),
            r#type: None,
            name: None,
            avatar_url: None,
            person: None,
            bot: None,
        };
    }
    UserDto {
        object: "user".to_string(),
        id: member.id.clone(),
        r#type: Some(UserType::Person),
        name: Some(member.name.clone()),
        avatar_url: None,
        person: Some(Person {
            email: (level >= 2).then(|| member.email.clone()),
            email_verified: level >= 2,
        }),
        bot: None,
    }
}

fn backoff_time(attempts: u8) -> String {
    let seconds = 2u64.saturating_pow(attempts as u32).min(3600);
    (chrono::Utc::now() + chrono::Duration::seconds(seconds as i64)).to_rfc3339()
}

fn category_for_filename(name: Option<&str>) -> String {
    let extension = name
        .and_then(|name| name.rsplit_once('.'))
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .unwrap_or_default();
    match extension.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "heic" | "bmp" | "tif" | "tiff" => {
            "image"
        }
        "pdf" => "pdf",
        "mp4" | "mov" | "webm" | "mkv" => "video",
        "mp3" | "wav" | "ogg" | "oga" | "m4a" => "audio",
        _ => "productivity",
    }
    .to_string()
}
