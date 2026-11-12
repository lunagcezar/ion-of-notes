//! Webhook subscriptions, event payloads, signatures and aggregation.
//!
//! This module is pure: it has no sockets or filesystem access. The server
//! owns storage and delivery; a production deployment can replace the local
//! sender with a TLS-capable adapter behind the same types.

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::net::IpAddr;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WebhookEventType {
    #[serde(rename = "page.created")]
    PageCreated,
    #[serde(rename = "page.content_updated")]
    PageContentUpdated,
    #[serde(rename = "page.properties_updated")]
    PagePropertiesUpdated,
    #[serde(rename = "page.moved")]
    PageMoved,
    #[serde(rename = "page.deleted")]
    PageDeleted,
    #[serde(rename = "page.undeleted")]
    PageUndeleted,
    #[serde(rename = "page.locked")]
    PageLocked,
    #[serde(rename = "page.unlocked")]
    PageUnlocked,
    #[serde(rename = "page.transcription_block_transcript_deleted")]
    PageTranscriptionBlockTranscriptDeleted,
    #[serde(rename = "database.created")]
    DatabaseCreated,
    #[serde(rename = "database.content_updated")]
    DatabaseContentUpdated,
    #[serde(rename = "database.schema_updated")]
    DatabaseSchemaUpdated,
    #[serde(rename = "database.moved")]
    DatabaseMoved,
    #[serde(rename = "database.deleted")]
    DatabaseDeleted,
    #[serde(rename = "database.undeleted")]
    DatabaseUndeleted,
    #[serde(rename = "data_source.created")]
    DataSourceCreated,
    #[serde(rename = "data_source.content_updated")]
    DataSourceContentUpdated,
    #[serde(rename = "data_source.schema_updated")]
    DataSourceSchemaUpdated,
    #[serde(rename = "data_source.moved")]
    DataSourceMoved,
    #[serde(rename = "data_source.deleted")]
    DataSourceDeleted,
    #[serde(rename = "data_source.undeleted")]
    DataSourceUndeleted,
    #[serde(rename = "comment.created")]
    CommentCreated,
    #[serde(rename = "comment.updated")]
    CommentUpdated,
    #[serde(rename = "comment.deleted")]
    CommentDeleted,
    #[serde(rename = "file_upload.created")]
    FileUploadCreated,
    #[serde(rename = "file_upload.completed")]
    FileUploadCompleted,
    #[serde(rename = "file_upload.expired")]
    FileUploadExpired,
    #[serde(rename = "file_upload.upload_failed")]
    FileUploadUploadFailed,
    #[serde(rename = "view.created")]
    ViewCreated,
    #[serde(rename = "view.updated")]
    ViewUpdated,
    #[serde(rename = "view.deleted")]
    ViewDeleted,
}

