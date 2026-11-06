//! Server side of the application.
//!
//! `WorkspaceServer` owns all state and answers `ApiRequest`s through the
//! `WorkspaceApi` seam. `serve_once` / `serve_forever` expose the very same
//! contract over loopback HTTP/1.1 for the optional `--serve` mode.

use crate::collab::{
    export_tree, realtime_contract, ChangeOp, CollabState, MemberDraft, Share,
};
use crate::model::{Page, PageDraft, PageSummary, SearchResult, Workspace};
use crate::store::{JsonStore, NoStore, WorkspaceStore};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::time::Duration;

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
/// `WorkspaceStore` port, so the core never depends on a concrete backend.
/// Collaboration state (accounts, permissions, sync log) is kept beside the
/// workspace and exposed through the decided API.
pub struct WorkspaceServer {
    pub workspace: Workspace,
    pub collab: CollabState,
    store: Box<dyn WorkspaceStore>,
}

impl WorkspaceServer {
    /// An in-memory server with no persistence (the null-object adapter).
    pub fn new(workspace: Workspace) -> Self {
        Self::with_store(workspace, NoStore)
    }

    /// A server that persists every mutation to a JSON file.
    pub fn with_path(workspace: Workspace, path: PathBuf) -> Self {
        Self::with_store(workspace, JsonStore::new(path))
    }

