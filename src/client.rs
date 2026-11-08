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
use crate::server::{
    API_VERSION, AUTH_HEADER, ApiError, ApiRequest, Envelope, Method, VERSION_HEADER, WorkspaceApi,
    url_encode,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum ClientError {
    Http(u16, String),
    Api(Box<ApiError>),
    Encode(String),
    Decode(String),
    NotFound(String),
    MissingToken,
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientError::Http(status, body) => write!(f, "server returned {status}: {body}"),
            ClientError::Api(error) => write!(
                f,
                "server returned {} {}: {}",
                error.status, error.code, error.message
            ),
            ClientError::Encode(message) => write!(f, "could not encode request: {message}"),
            ClientError::Decode(message) => write!(f, "could not decode response: {message}"),
            ClientError::NotFound(id) => write!(f, "not found: {id}"),
            ClientError::MissingToken => write!(f, "v2 request requires a bearer token"),
        }
    }
}

/// Serialize a request body, returning an explicit encode error instead of
/// sending an empty body (spec-021).
pub fn encode_body<T: Serialize>(value: &T) -> Result<String, ClientError> {
    serde_json::to_string(value).map_err(|e| ClientError::Encode(e.to_string()))
}

impl std::error::Error for ClientError {}

pub struct Client<A: WorkspaceApi> {
    api: A,
    token: Option<String>,
}

impl<A: WorkspaceApi> Client<A> {
    pub fn new(api: A) -> Self {
        Self { api, token: None }
    }

    /// Build a client that sends the v2 version and bearer headers.
    pub fn with_token(api: A, token: impl Into<String>) -> Self {
        Self {
            api,
            token: Some(token.into()),
        }
    }

    pub fn into_inner(self) -> A {
        self.api
    }

    fn v2_request(
        &self,
        method: Method,
        path: impl Into<String>,
        body: Option<String>,
    ) -> Result<ApiRequest, ClientError> {
        let token = self.token.as_deref().ok_or(ClientError::MissingToken)?;
        Ok(ApiRequest::new(method, path, body)
            .with_header(VERSION_HEADER, API_VERSION)
            .with_header(AUTH_HEADER, format!("Bearer {token}")))
    }

    fn call<T: DeserializeOwned>(&mut self, request: ApiRequest) -> Result<T, ClientError> {
        let response = self.api.handle(&request);
        if response.status == 404 {
            return Err(ClientError::NotFound(response.body));
        }
        if !(200..300).contains(&response.status) {
            return Err(ClientError::Http(response.status, response.body));
        }
        let envelope: Envelope<T> =
            serde_json::from_str(&response.body).map_err(|e| ClientError::Decode(e.to_string()))?;
        match (envelope.data, envelope.error) {
            (Some(data), _) => Ok(data),
            (None, Some(message)) => Err(ClientError::Http(response.status, message)),
            (None, None) => Err(ClientError::Decode("response has no data".to_string())),
        }
    }

    fn call_v2<T: DeserializeOwned>(&mut self, request: ApiRequest) -> Result<T, ClientError> {
        let response = self.api.handle(&request);
        if !(200..300).contains(&response.status) {
            if let Ok(error) = serde_json::from_str::<ApiError>(&response.body) {
                return Err(ClientError::Api(Box::new(error)));
            }
            return Err(ClientError::Http(response.status, response.body));
        }
        let envelope: Envelope<T> =
            serde_json::from_str(&response.body).map_err(|e| ClientError::Decode(e.to_string()))?;
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
        let body = encode_body(workspace)?;
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
        let body = encode_body(draft)?;
        self.call(ApiRequest::new(Method::Post, "/api/pages", Some(body)))
    }

