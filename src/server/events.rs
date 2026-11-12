//! Domain events emitted by the write use cases.
//!
//! One event names the domain fact; [`WorkspaceServer::notify`] maps it to the
//! outbound webhook contract. Handlers call one thing instead of spelling the
//! webhook type, entity type and entity id at every site (spec-100).

use super::WorkspaceServer;
use crate::webhooks::WebhookEventType;

/// A domain fact that subscribers may care about.
#[derive(Debug, Clone)]
pub(in crate::server) enum DomainEvent {
    PageCreated {
        page_id: String,
        parent: Option<String>,
    },
    PageContentUpdated {
        page_id: String,
    },
    PageLockChanged {
        page_id: String,
        locked: bool,
    },
    PageDeleted {
        page_id: String,
    },
    CommentCreated {
        comment_id: String,
        data: String,
    },
    CommentUpdated {
        comment_id: String,
        data: String,
    },
    CommentDeleted {
        comment_id: String,
        data: String,
    },
    ViewCreated {
        view_id: String,
        data_source_id: Option<String>,
    },
    ViewUpdated {
        view_id: String,
        data_source_id: Option<String>,
    },
    ViewDeleted {
        view_id: String,
    },
    FileUploadCreated {
        upload_id: String,
    },
    FileUploadCompleted {
        upload_id: String,
    },
}

impl WorkspaceServer {
    /// Publish one domain event to the webhook outbox.
    pub(in crate::server) fn notify(&mut self, event: DomainEvent) {
        match event {
            DomainEvent::PageCreated { page_id, parent } => self.emit_webhook(
                WebhookEventType::PageCreated,
                "page",
                &page_id,
                serde_json::json!({ "parent": parent }).to_string(),
            ),
            DomainEvent::PageContentUpdated { page_id } => self.emit_webhook(
                WebhookEventType::PageContentUpdated,
                "page",
                &page_id,
                serde_json::json!({}).to_string(),
            ),
            DomainEvent::PageLockChanged { page_id, locked } => self.emit_webhook(
                if locked {
                    WebhookEventType::PageLocked
                } else {
                    WebhookEventType::PageUnlocked
                },
                "page",
                &page_id,
                serde_json::json!({}).to_string(),
            ),
            DomainEvent::PageDeleted { page_id } => self.emit_webhook(
                WebhookEventType::PageDeleted,
                "page",
                &page_id,
                serde_json::json!({}).to_string(),
            ),
            DomainEvent::CommentCreated { comment_id, data } => self.emit_webhook(
                WebhookEventType::CommentCreated,
                "comment",
                &comment_id,
                data,
            ),
            DomainEvent::CommentUpdated { comment_id, data } => self.emit_webhook(
                WebhookEventType::CommentUpdated,
                "comment",
                &comment_id,
                data,
            ),
            DomainEvent::CommentDeleted { comment_id, data } => self.emit_webhook(
                WebhookEventType::CommentDeleted,
                "comment",
                &comment_id,
                data,
            ),
            DomainEvent::ViewCreated {
                view_id,
                data_source_id,
            } => self.emit_webhook(
                WebhookEventType::ViewCreated,
                "view",
                &view_id,
                serde_json::json!({ "data_source_id": data_source_id }).to_string(),
            ),
            DomainEvent::ViewUpdated {
                view_id,
                data_source_id,
            } => self.emit_webhook(
                WebhookEventType::ViewUpdated,
                "view",
                &view_id,
                serde_json::json!({ "data_source_id": data_source_id }).to_string(),
            ),
            DomainEvent::ViewDeleted { view_id } => self.emit_webhook(
                WebhookEventType::ViewDeleted,
                "view",
                &view_id,
                serde_json::json!({}).to_string(),
            ),
            DomainEvent::FileUploadCreated { upload_id } => self.emit_webhook(
                WebhookEventType::FileUploadCreated,
                "file_upload",
                &upload_id,
                serde_json::json!({ "status": "pending" }).to_string(),
            ),
            DomainEvent::FileUploadCompleted { upload_id } => self.emit_webhook(
                WebhookEventType::FileUploadCompleted,
                "file_upload",
                &upload_id,
                serde_json::json!({ "status": "uploaded" }).to_string(),
            ),
        }
    }
}
