//! v2 contract foundation: versioning, structured errors, capabilities and
//! request IDs (spec-030).
//!
//! This module is transport-neutral. It knows nothing about egui, sockets or
//! files; the HTTP adapter only copies headers in and out.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The version of our own v2 contract. This is **not** `Notion-Version`; the
/// project must not claim wire compatibility with Notion.
pub const API_VERSION: &str = "2026-11-08";

/// Versions accepted by `/api/v2`.
pub const SUPPORTED_VERSIONS: &[&str] = &[API_VERSION];

/// Required header for `/api/v2` requests.
pub const VERSION_HEADER: &str = "not-notion-version";

/// Standard authorization header.
pub const AUTH_HEADER: &str = "authorization";

/// Demo owner token for the local loopback server. A production deployment
/// replaces this with self-hosted sessions (ADR-003).
pub const OWNER_TOKEN: &str = "owner-token";

pub const CODE_MISSING_VERSION: &str = "missing_version";
pub const CODE_UNSUPPORTED_VERSION: &str = "unsupported_version";
pub const CODE_INVALID_JSON: &str = "invalid_json";
pub const CODE_VALIDATION_ERROR: &str = "validation_error";
pub const CODE_UNAUTHORIZED: &str = "unauthorized";
pub const CODE_RESTRICTED_RESOURCE: &str = "restricted_resource";
pub const CODE_NOT_FOUND: &str = "object_not_found";
pub const CODE_INVALID_CURSOR: &str = "invalid_cursor";
pub const CODE_INTERNAL: &str = "internal_server_error";
pub const CODE_RATE_LIMITED: &str = "rate_limited";
pub const CODE_SERVICE_OVERLOAD: &str = "service_overload";
pub const CODE_SERVICE_UNAVAILABLE: &str = "service_unavailable";
pub const CODE_CONFLICT: &str = "conflict_error";

/// Structured error body for `/api/v2` responses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiError {
    pub object: String,
    pub status: u16,
    pub code: String,
    pub message: String,
    pub request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_data: Option<serde_json::Value>,
}

impl ApiError {
    pub fn new(
        status: u16,
        code: impl Into<String>,
        message: impl Into<String>,
        request_id: impl Into<String>,
    ) -> Self {
        Self {
            object: "error".to_string(),
            status,
            code: code.into(),
            message: message.into(),
            request_id: request_id.into(),
            additional_data: None,
        }
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.additional_data = Some(data);
        self
    }
}

/// Standard v2 list envelope (spec-031).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListResponse<T> {
    pub object: String,
    pub r#type: String,
    pub results: Vec<T>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

impl<T> ListResponse<T> {
    pub fn new(
        type_name: impl Into<String>,
        results: Vec<T>,
        has_more: bool,
        next_cursor: Option<String>,
    ) -> Self {
        Self {
            object: "list".to_string(),
            r#type: type_name.into(),
            results,
            has_more,
            next_cursor,
        }
    }
}

/// Capabilities gate what a connection can do, separately from the per-page
/// `Access` level (spec-030).
///
/// Content and comment capabilities are independent. The three user-info
/// variants are **levels**, not additive grants; a connection should hold
/// exactly one. Spec-035 enforces that when user objects are introduced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Capability {
    ReadContent,
    UpdateContent,
    InsertContent,
    ReadComments,
    InsertComments,
    UserInfoNone,
    UserInfoWithoutEmail,
    UserInfoWithEmail,
}

impl Capability {
    pub fn as_str(&self) -> &'static str {
        match self {
            Capability::ReadContent => "read_content",
            Capability::UpdateContent => "update_content",
            Capability::InsertContent => "insert_content",
            Capability::ReadComments => "read_comments",
            Capability::InsertComments => "insert_comments",
            Capability::UserInfoNone => "user_info_none",
            Capability::UserInfoWithoutEmail => "user_info_without_email",
            Capability::UserInfoWithEmail => "user_info_with_email",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Capability::ReadContent => "Read content",
            Capability::UpdateContent => "Update content",
            Capability::InsertContent => "Insert content",
            Capability::ReadComments => "Read comments",
            Capability::InsertComments => "Insert comments",
            Capability::UserInfoNone => "No user information",
            Capability::UserInfoWithoutEmail => "User information without email",
            Capability::UserInfoWithEmail => "User information with email",
        }
    }

    /// Independent content/comment capabilities plus the highest user-info
    /// level. Use [`Self::user_info_levels`] when presenting the alternatives.
    pub fn all() -> Vec<Self> {
        vec![
            Capability::ReadContent,
            Capability::UpdateContent,
            Capability::InsertContent,
            Capability::ReadComments,
            Capability::InsertComments,
            Capability::UserInfoWithEmail,
        ]
    }

    pub fn user_info_levels() -> Vec<Self> {
        vec![
            Capability::UserInfoNone,
            Capability::UserInfoWithoutEmail,
            Capability::UserInfoWithEmail,
        ]
    }
}

/// A connection (token + member + workspace + capabilities).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Connection {
    pub id: String,
    pub token: String,
    pub member_id: String,
    pub workspace_id: String,
    pub capabilities: BTreeSet<Capability>,
}

impl Connection {
    pub fn new(
        id: impl Into<String>,
        token: impl Into<String>,
        member_id: impl Into<String>,
        workspace_id: impl Into<String>,
        capabilities: impl IntoIterator<Item = Capability>,
    ) -> Self {
        Self {
            id: id.into(),
            token: token.into(),
            member_id: member_id.into(),
            workspace_id: workspace_id.into(),
            capabilities: capabilities.into_iter().collect(),
        }
    }

    /// The local demo owner: every capability, one stable token.
    pub fn owner(member_id: impl Into<String>, workspace_id: impl Into<String>) -> Self {
        Self::new(
            "connection-owner",
            OWNER_TOKEN,
            member_id,
            workspace_id,
            Capability::all(),
        )
    }

    pub fn has(&self, capability: Capability) -> bool {
        self.capabilities.contains(&capability)
    }
}

/// One in-memory request-log entry. It makes the request ID observable in
/// tests without depending on stderr.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestLogEntry {
    pub request_id: String,
    pub method: String,
    pub path: String,
    pub status: u16,
}

pub fn new_request_id() -> String {
    format!("req_{}", uuid::Uuid::new_v4())
}

pub fn version_supported(version: &str) -> bool {
    SUPPORTED_VERSIONS.contains(&version)
}

/// Parse `Bearer <token>` from an authorization header value.
pub fn parse_bearer(value: &str) -> Option<&str> {
    let mut parts = value.splitn(2, ' ');
    let scheme = parts.next()?;
    if !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }
    parts
        .next()
        .map(str::trim)
        .filter(|token| !token.is_empty())
}
