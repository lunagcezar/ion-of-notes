//! Cursor-paginated v2 list routes.

use super::super::pagination::PageRequest;
use super::super::response::{query_value, v2_error, v2_list_response};
use super::super::{ApiRequest, ApiResponse, Method, WorkspaceServer};
use crate::model::PageSummary;

impl WorkspaceServer {
    pub(in crate::server) fn handle_v2_list(
        &mut self,
        request: &ApiRequest,
        path: &str,
        query: &str,
        request_id: &str,
    ) -> Option<ApiResponse> {
        if request.method != Method::Get {
            return None;
        }
        let is_list_path = matches!(
            path,
            "/api/v2/pages"
                | "/api/v2/members"
                | "/api/v2/shares"
                | "/api/v2/sync"
                | "/api/v2/search"
        ) || (path.starts_with("/api/v2/pages/") && path.ends_with("/versions"));
        if !is_list_path {
            return None;
        }
        let page = match PageRequest::from_query(query) {
            Ok(page) => page,
            Err(error) => {
                return Some(v2_error(
                    400,
                    error.code(),
                    error.message(),
                    request_id,
                    None,
                ));
            }
        };

        if path == "/api/v2/pages" {
            let items: Vec<PageSummary> = self
                .workspace
                .pages
                .iter()
                .map(|page| page.summary())
                .collect();
            return Some(v2_list_response(items, &page, "pages", "page", request_id));
        }
        if path == "/api/v2/members" {
            return Some(v2_list_response(
                self.collab.members.clone(),
                &page,
                "members",
                "member",
                request_id,
            ));
        }
        if path == "/api/v2/shares" {
            let page_id = query_value(query, "page");
            let shares = if page_id.is_empty() {
                self.collab.shares.clone()
            } else {
                self.collab.shares_for(&page_id)
            };
            let sort = format!("shares:{page_id}");
            return Some(v2_list_response(shares, &page, &sort, "share", request_id));
        }
        if path == "/api/v2/sync" {
            let since = query_value(query, "since").parse::<u64>().unwrap_or(0);
            let changes = self.collab.changes_since(since).changes;
            let sort = format!("sync:{since}");
            return Some(v2_list_response(
                changes, &page, &sort, "change", request_id,
            ));
        }
        if path == "/api/v2/search" {
            let q = query_value(query, "q");
            let results = self.workspace.search(&q);
            let sort = format!("search:{q}");
            return Some(v2_list_response(
                results,
                &page,
                &sort,
                "search_result",
                request_id,
            ));
        }
        if let Some(id) = path
            .strip_prefix("/api/v2/pages/")
            .and_then(|rest| rest.strip_suffix("/versions"))
        {
            let versions = self.collab.versions_for(id);
            let sort = format!("versions:{id}");
            return Some(v2_list_response(
                versions,
                &page,
                &sort,
                "page_version",
                request_id,
            ));
        }
        None
    }
}