    pub fn update_page(&mut self, page: &Page) -> Result<Page, ClientError> {
        let body = encode_body(page)?;
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
        let body = encode_body(draft)?;
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
        let body = encode_body(share)?;
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

    // ---- v2 contract (spec-030) ------------------------------------------
    pub fn workspace_v2(&mut self) -> Result<Workspace, ClientError> {
        let request = self.v2_request(Method::Get, "/api/v2/workspace", None)?;
        self.call_v2(request)
    }

    pub fn replace_workspace_v2(
        &mut self,
        workspace: &Workspace,
    ) -> Result<Workspace, ClientError> {
        let body = encode_body(workspace)?;
        let request = self.v2_request(Method::Put, "/api/v2/workspace", Some(body))?;
        self.call_v2(request)
    }

    pub fn list_pages_v2(&mut self) -> Result<Vec<PageSummary>, ClientError> {
        let request = self.v2_request(Method::Get, "/api/v2/pages", None)?;
        self.call_v2(request)
    }

    pub fn get_page_v2(&mut self, id: &str) -> Result<Page, ClientError> {
        let request = self.v2_request(Method::Get, format!("/api/v2/pages/{id}"), None)?;
        self.call_v2(request)
    }

    pub fn create_page_v2(&mut self, draft: &PageDraft) -> Result<Page, ClientError> {
        let body = encode_body(draft)?;
        let request = self.v2_request(Method::Post, "/api/v2/pages", Some(body))?;
        self.call_v2(request)
    }

    pub fn update_page_v2(&mut self, page: &Page) -> Result<Page, ClientError> {
        let body = encode_body(page)?;
        let request = self.v2_request(
            Method::Patch,
            format!("/api/v2/pages/{}", page.id),
            Some(body),
        )?;
        self.call_v2(request)
    }

    pub fn delete_page_v2(&mut self, id: &str) -> Result<Vec<String>, ClientError> {
        let request = self.v2_request(Method::Delete, format!("/api/v2/pages/{id}"), None)?;
        self.call_v2(request)
    }

    pub fn page_versions_v2(&mut self, id: &str) -> Result<Vec<PageVersion>, ClientError> {
        let request = self.v2_request(Method::Get, format!("/api/v2/pages/{id}/versions"), None)?;
        self.call_v2(request)
    }

    pub fn search_v2(&mut self, query: &str) -> Result<Vec<SearchResult>, ClientError> {
        let encoded = url_encode(query);
        let request = self.v2_request(Method::Get, format!("/api/v2/search?q={encoded}"), None)?;
        self.call_v2(request)
    }

    pub fn list_members_v2(&mut self) -> Result<Vec<Member>, ClientError> {
        let request = self.v2_request(Method::Get, "/api/v2/members", None)?;
        self.call_v2(request)
    }

    pub fn invite_member_v2(&mut self, draft: &MemberDraft) -> Result<Member, ClientError> {
        let body = encode_body(draft)?;
        let request = self.v2_request(Method::Post, "/api/v2/members", Some(body))?;
        self.call_v2(request)
    }

    pub fn list_shares_v2(&mut self, page_id: Option<&str>) -> Result<Vec<Share>, ClientError> {
        let path = match page_id {
            Some(id) => format!("/api/v2/shares?page={}", url_encode(id)),
            None => "/api/v2/shares".to_string(),
        };
        let request = self.v2_request(Method::Get, path, None)?;
        self.call_v2(request)
    }

    pub fn set_share_v2(&mut self, share: &Share) -> Result<Share, ClientError> {
        let body = encode_body(share)?;
        let request = self.v2_request(Method::Put, "/api/v2/shares", Some(body))?;
        self.call_v2(request)
    }

    pub fn sync_v2(&mut self, since: u64) -> Result<SyncBatch, ClientError> {
        let request = self.v2_request(Method::Get, format!("/api/v2/sync?since={since}"), None)?;
        self.call_v2(request)
    }

    pub fn export_tree_v2(&mut self) -> Result<Vec<ExportFile>, ClientError> {
        let request = self.v2_request(Method::Get, "/api/v2/export", None)?;
        self.call_v2(request)
    }

    pub fn realtime_contract_v2(&mut self) -> Result<RealtimeContract, ClientError> {
        let request = self.v2_request(Method::Get, "/api/v2/realtime", None)?;
        self.call_v2(request)
    }
}
