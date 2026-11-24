//! Resource-scoped authorization and principal-aware projections.
//!
//! Capabilities gate a route; they do not prove that the connection may touch
//! one particular page. This module resolves the owning page of a nested
//! resource (block, comment, database, data source, view) and applies the same
//! direct/general share rules as a direct page route.
//!
//! Workspace-level resources have no owning page. They are owner-only, which
//! keeps a member with a page share from reaching workspace administration.

use super::{Connection, WorkspaceServer};
use crate::model::{
    CommentDto, DataSource, Database, Page, PageSummary, ParentDto, View, Workspace,
};
use std::collections::HashSet;

/// Which operation a caller wants to perform on a nested resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceAccess {
    Read,
    Write,
}

impl WorkspaceServer {
    /// The page that owns a parent DTO, if it is page-backed.
    pub fn page_for_parent(&self, parent: &ParentDto) -> Option<String> {
        match parent {
            ParentDto::Page { page_id } => Some(page_id.clone()),
            ParentDto::Block { block_id } => self.page_for_block(block_id),
            ParentDto::Database { database_id } => self.page_for_database(database_id),
            ParentDto::DataSource { data_source_id } => self.page_for_data_source(data_source_id),
            ParentDto::Workspace { .. } => None,
        }
    }

    /// The page that owns a block (or the page itself when the id is a page).
    pub fn page_for_block(&self, block_id: &str) -> Option<String> {
        self.workspace
            .page_id_for_block(block_id)
            .or_else(|| self.workspace.page(block_id).map(|page| page.id.clone()))
    }

    /// The page that owns a comment.
    pub fn page_for_comment(&self, comment: &CommentDto) -> Option<String> {
        self.page_for_parent(&comment.parent)
    }

    /// The page that owns a database.
    pub fn page_for_database(&self, database_id: &str) -> Option<String> {
        self.workspace
            .database(database_id)
            .and_then(|database| self.page_for_parent(&database.parent))
    }

    /// The page that owns a data source.
    pub fn page_for_data_source(&self, data_source_id: &str) -> Option<String> {
        let data_source = self.workspace.data_source(data_source_id)?;
        self.page_for_parent(&data_source.database_parent)
            .or_else(|| self.page_for_parent(&data_source.parent))
    }

    /// The page that owns a saved view.
    pub fn page_for_view(&self, view_id: &str) -> Option<String> {
        let view = self.workspace.view(view_id)?;
        self.page_for_parent(&view.parent).or_else(|| {
            view.data_source_id
                .as_deref()
                .and_then(|id| self.page_for_data_source(id))
        })
    }

    /// Whether the connection may read a saved view.
    pub fn view_visible(&self, connection: &Connection, view_id: &str) -> bool {
        self.can_access_resource(
            connection,
            self.page_for_view(view_id).as_deref(),
            ResourceAccess::Read,
        )
    }

    /// Whether the connection may access a resource that belongs to `page`.
    ///
    /// `None` means a workspace-level resource, which only the owner may use.
    pub fn can_access_resource(
        &self,
        connection: &Connection,
        page: Option<&str>,
        access: ResourceAccess,
    ) -> bool {
        match page {
            Some(page_id) => match access {
                ResourceAccess::Read => self.can_read_page(connection, page_id),
                ResourceAccess::Write => self.can_write_page(connection, page_id),
            },
            None => self.is_owner(connection),
        }
    }

    /// Filter page summaries to the pages the connection may read.
    pub fn visible_page_summaries(&self, connection: &Connection) -> Vec<PageSummary> {
        self.workspace
            .pages
            .iter()
            .filter(|page| self.can_read_page(connection, &page.id))
            .map(Page::summary)
            .collect()
    }

