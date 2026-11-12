//! Client side of the application.
//!
//! `Client<A>` is the only data access object the UI may use. It builds
//! `ApiRequest`s, checks status codes, and decodes JSON envelopes. Because it
//! is generic over `WorkspaceApi`, the same client works with the in-process
//! server and, later, a remote transport.

use crate::collab::{
    Change, ExportFile, Member, MemberDraft, PageVersion, RealtimeContract, Share, SyncBatch,
};
use crate::model::{
    BlockChildrenRequest, BlockDto, BlockInput, CommentDto, CommentInput, DataSource,
    DataSourceInput, Database, DatabaseInput, FileUpload, FileUploadInput, Page, PageDraft,
    PageProperty, PageSummary, SearchResult, UserDto, View, ViewInput, ViewQueryInput,
    ViewQueryResult, Workspace,
};
use crate::server::{
    API_VERSION, AUTH_HEADER, ApiError, ApiRequest, Envelope, ListResponse, Method, VERSION_HEADER,
    WorkspaceApi, url_encode,
};
use crate::webhooks::{
    WebhookDelivery, WebhookSubscription, WebhookSubscriptionInput, WebhookVerificationInput,
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
    Realtime(String),
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
            ClientError::Realtime(message) => write!(f, "real-time error: {message}"),
        }
    }
}

/// Serialize a request body, returning an explicit encode error instead of
/// sending an empty body (spec-021).
pub fn encode_body<T: Serialize>(value: &T) -> Result<String, ClientError> {
    serde_json::to_string(value).map_err(|e| ClientError::Encode(e.to_string()))
}

impl std::error::Error for ClientError {}

mod retry;

