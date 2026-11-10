//! Page and search use cases.

use crate::collab::ChangeOp;
use crate::model::{Page, PageDraft, PageSummary, SearchResult};
use crate::webhooks::WebhookEventType;

use super::super::WorkspaceServer;
use super::UseCaseError;

/// Every page as a lightweight summary.
pub(in crate::server) fn list_pages(server: &WorkspaceServer) -> Vec<PageSummary> {
    server.workspace.pages.iter().map(Page::summary).collect()
}

/// One page by id.
pub(in crate::server) fn get_page(
    server: &WorkspaceServer,
    id: &str,
) -> Result<Page, UseCaseError> {
    server
        .workspace
        .page(id)
        .cloned()
        .ok_or_else(|| UseCaseError::NotFound("page not found".to_string()))
}

/// Create a page, record its first version, persist and emit the event.
pub(in crate::server) fn create_page(
    server: &mut WorkspaceServer,
    draft: PageDraft,
) -> Result<Page, UseCaseError> {
    let title = draft.title.trim().to_string();
    if title.is_empty() {
        return Err(UseCaseError::Invalid(
            "page title must not be empty".to_string(),
        ));
    }
    if let Some(parent) = draft.parent.as_deref()
        && server.workspace.page(parent).is_none()
    {
        return Err(UseCaseError::Invalid("unknown parent page".to_string()));
    }
    let icon = if draft.icon.trim().is_empty() {
        "P".to_string()
    } else {
        draft.icon
    };
    let mut page = Page::new(uuid::Uuid::new_v4().to_string(), title, icon);
    page.parent = draft.parent;
    server
        .workspace
        .add_page(page.clone())
        .map_err(|error| UseCaseError::Invalid(error.to_string()))?;
    let page_id = page.id.clone();
    let actor = server.collab.owner_id();
    server.collab.record_version(&page, &actor);
    server
        .write(ChangeOp::Create, &page_id)
        .map_err(|error| UseCaseError::Storage(error.to_string()))?;
    server.emit_webhook(
        WebhookEventType::PageCreated,
        "page",
        &page_id,
        serde_json::json!({ "parent": page.parent }).to_string(),
    );
    Ok(page)
}

/// Replace one page and emit the content or lock event.
pub(in crate::server) fn update_page(
    server: &mut WorkspaceServer,
    id: &str,
    mut page: Page,
) -> Result<Page, UseCaseError> {
    page.id = id.to_string();
    let was_locked = server
        .workspace
        .page(id)
        .map(|existing| existing.locked)
        .unwrap_or(false);
    {
        let Some(slot) = server.workspace.page_mut(id) else {
            return Err(UseCaseError::NotFound("page not found".to_string()));
        };
        *slot = page.clone();
    }
    let actor = server.collab.owner_id();
    server.collab.record_version(&page, &actor);
    server
        .write(ChangeOp::Update, id)
        .map_err(|error| UseCaseError::Storage(error.to_string()))?;
    if was_locked != page.locked {
        server.emit_webhook(
            if page.locked {
                WebhookEventType::PageLocked
            } else {
                WebhookEventType::PageUnlocked
            },
            "page",
            id,
            serde_json::json!({}).to_string(),
        );
    } else {
        server.emit_webhook(
            WebhookEventType::PageContentUpdated,
            "page",
            id,
            serde_json::json!({}).to_string(),
        );
    }
    Ok(page)
}

/// Delete a page, its descendants and emit the event.
pub(in crate::server) fn delete_page(
    server: &mut WorkspaceServer,
    id: &str,
) -> Result<Vec<String>, UseCaseError> {
    let removed = server
        .workspace
        .remove_page(id)
        .map_err(|error| UseCaseError::NotFound(error.to_string()))?;
    server
        .write(ChangeOp::Delete, id)
        .map_err(|error| UseCaseError::Storage(error.to_string()))?;
    server.emit_webhook(
        WebhookEventType::PageDeleted,
        "page",
        id,
        serde_json::json!({}).to_string(),
    );
    Ok(removed)
}

/// Version history for one page, newest first.
pub(in crate::server) fn page_versions(
    server: &WorkspaceServer,
    id: &str,
) -> Vec<crate::collab::PageVersion> {
    server.collab.versions_for(id)
}

/// Ranked workspace search.
pub(in crate::server) fn search(server: &WorkspaceServer, query: &str) -> Vec<SearchResult> {
    server.workspace.search(query)
}
