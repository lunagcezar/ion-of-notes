//! Page and search use cases.

use super::super::events::DomainEvent;
use crate::collab::ChangeOp;
use crate::model::{Page, PageDraft, PageSummary, SearchResult};

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
    let icon = draft.icon;
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
    server.notify(DomainEvent::PageCreated {
        page_id,
        parent: page.parent.clone(),
    });
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
        server.notify(DomainEvent::PageLockChanged {
            page_id: id.to_string(),
            locked: page.locked,
        });
    } else {
        server.notify(DomainEvent::PageContentUpdated {
            page_id: id.to_string(),
        });
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
    server.notify(DomainEvent::PageDeleted {
        page_id: id.to_string(),
    });
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
