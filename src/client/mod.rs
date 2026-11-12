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
    API_VERSION, AUTH_HEADER, ApiError, ApiRequest, Envelope, ListResponse, Method,
    RequestTransport, VERSION_HEADER, WorkspaceApi, url_encode,
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

mod blocks;
mod collab;
mod comments;
mod databases;
mod pages;
mod sharing;
mod views;
mod webhooks;
pub use retry::{RetryDecision, RetryPolicy};

pub struct Client<A> {
    api: A,
    token: Option<String>,
}

impl<A: RequestTransport> Client<A> {
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

    /// Borrow the transport (used by the desktop to start the remote worker).
    pub fn api(&self) -> &A {
        &self.api
    }

    /// Mutably borrow the transport.
    pub fn api_mut(&mut self) -> &mut A {
        &mut self.api
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
}
