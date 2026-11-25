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
use crate::collab::{CollabState, Role, general_access};
use crate::model::{
    CommentDto, DataSource, Database, FileUpload, Page, PageSummary, ParentDto, View, Workspace,
};
use std::collections::HashSet;

/// A read-only view of the workspace and collaboration state.
///
/// The HTTP adapter clones both once while holding the server lock and then
/// runs authorization, projection and serialization on this view after the lock
/// is released (spec-143).
pub struct ReadView<'a> {
    pub(crate) workspace: &'a Workspace,
    pub(crate) collab: &'a CollabState,
}

impl<'a> ReadView<'a> {
    pub fn new(workspace: &'a Workspace, collab: &'a CollabState) -> Self {
        Self { workspace, collab }
    }
}

/// Which operation a caller wants to perform on a nested resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceAccess {
    Read,
    Write,
}

macro_rules! authorization_methods {
    () => {
        /// Whether the connection is the workspace owner.
        pub fn is_owner(&self, connection: &Connection) -> bool {
            self.collab
                .members
                .iter()
                .find(|member| member.id == connection.member_id)
                .is_some_and(|member| member.role == Role::Owner)
        }

        /// Resolve the effective access a connection has to one page.
        ///
        /// Direct shares win over page-general rules, which win over the
        /// workspace-general rule. An owner always has full access.
        pub fn page_access(&self, connection: &Connection, page_id: &str) -> crate::collab::Access {
            let Some(member) = self
                .collab
                .members
                .iter()
                .find(|member| member.id == connection.member_id)
            else {
                return crate::collab::Access::None;
            };
            if member.role == Role::Owner {
                return crate::collab::Access::Full;
            }
            let direct = self
                .collab
                .shares
                .iter()
                .filter(|share| share.page_id == page_id)
                .filter(|share| {
                    share.subject == member.id
                        || share.subject.eq_ignore_ascii_case(&member.email)
                        || share.subject == member.name
                })
                .max_by_key(|share| access_rank(share.access))
                .map(|share| share.access);
            direct.unwrap_or_else(|| general_access(&self.collab.shares, page_id))
        }

        /// Whether the connection may read a page.
        pub fn can_read_page(&self, connection: &Connection, page_id: &str) -> bool {
            self.page_access(connection, page_id) != crate::collab::Access::None
        }

        /// Whether the connection may modify a page.
        pub fn can_write_page(&self, connection: &Connection, page_id: &str) -> bool {
            self.page_access(connection, page_id).can_write()
        }

        /// The page that owns a parent DTO, if it is page-backed.
        pub fn page_for_parent(&self, parent: &ParentDto) -> Option<String> {
            match parent {
                ParentDto::Page { page_id } => Some(page_id.clone()),
                ParentDto::Block { block_id } => self.page_for_block(block_id),
                ParentDto::Database { database_id } => self.page_for_database(database_id),
                ParentDto::DataSource { data_source_id } => {
                    self.page_for_data_source(data_source_id)
                }
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
        pub fn project_workspace(&self, connection: &Connection) -> Workspace {
            let pages: Vec<Page> = self
                .workspace
                .pages
                .iter()
                .filter(|page| self.can_read_page(connection, &page.id))
                .cloned()
                .collect();
            let readable: HashSet<String> = pages.iter().map(|page| page.id.clone()).collect();

            let databases: Vec<Database> = self
                .workspace
                .databases
                .iter()
                .filter(|database| self.resource_readable(connection, &readable, database))
                .cloned()
                .collect();

            let data_sources: Vec<DataSource> = self
                .workspace
                .data_sources
                .iter()
                .filter(|data_source| self.data_source_readable(connection, &readable, data_source))
                .cloned()
                .collect();

            let views: Vec<View> = self
                .workspace
                .views
                .iter()
                .filter(|view| self.view_readable(connection, &readable, view))
                .cloned()
                .collect();

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
            let file_uploads: Vec<FileUpload> = self
                .workspace
                .file_uploads
                .iter()
                .filter(|upload| {
                    self.is_owner(connection) || referenced_uploads.contains(&upload.id)
                })
                .cloned()
                .collect();

            Workspace::from_stored_parts(
                self.workspace.name.clone(),
                pages,
                self.workspace.version,
                self.workspace.settings.clone(),
                databases,
                data_sources,
                views,
                comments,
                file_uploads,
            )
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
        pub fn visible_changes(
            &self,
            connection: &Connection,
            since: u64,
        ) -> crate::collab::SyncBatch {
            let mut batch = self.collab.changes_since(since);
            batch
                .changes
                .retain(|change| self.change_visible(connection, &change.page_id));
            batch
        }

        /// Whether one change-feed entry is visible to the connection.
        ///
        /// Uses the persisted share rules rather than the current page tree, so
        /// a delete event still reaches members who had access to the page
        /// (spec-139). Workspace-level markers stay owner-only.
        pub fn change_visible(&self, connection: &Connection, page_id: &str) -> bool {
            if page_id == "*" || page_id.starts_with("__") {
                return self.is_owner(connection);
            }
            self.is_owner(connection) || self.can_read_page(connection, page_id)
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
    };
}

impl WorkspaceServer {
    authorization_methods!();
}

impl ReadView<'_> {
    authorization_methods!();
}

fn access_rank(access: crate::collab::Access) -> u8 {
    match access {
        crate::collab::Access::None => 0,
        crate::collab::Access::View => 1,
        crate::collab::Access::Edit => 2,
        crate::collab::Access::Full => 3,
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
