//! Server side of the application.
//!
//! `WorkspaceServer` owns all state and answers `ApiRequest`s through the
//! `WorkspaceApi` seam. `serve_once` / `serve_forever` expose the very same
//! contract over loopback HTTP/1.1 for the optional `--serve` mode.

use crate::collab::{ChangeOp, CollabState, MemberDraft, Share, export_tree, realtime_contract};
use crate::model::{
    Block, BlockChildrenRequest, BlockDto, BlockInput, BlockKind, DataSource, DataSourceInput,
    DataSourceRef, Database, DatabaseInput, Page, PageDraft, PagePropertiesInput, PageSummary,
    ParentDto, RichText, SearchResult, View, ViewConfig, ViewInput, ViewQueryInput,
    ViewQueryResult, Workspace, now_iso, now_label,
};
use crate::store::{
    CollabStore, JsonCollabStore, JsonStore, NoCollabStore, NoStore, WorkspaceStore,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

mod contract;
mod http;
mod pagination;
mod url;

pub use contract::{
    API_VERSION, AUTH_HEADER, ApiError, CODE_INTERNAL, CODE_INVALID_CURSOR, CODE_INVALID_JSON,
    CODE_MISSING_VERSION, CODE_NOT_FOUND, CODE_RESTRICTED_RESOURCE, CODE_UNAUTHORIZED,
    CODE_UNSUPPORTED_VERSION, CODE_VALIDATION_ERROR, Capability, Connection, ListResponse,
    OWNER_TOKEN, RequestLogEntry, SUPPORTED_VERSIONS, VERSION_HEADER, new_request_id, parse_bearer,
    version_supported,
};
pub use http::{format_http_response, parse_http_request, serve_forever, serve_on, serve_once};
pub use pagination::{PageRequest, PaginationError, paginate};
use std::collections::BTreeMap;
use std::path::PathBuf;
use url::url_decode;
pub use url::url_encode;

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
    connections: Vec<Connection>,
    request_log: Vec<RequestLogEntry>,
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
        let owner = collab.owner_id();
        let workspace_id = workspace.settings.id.clone();
        Self {
            workspace,
            collab,
            store: Box::new(store),
            collab_store: Box::new(collab_store),
            connections: vec![Connection::owner(owner, workspace_id)],
            request_log: Vec::new(),
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

    pub fn connection_for_token(&self, token: &str) -> Option<&Connection> {
        self.connections.iter().find(|c| c.token == token)
    }

    /// In-memory request log for `/api/v2`; the same IDs are written to the
    /// HTTP response and (in a production adapter) to structured logs.
    pub fn request_log(&self) -> &[RequestLogEntry] {
        &self.request_log
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
        self.collab.record(op, page_id, &actor);
        self.persist()
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
        let response = self.handle_v2_inner(request, path, query, &request_id);
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

        let Some(token) = request.header(AUTH_HEADER).and_then(parse_bearer) else {
            return v2_error(
                401,
                CODE_UNAUTHORIZED,
                "missing or malformed bearer token",
                request_id,
                None,
            );
        };
        let Some(connection) = self.connection_for_token(token).cloned() else {
            return v2_error(
                401,
                CODE_UNAUTHORIZED,
                "invalid bearer token",
                request_id,
                None,
            );
        };

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

        self.workspace.ensure_data_sources();
        self.workspace.ensure_views();
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
                Some(json(200, Envelope::ok(rest.to_string())))
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
                            match self.workspace.page_mut(id) {
                                Some(slot) => {
                                    *slot = page.clone();
                                    let actor = self.collab.owner_id();
                                    self.collab.record_version(&page, &actor);
                                    if let Err(error) = self.write(ChangeOp::Update, id) {
                                        return server_error(&error.to_string());
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