impl WebhookEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            WebhookEventType::PageCreated => "page.created",
            WebhookEventType::PageContentUpdated => "page.content_updated",
            WebhookEventType::PagePropertiesUpdated => "page.properties_updated",
            WebhookEventType::PageMoved => "page.moved",
            WebhookEventType::PageDeleted => "page.deleted",
            WebhookEventType::PageUndeleted => "page.undeleted",
            WebhookEventType::PageLocked => "page.locked",
            WebhookEventType::PageUnlocked => "page.unlocked",
            WebhookEventType::PageTranscriptionBlockTranscriptDeleted => {
                "page.transcription_block_transcript_deleted"
            }
            WebhookEventType::DatabaseCreated => "database.created",
            WebhookEventType::DatabaseContentUpdated => "database.content_updated",
            WebhookEventType::DatabaseSchemaUpdated => "database.schema_updated",
            WebhookEventType::DatabaseMoved => "database.moved",
            WebhookEventType::DatabaseDeleted => "database.deleted",
            WebhookEventType::DatabaseUndeleted => "database.undeleted",
            WebhookEventType::DataSourceCreated => "data_source.created",
            WebhookEventType::DataSourceContentUpdated => "data_source.content_updated",
            WebhookEventType::DataSourceSchemaUpdated => "data_source.schema_updated",
            WebhookEventType::DataSourceMoved => "data_source.moved",
            WebhookEventType::DataSourceDeleted => "data_source.deleted",
            WebhookEventType::DataSourceUndeleted => "data_source.undeleted",
            WebhookEventType::CommentCreated => "comment.created",
            WebhookEventType::CommentUpdated => "comment.updated",
            WebhookEventType::CommentDeleted => "comment.deleted",
            WebhookEventType::FileUploadCreated => "file_upload.created",
            WebhookEventType::FileUploadCompleted => "file_upload.completed",
            WebhookEventType::FileUploadExpired => "file_upload.expired",
            WebhookEventType::FileUploadUploadFailed => "file_upload.upload_failed",
            WebhookEventType::ViewCreated => "view.created",
            WebhookEventType::ViewUpdated => "view.updated",
            WebhookEventType::ViewDeleted => "view.deleted",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "page.created" => WebhookEventType::PageCreated,
            "page.content_updated" => WebhookEventType::PageContentUpdated,
            "page.properties_updated" => WebhookEventType::PagePropertiesUpdated,
            "page.moved" => WebhookEventType::PageMoved,
            "page.deleted" => WebhookEventType::PageDeleted,
            "page.undeleted" => WebhookEventType::PageUndeleted,
            "page.locked" => WebhookEventType::PageLocked,
            "page.unlocked" => WebhookEventType::PageUnlocked,
            "page.transcription_block_transcript_deleted" => {
                WebhookEventType::PageTranscriptionBlockTranscriptDeleted
            }
            "database.created" => WebhookEventType::DatabaseCreated,
            "database.content_updated" => WebhookEventType::DatabaseContentUpdated,
            "database.schema_updated" => WebhookEventType::DatabaseSchemaUpdated,
            "database.moved" => WebhookEventType::DatabaseMoved,
            "database.deleted" => WebhookEventType::DatabaseDeleted,
            "database.undeleted" => WebhookEventType::DatabaseUndeleted,
            "data_source.created" => WebhookEventType::DataSourceCreated,
            "data_source.content_updated" => WebhookEventType::DataSourceContentUpdated,
            "data_source.schema_updated" => WebhookEventType::DataSourceSchemaUpdated,
            "data_source.moved" => WebhookEventType::DataSourceMoved,
            "data_source.deleted" => WebhookEventType::DataSourceDeleted,
            "data_source.undeleted" => WebhookEventType::DataSourceUndeleted,
            "comment.created" => WebhookEventType::CommentCreated,
            "comment.updated" => WebhookEventType::CommentUpdated,
            "comment.deleted" => WebhookEventType::CommentDeleted,
            "file_upload.created" => WebhookEventType::FileUploadCreated,
            "file_upload.completed" => WebhookEventType::FileUploadCompleted,
            "file_upload.expired" => WebhookEventType::FileUploadExpired,
            "file_upload.upload_failed" => WebhookEventType::FileUploadUploadFailed,
            "view.created" => WebhookEventType::ViewCreated,
            "view.updated" => WebhookEventType::ViewUpdated,
            "view.deleted" => WebhookEventType::ViewDeleted,
            _ => return None,
        })
    }

    /// High-frequency events are batched by the public contract.
    pub fn is_aggregated(&self) -> bool {
        matches!(
            self,
            WebhookEventType::PageCreated
                | WebhookEventType::PageContentUpdated
                | WebhookEventType::PagePropertiesUpdated
                | WebhookEventType::PageMoved
                | WebhookEventType::PageDeleted
                | WebhookEventType::PageUndeleted
                | WebhookEventType::DatabaseCreated
                | WebhookEventType::DatabaseContentUpdated
                | WebhookEventType::DatabaseSchemaUpdated
                | WebhookEventType::DatabaseMoved
                | WebhookEventType::DatabaseDeleted
                | WebhookEventType::DatabaseUndeleted
                | WebhookEventType::DataSourceCreated
                | WebhookEventType::DataSourceContentUpdated
                | WebhookEventType::DataSourceSchemaUpdated
                | WebhookEventType::DataSourceMoved
                | WebhookEventType::DataSourceDeleted
                | WebhookEventType::DataSourceUndeleted
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Pending,
    Active,
    Paused,
    Deleted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebhookSubscription {
    pub id: String,
    pub url: String,
    pub events: Vec<WebhookEventType>,
    pub verification_token: String,
    pub status: SubscriptionStatus,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebhookSubscriptionInput {
    pub url: String,
    pub events: Vec<WebhookEventType>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebhookVerificationInput {
    pub verification_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorRef {
    pub id: String,
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityRef {
    pub id: String,
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebhookEvent {
    pub id: String,
    pub timestamp: String,
    pub workspace_id: String,
    pub subscription_id: String,
    pub integration_id: String,
    pub r#type: WebhookEventType,
    pub authors: Vec<ActorRef>,
    pub accessible_by: Vec<ActorRef>,
    pub attempt_number: u8,
    pub entity: EntityRef,
    /// Event-specific JSON data, kept as text so the state stays Eq.
    pub data: String,
}

impl WebhookEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        workspace_id: impl Into<String>,
        subscription_id: impl Into<String>,
        event_type: WebhookEventType,
        entity: EntityRef,
        actor: impl Into<String>,
        data: impl Into<String>,
    ) -> Self {
        let actor = ActorRef {
            id: actor.into(),
            r#type: "person".to_string(),
        };
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            workspace_id: workspace_id.into(),
            subscription_id: subscription_id.into(),
            integration_id: "ion-of-notes".to_string(),
            r#type: event_type,
            authors: vec![actor.clone()],
            accessible_by: vec![actor],
            attempt_number: 0,
            entity,
            data: data.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebhookDelivery {
    pub id: String,
    pub subscription_id: String,
    pub event_id: String,
    #[serde(default)]
    pub payload: String,
    pub attempts: u8,
    pub last_status: Option<u16>,
    pub last_error: Option<String>,
    pub delivered: bool,
    pub next_attempt_at: Option<String>,
    /// True after the retry budget is exhausted; kept for inspection/requeue.
    #[serde(default)]
    pub dead_lettered: bool,
}

/// Counts exposed by `/metrics` and the delivery API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WebhookStats {
    pub pending: usize,
    pub dead_letters: usize,
    pub deliveries: usize,
}

/// Body for `POST /api/v2/webhook_deliveries/requeue`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebhookRequeueInput {
    pub id: String,
}

/// Group aggregated events by subscription and entity, keeping the latest.
pub fn aggregate_events(events: Vec<WebhookEvent>) -> Vec<WebhookEvent> {
    let mut result: Vec<WebhookEvent> = Vec::new();
    for event in events {
        if event.r#type.is_aggregated()
            && let Some(existing) = result.iter_mut().find(|existing| {
                existing.subscription_id == event.subscription_id
                    && existing.entity.id == event.entity.id
                    && existing.r#type == event.r#type
            })
        {
            *existing = event;
        } else {
            result.push(event);
        }
    }
    result
}

pub fn sign(secret: &str, body: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts keys of any length");
    mac.update(body.as_bytes());
    format!("sha256={}", to_hex(&mac.finalize().into_bytes()))
}

pub fn verify(secret: &str, body: &str, signature: &str) -> bool {
    let expected = sign(secret, body);
    constant_time_eq(expected.as_bytes(), signature.as_bytes())
}

fn to_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0u8;
    for (left, right) in left.iter().zip(right) {
        difference |= left ^ right;
    }
    difference == 0
}

/// Accept HTTPS anywhere public; allow loopback HTTP only for explicit
/// development. Private, loopback and link-local HTTPS targets are rejected.
pub fn is_allowed_url(url: &str, allow_insecure: bool) -> bool {
    let Some((scheme, host)) = url_host(url) else {
        return false;
    };
    if scheme.eq_ignore_ascii_case("https") {
        return !is_private_host(host);
    }
    allow_insecure && scheme.eq_ignore_ascii_case("http") && is_loopback_host(host)
}

/// Extract the scheme and host from a URL without a full parser.
pub(crate) fn url_host(url: &str) -> Option<(&str, &str)> {
    let (scheme, rest) = url.split_once("://")?;
    let authority = rest.split(['/', '?', '#']).next()?;
    let authority = authority.rsplit('@').next()?;
    let host = if let Some(rest) = authority.strip_prefix('[') {
        rest.split_once(']')?.0
    } else {
        authority.split(':').next()?
    };
    (!host.is_empty()).then_some((scheme, host))
}

/// Whether a host is a loopback name or address.
pub(crate) fn is_loopback_host(host: &str) -> bool {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    if host == "localhost" {
        return true;
    }
    host.parse::<IpAddr>()
        .map(|address| address.is_loopback())
        .unwrap_or(false)
}

/// Whether a host is a private, loopback, link-local, unspecified or multicast
/// address. Hostnames that are not IP literals are not resolved here; the
/// production sender performs the resolution check.
pub(crate) fn is_private_host(host: &str) -> bool {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    if host == "localhost" || host.ends_with(".localhost") {
        return true;
    }
    host.parse::<IpAddr>().map(is_private_ip).unwrap_or(false)
}

/// Whether an IP address must not be used as an outbound webhook target.
pub(crate) fn is_private_ip(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => {
            let octets = address.octets();
            address.is_loopback()
                || address.is_private()
                || address.is_link_local()
                || address.is_unspecified()
                || address.is_multicast()
                || octets[0] == 0
                || octets[0] == 255
        }
        IpAddr::V6(address) => {
            let segments = address.segments();
            address.is_loopback()
                || address.is_unspecified()
                || address.is_multicast()
                || (segments[0] & 0xfe00) == 0xfc00
                || (segments[0] & 0xffc0) == 0xfe80
        }
    }
}
