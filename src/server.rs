//! Server side of the application.
//!
//! `WorkspaceServer` owns all state and answers `ApiRequest`s through the
//! `WorkspaceApi` seam. `serve_once` / `serve_forever` expose the very same
//! contract over loopback HTTP/1.1 for the optional `--serve` mode.

use crate::collab::{ChangeOp, CollabState, MemberDraft, Share, export_tree, realtime_contract};
use crate::model::{Page, PageDraft, PageSummary, SearchResult, Workspace};
use crate::store::{
    CollabStore, JsonCollabStore, JsonStore, NoCollabStore, NoStore, WorkspaceStore,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

mod http;
mod url;

pub use http::{format_http_response, parse_http_request, serve_forever, serve_on, serve_once};
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
}

impl ApiRequest {
    pub fn new(method: Method, path: impl Into<String>, body: Option<String>) -> Self {
        Self {
            method,
            path: path.into(),
            body,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiResponse {
    pub status: u16,
    pub body: String,
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
        Self {
            workspace,
            collab,
            store: Box::new(store),
            collab_store: Box::new(collab_store),
        }
    }

    pub fn snapshot(&self) -> &Workspace {
        &self.workspace
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

impl WorkspaceApi for WorkspaceServer {
    fn handle(&mut self, request: &ApiRequest) -> ApiResponse {
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