    /// Inject any persistence adapter without changing server logic (DIP/OCP).
    pub fn with_store<S: WorkspaceStore + 'static>(workspace: Workspace, store: S) -> Self {
        Self {
            workspace,
            collab: CollabState::new(),
            store: Box::new(store),
        }
    }

    pub fn snapshot(&self) -> &Workspace {
        &self.workspace
    }

    /// Read-only access to accounts, permissions and the sync log.
    pub fn collab(&self) -> &CollabState {
        &self.collab
    }

    pub fn persist(&self) -> std::io::Result<()> {
        self.store.persist(&self.workspace)
    }

    /// Single write path (DRY): bump the version, append one sync-log entry,
    /// and persist. Every mutation goes through here.
    fn write(&mut self, op: ChangeOp, page_id: &str) {
        self.workspace.bump_version();
        let actor = self.collab.owner_id();
        self.collab.record(op, page_id, &actor);
        let _ = self.persist();
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

/// Minimal percent-decoding for the `?q=` search parameter.
fn url_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    out.push(byte);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            other => {
                out.push(other);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

impl WorkspaceApi for WorkspaceServer {
    fn handle(&mut self, request: &ApiRequest) -> ApiResponse {
        let (path, query) = match request.path.split_once('?') {
            Some((p, q)) => (p, q),
            None => (request.path.as_str(), ""),
        };

        match (request.method, path) {
            (Method::Get, "/health") => json(200, Envelope::ok("ok".to_string())),

            (Method::Get, "/api/workspace") => {
                json(200, Envelope::ok(self.workspace.clone()))
            }
            (Method::Put, "/api/workspace") => match parse_body::<Workspace>(request) {
                Ok(workspace) => {
                    self.workspace = workspace;
                    self.write(ChangeOp::Replace, "__workspace__");
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
                    self.write(ChangeOp::Create, &page_id);
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
                    json(201, Envelope::ok(self.collab.invite(draft)))
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
            },
            (Method::Put, "/api/shares") => match parse_body::<Share>(request) {
                Ok(share) => {
                    self.collab.set_access(share.clone());
                    json(200, Envelope::ok(share))
                }
                Err(message) => bad(&message),
            },
            (Method::Get, "/api/sync") => {
                let since = query_value(query, "since").parse::<u64>().unwrap_or(0);
                json(200, Envelope::ok(self.collab.changes_since(since)))
            },
            (Method::Get, "/api/export") => {
                json(200, Envelope::ok(export_tree(&self.workspace, &self.collab.members)))
            },
            (Method::Get, "/api/realtime") => json(200, Envelope::ok(realtime_contract())),
            // ---------------------------------------------------------------

            (method, page_path) if page_path.starts_with("/api/pages/") => {
                let id = &page_path["/api/pages/".len()..];
                if id.is_empty() {
                    return not_found("unknown route");
                }
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
                                    self.write(ChangeOp::Update, id);
                                    json(200, Envelope::ok(page))
                                }
                                None => not_found("page not found"),
                            }
                        }
                        Err(message) => bad(&message),
                    },
                    Method::Delete => match self.workspace.remove_page(id) {
                        Ok(removed) => {
                            self.write(ChangeOp::Delete, id);
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

/// Parse one HTTP/1.1 request into an `ApiRequest`.
pub fn parse_http_request(bytes: &[u8]) -> Option<ApiRequest> {
    let text = String::from_utf8_lossy(bytes);
    let (head, body_bytes) = match text.split_once("\r\n\r\n") {
        Some((h, b)) => (h, b),
        None => (text.as_ref(), ""),
    };
    let mut lines = head.split("\r\n");
    let mut words = lines.next().unwrap_or("").split_whitespace();
    let method = Method::parse(words.next().unwrap_or(""))?;
    let path = words.next().unwrap_or("").to_string();
    if path.is_empty() {
        return None;
    }
    let mut content_length = 0usize;
    for line in lines {
        if let Some((key, value)) = line.split_once(':')
            && key.trim().eq_ignore_ascii_case("content-length")
        {
            content_length = value.trim().parse().unwrap_or(0);
        }
    }
    let body = if content_length > 0 {
        let raw = body_bytes.as_bytes();
        let end = content_length.min(raw.len());
        Some(String::from_utf8_lossy(&raw[..end]).into_owned())
    } else {
        None
    };
    Some(ApiRequest {
        method,
        path,
        body,
    })
}

/// Serialize an `ApiResponse` as an HTTP/1.1 response.
pub fn format_http_response(response: &ApiResponse) -> Vec<u8> {
    let reason = match response.status {
        200 => "OK",
        201 => "Created",
        400 => "Bad Request",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "OK",
    };
    format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n{}",
        response.status,
        reason,
        response.body.len(),
        response.body
    )
    .into_bytes()
}

fn request_complete(buffer: &[u8]) -> bool {
    let Some(index) = buffer.windows(4).position(|w| w == b"\r\n\r\n") else {
        return false;
    };
    let head = String::from_utf8_lossy(&buffer[..index]);
    let mut length = 0usize;
    for line in head.split("\r\n").skip(1) {
        if let Some((key, value)) = line.split_once(':')
            && key.trim().eq_ignore_ascii_case("content-length")
        {
            length = value.trim().parse().unwrap_or(0);
        }
    }
    buffer.len() >= index + 4 + length
}

/// Handle one connection and write one response.
pub fn serve_once<A: WorkspaceApi>(api: &mut A, stream: &mut TcpStream) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];
    for _ in 0..32 {
        if request_complete(&buffer) {
            break;
        }
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => buffer.extend_from_slice(&chunk[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => break,
            Err(e) => return Err(e),
        }
    }
    let request = parse_http_request(&buffer)
        .unwrap_or_else(|| ApiRequest::new(Method::Get, "/health", None));
    let response = api.handle(&request);
    stream.write_all(&format_http_response(&response))?;
    stream.flush()
}

/// Blocking loopback API server. Only used by `--serve`.
pub fn serve_forever(addr: &str, mut server: WorkspaceServer) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr)?;
    println!("not-notion API listening on http://{addr}");
    for incoming in listener.incoming() {
        match incoming {
            Ok(mut stream) => {
                let _ = serve_once(&mut server, &mut stream);
            }
            Err(e) => eprintln!("connection error: {e}"),
        }
    }
    Ok(())
}

/// Percent-encode a string for use in a query parameter.
pub fn url_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}