    /// Filter a workspace snapshot to the resources the connection may read.
    ///
    /// This is the single projection used by `/api/v2/workspace` and by the v1
    /// workspace read. A principal never receives a page it cannot read, nor
    /// the databases, data sources, views, comments or file uploads attached to
    /// such a page. Workspace-level resources stay owner-only.
    pub fn project_workspace(&self, connection: &Connection) -> Workspace {
        let mut workspace = self.workspace.clone();
        let readable: HashSet<String> = self
            .workspace
            .pages
            .iter()
            .filter(|page| self.can_read_page(connection, &page.id))
            .map(|page| page.id.clone())
            .collect();

        workspace.retain_pages(|page| readable.contains(&page.id));

        let databases: Vec<Database> = self
            .workspace
            .databases
            .iter()
            .filter(|database| self.resource_readable(connection, &readable, database))
            .cloned()
            .collect();
        workspace.databases = databases;

        let data_sources: Vec<DataSource> = self
            .workspace
            .data_sources
            .iter()
            .filter(|data_source| self.data_source_readable(connection, &readable, data_source))
            .cloned()
            .collect();
        workspace.data_sources = data_sources;

        let views: Vec<View> = self
            .workspace
            .views
            .iter()
            .filter(|view| self.view_readable(connection, &readable, view))
            .cloned()
            .collect();
        workspace.views = views;

        let comments: Vec<CommentDto> = self
            .workspace
            .comments
            .iter()
            .filter(|comment| {
                self.page_for_comment(comment)
                    .is_some_and(|page_id| readable.contains(&page_id))
            })
            .cloned()
            .collect();
        let referenced_uploads: HashSet<String> = comments
            .iter()
            .flat_map(|comment| comment.attachments.iter())
            .filter_map(|attachment| upload_id(&attachment.file))
            .collect();
        workspace.comments = comments;
        workspace
            .file_uploads
            .retain(|upload| self.is_owner(connection) || referenced_uploads.contains(&upload.id));
        workspace
    }

    /// Filter search results to pages the connection may read.
    pub fn visible_search(
        &self,
        connection: &Connection,
        query: &str,
    ) -> Vec<crate::model::SearchResult> {
        self.workspace
            .search(query)
            .into_iter()
            .filter(|result| self.can_read_page(connection, &result.page_id))
            .collect()
    }

    /// Filter the sync log to changes the connection may observe.
    pub fn visible_changes(&self, connection: &Connection, since: u64) -> crate::collab::SyncBatch {
        let mut batch = self.collab.changes_since(since);
        batch
            .changes
            .retain(|change| self.change_visible(connection, &change.page_id));
        batch
    }

    /// Whether one change-feed entry is visible to the connection.
    pub fn change_visible(&self, connection: &Connection, page_id: &str) -> bool {
        self.workspace
            .page(page_id)
            .is_some_and(|page| self.can_read_page(connection, &page.id))
            || self.is_owner(connection)
    }

    fn resource_readable(
        &self,
        connection: &Connection,
        readable: &HashSet<String>,
        database: &Database,
    ) -> bool {
        match self.page_for_parent(&database.parent) {
            Some(page_id) => readable.contains(&page_id),
            None => self.is_owner(connection),
        }
    }

    fn data_source_readable(
        &self,
        connection: &Connection,
        readable: &HashSet<String>,
        data_source: &DataSource,
    ) -> bool {
        let page = self
            .page_for_parent(&data_source.database_parent)
            .or_else(|| self.page_for_parent(&data_source.parent));
        match page {
            Some(page_id) => readable.contains(&page_id),
            None => self.is_owner(connection),
        }
    }

    fn view_readable(
        &self,
        connection: &Connection,
        readable: &HashSet<String>,
        view: &View,
    ) -> bool {
        let page = self.page_for_parent(&view.parent).or_else(|| {
            view.data_source_id
                .as_deref()
                .and_then(|id| self.page_for_data_source(id))
        });
        match page {
            Some(page_id) => readable.contains(&page_id),
            None => self.is_owner(connection),
        }
    }
}

/// Extract the upload id from a comment attachment.
fn upload_id(file: &crate::model::FileObject) -> Option<String> {
    match file {
        crate::model::FileObject::FileUpload { id } => Some(id.clone()),
        crate::model::FileObject::File { url, .. } | crate::model::FileObject::External { url } => {
            let rest = url.strip_prefix("local://file_uploads/")?;
            Some(rest.split('?').next().unwrap_or(rest).to_string())
        }
    }
}
