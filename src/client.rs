//! Client side of the application.
//!
//! `Client<A>` is the only data access object the UI may use. It builds
//! `ApiRequest`s, checks status codes, and decodes JSON envelopes. Because it
//! is generic over `WorkspaceApi`, the same client works with the in-process
//! server and, later, a remote transport.

use crate::collab::{
    ExportFile, Member, MemberDraft, PageVersion, RealtimeContract, Share, SyncBatch,
};
use crate::model::{Page, PageDraft, PageSummary, SearchResult, Workspace};
use crate::server::{url_encode, ApiRequest, Envelope, Method, WorkspaceApi};
use serde::de::DeserializeOwned;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientError {
    Http(u16, String),
    Decode(String),
    NotFound(String),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientError::Http(status, body) => write!(f, "server returned {status}: {body}"),
            ClientError::Decode(message) => write!(f, "could not decode response: {message}"),
            ClientError::NotFound(id) => write!(f, "not found: {id}"),
        }
    }
}

impl std::error::Error for ClientError {}

pub struct Client<A: WorkspaceApi> {
    api: A,
}

impl<A: WorkspaceApi> Client<A> {
    pub fn new(api: A) -> Self {
        Self { api }
    }

    pub fn into_inner(self) -> A {
        self.api
    }

    fn call<T: DeserializeOwned>(&mut self, request: ApiRequest) -> Result<T, ClientError> {
        let response = self.api.handle(&request);
        if response.status == 404 {
            return Err(ClientError::NotFound(response.body));
        }
        if !(200..300).contains(&response.status) {
            return Err(ClientError::Http(response.status, response.body));
        }
        let envelope: Envelope<T> = serde_json::from_str(&response.body)
            .map_err(|e| ClientError::Decode(e.to_string()))?;
        match (envelope.data, envelope.error) {
            (Some(data), _) => Ok(data),
            (None, Some(message)) => Err(ClientError::Http(response.status, message)),
            (None, None) => Err(ClientError::Decode("response has no data".to_string())),
        }
    }

    pub fn workspace(&mut self) -> Result<Workspace, ClientError> {
        self.call(ApiRequest::new(Method::Get, "/api/workspace", None))
    }

    pub fn replace_workspace(&mut self, workspace: &Workspace) -> Result<Workspace, ClientError> {
        let body = serde_json::to_string(workspace).unwrap_or_default();
        self.call(ApiRequest::new(Method::Put, "/api/workspace", Some(body)))
    }

    pub fn list_pages(&mut self) -> Result<Vec<PageSummary>, ClientError> {
        self.call(ApiRequest::new(Method::Get, "/api/pages", None))
    }

    pub fn get_page(&mut self, id: &str) -> Result<Page, ClientError> {
        self.call(ApiRequest::new(
            Method::Get,
            format!("/api/pages/{id}"),
            None,
        ))
    }

    pub fn create_page(&mut self, draft: &PageDraft) -> Result<Page, ClientError> {
        let body = serde_json::to_string(draft).unwrap_or_default();
        self.call(ApiRequest::new(Method::Post, "/api/pages", Some(body)))
    }

    pub fn update_page(&mut self, page: &Page) -> Result<Page, ClientError> {
        let body = serde_json::to_string(page).unwrap_or_default();
        self.call(ApiRequest::new(
            Method::Patch,
            format!("/api/pages/{}", page.id),
            Some(body),
        ))
    }

    pub fn delete_page(&mut self, id: &str) -> Result<Vec<String>, ClientError> {
        self.call(ApiRequest::new(
            Method::Delete,
            format!("/api/pages/{id}"),
            None,
        ))
    }

    /// Newest-first snapshots recorded by the server (version history).
    pub fn page_versions(&mut self, id: &str) -> Result<Vec<PageVersion>, ClientError> {
        self.call(ApiRequest::new(
            Method::Get,
            format!("/api/pages/{id}/versions"),
            None,
        ))
    }

    pub fn search(&mut self, query: &str) -> Result<Vec<SearchResult>, ClientError> {
        let encoded = url_encode(query);
        self.call(ApiRequest::new(
            Method::Get,
            format!("/api/search?q={encoded}"),
            None,
        ))
    }

    // ---- decided collaborative API ---------------------------------------
    pub fn list_members(&mut self) -> Result<Vec<Member>, ClientError> {
        self.call(ApiRequest::new(Method::Get, "/api/members", None))
    }

    pub fn invite_member(&mut self, draft: &MemberDraft) -> Result<Member, ClientError> {
        let body = serde_json::to_string(draft).unwrap_or_default();
        self.call(ApiRequest::new(Method::Post, "/api/members", Some(body)))
    }

    pub fn list_shares(&mut self, page_id: Option<&str>) -> Result<Vec<Share>, ClientError> {
        let path = match page_id {
            Some(id) => format!("/api/shares?page={}", url_encode(id)),
            None => "/api/shares".to_string(),
        };
        self.call(ApiRequest::new(Method::Get, path, None))
    }

    pub fn set_share(&mut self, share: &Share) -> Result<Share, ClientError> {
        let body = serde_json::to_string(share).unwrap_or_default();
        self.call(ApiRequest::new(Method::Put, "/api/shares", Some(body)))
    }

    pub fn sync(&mut self, since: u64) -> Result<SyncBatch, ClientError> {
        self.call(ApiRequest::new(
            Method::Get,
            format!("/api/sync?since={since}"),
            None,
        ))
    }

    pub fn export_tree(&mut self) -> Result<Vec<ExportFile>, ClientError> {
        self.call(ApiRequest::new(Method::Get, "/api/export", None))
    }

    pub fn realtime_contract(&mut self) -> Result<RealtimeContract, ClientError> {
        self.call(ApiRequest::new(Method::Get, "/api/realtime", None))
    }
}