pub use retry::{RetryDecision, RetryPolicy};

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

    fn call<T: DeserializeOwned>(&mut self, mut request: ApiRequest) -> Result<T, ClientError> {
        if let Some(token) = &self.token {
            request = request.with_header(AUTH_HEADER, format!("Bearer {token}"));
        }
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

    fn paged_path(path: &str, page_size: u16, cursor: Option<&str>) -> String {
        let separator = if path.contains('?') { '&' } else { '?' };
        let mut result = format!("{path}{separator}page_size={page_size}");
        if let Some(cursor) = cursor {
            result.push_str("&start_cursor=");
            result.push_str(&url_encode(cursor));
        }
        result
    }

    fn list_page_method_v2<T: DeserializeOwned>(
        &mut self,
        method: Method,
        path: &str,
        page_size: u16,
        cursor: Option<&str>,
    ) -> Result<ListResponse<T>, ClientError> {
        let request = self.v2_request(method, Self::paged_path(path, page_size, cursor), None)?;
        self.call_v2(request)
    }

    fn list_page_v2<T: DeserializeOwned>(
        &mut self,
        path: &str,
        page_size: u16,
        cursor: Option<&str>,
    ) -> Result<ListResponse<T>, ClientError> {
        self.list_page_method_v2(Method::Get, path, page_size, cursor)
    }

    fn collect_all_method_v2<T: DeserializeOwned>(
        &mut self,
        method: Method,
        path: &str,
        page_size: u16,
    ) -> Result<Vec<T>, ClientError> {
        let mut results = Vec::new();
        let mut cursor: Option<String> = None;
        let mut pages = 0usize;
        loop {
            let page: ListResponse<T> =
                self.list_page_method_v2(method, path, page_size, cursor.as_deref())?;
            results.extend(page.results);
            if !page.has_more {
                break;
            }
            cursor = page.next_cursor;
            if cursor.is_none() {
                return Err(ClientError::Decode(
                    "list said has_more but returned no cursor".to_string(),
                ));
            }
            pages += 1;
            if pages > 1000 {
                return Err(ClientError::Decode(
                    "pagination did not terminate".to_string(),
                ));
            }
        }
        Ok(results)
    }

    fn collect_all_v2<T: DeserializeOwned>(
        &mut self,
        path: &str,
        page_size: u16,
    ) -> Result<Vec<T>, ClientError> {
        self.collect_all_method_v2(Method::Get, path, page_size)
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
        self.collect_all_v2("/api/v2/pages", 100)
    }

    pub fn list_pages_page_v2(
        &mut self,
        page_size: u16,
        cursor: Option<&str>,
    ) -> Result<ListResponse<PageSummary>, ClientError> {
        self.list_page_v2("/api/v2/pages", page_size, cursor)
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
        let path = format!("/api/v2/pages/{id}/versions");
        self.collect_all_v2(&path, 100)
    }

    pub fn page_versions_page_v2(
        &mut self,
        id: &str,
        page_size: u16,
        cursor: Option<&str>,
    ) -> Result<ListResponse<PageVersion>, ClientError> {
        let path = format!("/api/v2/pages/{id}/versions");
        self.list_page_v2(&path, page_size, cursor)
    }

    pub fn search_v2(&mut self, query: &str) -> Result<Vec<SearchResult>, ClientError> {
        let path = format!("/api/v2/search?q={}", url_encode(query));
        self.collect_all_v2(&path, 100)
    }

    pub fn search_page_v2(
        &mut self,
        query: &str,
        page_size: u16,
        cursor: Option<&str>,
    ) -> Result<ListResponse<SearchResult>, ClientError> {
        let path = format!("/api/v2/search?q={}", url_encode(query));
        self.list_page_v2(&path, page_size, cursor)
    }

    pub fn list_members_v2(&mut self) -> Result<Vec<Member>, ClientError> {
        self.collect_all_v2("/api/v2/members", 100)
    }

    pub fn list_members_page_v2(
        &mut self,
        page_size: u16,
        cursor: Option<&str>,
    ) -> Result<ListResponse<Member>, ClientError> {
        self.list_page_v2("/api/v2/members", page_size, cursor)
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
        self.collect_all_v2(&path, 100)
    }

    pub fn list_shares_page_v2(
        &mut self,
        page_id: Option<&str>,
        page_size: u16,
        cursor: Option<&str>,
    ) -> Result<ListResponse<Share>, ClientError> {
        let path = match page_id {
            Some(id) => format!("/api/v2/shares?page={}", url_encode(id)),
            None => "/api/v2/shares".to_string(),
        };
        self.list_page_v2(&path, page_size, cursor)
    }

    pub fn set_share_v2(&mut self, share: &Share) -> Result<Share, ClientError> {
        let body = encode_body(share)?;
        let request = self.v2_request(Method::Put, "/api/v2/shares", Some(body))?;
        self.call_v2(request)
    }

    pub fn sync_v2(&mut self, since: u64) -> Result<Vec<Change>, ClientError> {
        let path = format!("/api/v2/sync?since={since}");
        self.collect_all_v2(&path, 100)
    }

    pub fn sync_page_v2(
        &mut self,
        since: u64,
        page_size: u16,
        cursor: Option<&str>,
    ) -> Result<ListResponse<Change>, ClientError> {
        let path = format!("/api/v2/sync?since={since}");
        self.list_page_v2(&path, page_size, cursor)
    }

    pub fn export_tree_v2(&mut self) -> Result<Vec<ExportFile>, ClientError> {
        let request = self.v2_request(Method::Get, "/api/v2/export", None)?;
        self.call_v2(request)
    }

    pub fn realtime_contract_v2(&mut self) -> Result<RealtimeContract, ClientError> {
        let request = self.v2_request(Method::Get, "/api/v2/realtime", None)?;
        self.call_v2(request)
    }

    pub fn get_block_v2(&mut self, id: &str) -> Result<BlockDto, ClientError> {
        let request = self.v2_request(Method::Get, format!("/api/v2/blocks/{id}"), None)?;
        self.call_v2(request)
    }

    pub fn list_block_children_page_v2(
        &mut self,
        id: &str,
        page_size: u16,
        cursor: Option<&str>,
    ) -> Result<ListResponse<BlockDto>, ClientError> {
        let path = format!("/api/v2/blocks/{id}/children");
        self.list_page_v2(&path, page_size, cursor)
    }

    pub fn list_block_children_v2(&mut self, id: &str) -> Result<Vec<BlockDto>, ClientError> {
        let path = format!("/api/v2/blocks/{id}/children");
        self.collect_all_v2(&path, 100)
    }

    pub fn append_block_children_v2(
        &mut self,
        id: &str,
        children: &[BlockInput],
    ) -> Result<Vec<BlockDto>, ClientError> {
        let body = encode_body(&BlockChildrenRequest {
            children: children.to_vec(),
        })?;
        let request = self.v2_request(
            Method::Patch,
            format!("/api/v2/blocks/{id}/children"),
            Some(body),
        )?;
        self.call_v2(request)
    }

    pub fn update_block_v2(
        &mut self,
        id: &str,
        input: &BlockInput,
    ) -> Result<BlockDto, ClientError> {
        let body = encode_body(input)?;
        let request = self.v2_request(Method::Patch, format!("/api/v2/blocks/{id}"), Some(body))?;
        self.call_v2(request)
    }

    pub fn delete_block_v2(&mut self, id: &str) -> Result<BlockDto, ClientError> {
        let request = self.v2_request(Method::Delete, format!("/api/v2/blocks/{id}"), None)?;
        self.call_v2(request)
    }

    pub fn list_databases_v2(&mut self) -> Result<Vec<Database>, ClientError> {
        let request = self.v2_request(Method::Get, "/api/v2/databases", None)?;
        self.call_v2(request)
    }

    pub fn create_database_v2(&mut self, input: &DatabaseInput) -> Result<Database, ClientError> {
        let body = encode_body(input)?;
        let request = self.v2_request(Method::Post, "/api/v2/databases", Some(body))?;
        self.call_v2(request)
    }

    pub fn get_database_v2(&mut self, id: &str) -> Result<Database, ClientError> {
        let request = self.v2_request(Method::Get, format!("/api/v2/databases/{id}"), None)?;
        self.call_v2(request)
    }

    pub fn update_database_v2(
        &mut self,
        id: &str,
        input: &DatabaseInput,
    ) -> Result<Database, ClientError> {
        let body = encode_body(input)?;
        let request =
            self.v2_request(Method::Patch, format!("/api/v2/databases/{id}"), Some(body))?;
        self.call_v2(request)
    }

    pub fn delete_database_v2(&mut self, id: &str) -> Result<Database, ClientError> {
        let request = self.v2_request(Method::Delete, format!("/api/v2/databases/{id}"), None)?;
        self.call_v2(request)
    }

    pub fn list_data_sources_v2(
        &mut self,
        database_id: &str,
    ) -> Result<Vec<DataSource>, ClientError> {
        let request = self.v2_request(
            Method::Get,
            format!("/api/v2/databases/{database_id}/data_sources"),
            None,
        )?;
        self.call_v2(request)
    }

    pub fn create_data_source_v2(
        &mut self,
        database_id: &str,
        input: &DataSourceInput,
    ) -> Result<DataSource, ClientError> {
        let body = encode_body(input)?;
        let request = self.v2_request(
            Method::Post,
            format!("/api/v2/databases/{database_id}/data_sources"),
            Some(body),
        )?;
        self.call_v2(request)
    }

    pub fn get_data_source_v2(&mut self, id: &str) -> Result<DataSource, ClientError> {
        let request = self.v2_request(Method::Get, format!("/api/v2/data_sources/{id}"), None)?;
        self.call_v2(request)
    }

    pub fn update_data_source_v2(
        &mut self,
        id: &str,
        input: &DataSourceInput,
    ) -> Result<DataSource, ClientError> {
        let body = encode_body(input)?;
        let request = self.v2_request(
            Method::Patch,
            format!("/api/v2/data_sources/{id}"),
            Some(body),
        )?;
        self.call_v2(request)
    }

    pub fn query_data_source_page_v2(
        &mut self,
        id: &str,
        page_size: u16,
        cursor: Option<&str>,
    ) -> Result<ListResponse<Page>, ClientError> {
        let path = format!("/api/v2/data_sources/{id}/query");
        self.list_page_method_v2(Method::Post, &path, page_size, cursor)
    }

    pub fn query_data_source_v2(&mut self, id: &str) -> Result<Vec<Page>, ClientError> {
        let path = format!("/api/v2/data_sources/{id}/query");
        self.collect_all_method_v2(Method::Post, &path, 100)
    }

    pub fn page_property_items_page_v2(
        &mut self,
        page_id: &str,
        property_id: &str,
        page_size: u16,
        cursor: Option<&str>,
    ) -> Result<ListResponse<PageProperty>, ClientError> {
        let path = format!("/api/v2/pages/{page_id}/properties/{property_id}");
        self.list_page_v2(&path, page_size, cursor)
    }

    pub fn page_property_items_v2(
        &mut self,
        page_id: &str,
        property_id: &str,
    ) -> Result<Vec<PageProperty>, ClientError> {
        let path = format!("/api/v2/pages/{page_id}/properties/{property_id}");
        self.collect_all_v2(&path, 100)
    }

    pub fn list_views_v2(&mut self, data_source: Option<&str>) -> Result<Vec<View>, ClientError> {
        let path = match data_source {
            Some(id) => format!("/api/v2/views?data_source={}", url_encode(id)),
            None => "/api/v2/views".to_string(),
        };
        let request = self.v2_request(Method::Get, path, None)?;
        self.call_v2(request)
    }

    pub fn create_view_v2(&mut self, input: &ViewInput) -> Result<View, ClientError> {
        let body = encode_body(input)?;
        let request = self.v2_request(Method::Post, "/api/v2/views", Some(body))?;
        self.call_v2(request)
    }

    pub fn get_view_v2(&mut self, id: &str) -> Result<View, ClientError> {
        let request = self.v2_request(Method::Get, format!("/api/v2/views/{id}"), None)?;
        self.call_v2(request)
    }

    pub fn update_view_v2(&mut self, id: &str, input: &ViewInput) -> Result<View, ClientError> {
        let body = encode_body(input)?;
        let request = self.v2_request(Method::Patch, format!("/api/v2/views/{id}"), Some(body))?;
        self.call_v2(request)
    }

    pub fn delete_view_v2(&mut self, id: &str) -> Result<String, ClientError> {
        let request = self.v2_request(Method::Delete, format!("/api/v2/views/{id}"), None)?;
        self.call_v2(request)
    }

    pub fn query_view_v2(
        &mut self,
        id: &str,
        input: Option<&ViewQueryInput>,
        page_size: u16,
        cursor: Option<&str>,
    ) -> Result<ViewQueryResult, ClientError> {
        let body = match input {
            Some(input) => Some(encode_body(input)?),
            None => None,
        };
        let path = Self::paged_path(&format!("/api/v2/views/{id}/query"), page_size, cursor);
        let request = self.v2_request(Method::Post, path, body)?;
        self.call_v2(request)
    }

    fn comments_path(page: Option<&str>, block: Option<&str>) -> String {
        match (page, block) {
            (Some(page), _) => format!("/api/v2/comments?page={}", url_encode(page)),
            (None, Some(block)) => format!("/api/v2/comments?block={}", url_encode(block)),
            (None, None) => "/api/v2/comments".to_string(),
        }
    }

    pub fn list_comments_page_v2(
        &mut self,
        page: Option<&str>,
        block: Option<&str>,
        page_size: u16,
        cursor: Option<&str>,
    ) -> Result<ListResponse<CommentDto>, ClientError> {
        let path = Self::comments_path(page, block);
        self.list_page_v2(&path, page_size, cursor)
    }

    pub fn list_comments_v2(
        &mut self,
        page: Option<&str>,
        block: Option<&str>,
    ) -> Result<Vec<CommentDto>, ClientError> {
        let path = Self::comments_path(page, block);
        self.collect_all_v2(&path, 100)
    }

    pub fn create_comment_v2(&mut self, input: &CommentInput) -> Result<CommentDto, ClientError> {
        let body = encode_body(input)?;
        let request = self.v2_request(Method::Post, "/api/v2/comments", Some(body))?;
        self.call_v2(request)
    }

    pub fn get_comment_v2(&mut self, id: &str) -> Result<CommentDto, ClientError> {
        let request = self.v2_request(Method::Get, format!("/api/v2/comments/{id}"), None)?;
        self.call_v2(request)
    }

    pub fn update_comment_v2(
        &mut self,
        id: &str,
        input: &CommentInput,
    ) -> Result<CommentDto, ClientError> {
        let body = encode_body(input)?;
        let request =
            self.v2_request(Method::Patch, format!("/api/v2/comments/{id}"), Some(body))?;
        self.call_v2(request)
    }

    pub fn delete_comment_v2(&mut self, id: &str) -> Result<CommentDto, ClientError> {
        let request = self.v2_request(Method::Delete, format!("/api/v2/comments/{id}"), None)?;
        self.call_v2(request)
    }

    pub fn list_users_v2(&mut self) -> Result<Vec<UserDto>, ClientError> {
        let request = self.v2_request(Method::Get, "/api/v2/users", None)?;
        self.call_v2(request)
    }

    pub fn get_user_v2(&mut self, id: &str) -> Result<UserDto, ClientError> {
        let request = self.v2_request(Method::Get, format!("/api/v2/users/{id}"), None)?;
        self.call_v2(request)
    }

    pub fn create_file_upload_v2(
        &mut self,
        input: &FileUploadInput,
    ) -> Result<FileUpload, ClientError> {
        let body = encode_body(input)?;
        let request = self.v2_request(Method::Post, "/api/v2/file_uploads", Some(body))?;
        self.call_v2(request)
    }

    pub fn send_file_upload_v2(
        &mut self,
        id: &str,
        content: &str,
    ) -> Result<FileUpload, ClientError> {
        let request = self.v2_request(
            Method::Post,
            format!("/api/v2/file_uploads/{id}/send"),
            Some(content.to_string()),
        )?;
        self.call_v2(request)
    }

    pub fn complete_file_upload_v2(&mut self, id: &str) -> Result<FileUpload, ClientError> {
        let request = self.v2_request(
            Method::Post,
            format!("/api/v2/file_uploads/{id}/complete"),
            None,
        )?;
        self.call_v2(request)
    }

    pub fn get_file_upload_v2(&mut self, id: &str) -> Result<FileUpload, ClientError> {
        let request = self.v2_request(Method::Get, format!("/api/v2/file_uploads/{id}"), None)?;
        self.call_v2(request)
    }

    pub fn list_file_uploads_page_v2(
        &mut self,
        page_size: u16,
        cursor: Option<&str>,
    ) -> Result<ListResponse<FileUpload>, ClientError> {
        self.list_page_v2("/api/v2/file_uploads", page_size, cursor)
    }

    pub fn list_file_uploads_v2(&mut self) -> Result<Vec<FileUpload>, ClientError> {
        self.collect_all_v2("/api/v2/file_uploads", 100)
    }

    /// Sync the in-process CRDT document for a page.
    pub fn crdt_sync(
        &mut self,
        page_id: &str,
        state_vector: Option<&[u8]>,
    ) -> Result<(Vec<u8>, Vec<u8>), ClientError> {
        self.api
            .crdt_sync(page_id, state_vector)
            .map_err(ClientError::Realtime)
    }

    /// Apply a CRDT update through the in-process transport.
    pub fn apply_crdt_update(
        &mut self,
        page_id: &str,
        update: &[u8],
        actor: &str,
    ) -> Result<Vec<u8>, ClientError> {
        self.api
            .apply_crdt_update(page_id, update, actor)
            .map_err(ClientError::Realtime)
    }

    /// Subscribe to the in-process real-time hub, if this transport has one.
    pub fn subscribe_realtime(
        &self,
    ) -> Option<tokio::sync::broadcast::Receiver<crate::server::realtime::ServerMessage>> {
        self.api.subscribe_realtime()
    }

    /// Publish cursor/selection awareness through the in-process transport.
    pub fn publish_awareness(
        &mut self,
        awareness: &crate::server::realtime::Awareness,
    ) -> Result<(), ClientError> {
        self.api
            .publish_awareness(awareness.clone())
            .map_err(ClientError::Realtime)
    }

    pub fn list_webhooks_v2(&mut self) -> Result<Vec<WebhookSubscription>, ClientError> {
        let request = self.v2_request(Method::Get, "/api/v2/webhooks", None)?;
        self.call_v2(request)
    }

    pub fn create_webhook_v2(
        &mut self,
        input: &WebhookSubscriptionInput,
    ) -> Result<WebhookSubscription, ClientError> {
        let body = encode_body(input)?;
        let request = self.v2_request(Method::Post, "/api/v2/webhooks", Some(body))?;
        self.call_v2(request)
    }

    pub fn verify_webhook_v2(&mut self, id: &str, token: &str) -> Result<String, ClientError> {
        let body = encode_body(&WebhookVerificationInput {
            verification_token: token.to_string(),
        })?;
        let request = self.v2_request(
            Method::Post,
            format!("/api/v2/webhooks/{id}/verify"),
            Some(body),
        )?;
        self.call_v2(request)
    }

    pub fn delete_webhook_v2(&mut self, id: &str) -> Result<String, ClientError> {
        let request = self.v2_request(Method::Delete, format!("/api/v2/webhooks/{id}"), None)?;
        self.call_v2(request)
    }

    pub fn list_webhook_deliveries_v2(&mut self) -> Result<Vec<WebhookDelivery>, ClientError> {
        let request = self.v2_request(Method::Get, "/api/v2/webhook_deliveries", None)?;
        self.call_v2(request)
    }

    pub fn retry_webhook_deliveries_v2(&mut self) -> Result<Vec<WebhookDelivery>, ClientError> {
        let request = self.v2_request(Method::Post, "/api/v2/webhook_deliveries/retry", None)?;
        self.call_v2(request)
    }
}
