//! Cursor-paginated v2 list routes.

use super::super::error::{HandlerResult, fail, ok};
use super::super::pagination::PageRequest;
use super::super::response::{query_value, v2_error, v2_list_response};
use super::super::usecases::{list_members, list_pages, list_shares, page_versions, search, sync};
use super::super::{ApiRequest, ApiResponse, Method, WorkspaceServer};

impl WorkspaceServer {
    pub(in crate::server) fn handle_v2_list(
        &mut self,
        request: &ApiRequest,
        path: &str,
        query: &str,
        request_id: &str,
    ) -> HandlerResult<Option<ApiResponse>> {
        if request.method != Method::Get {
            return Ok(None);
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
            return Ok(None);
        }
        let page = match PageRequest::from_query(query) {
            Ok(page) => page,
            Err(error) => {
                return fail(v2_error(
                    400,
                    error.code(),
                    error.message(),
                    request_id,
                    None,
                ));
            }
        };

        if path == "/api/v2/pages" {
            let items = list_pages(self);
            return ok(v2_list_response(items, &page, "pages", "page", request_id));
        }
        if path == "/api/v2/members" {
            return ok(v2_list_response(
                list_members(self),
                &page,
                "members",
                "member",
                request_id,
            ));
        }
        if path == "/api/v2/shares" {
            let page_id = query_value(query, "page");
            let shares = list_shares(self, Some(&page_id));
            let sort = format!("shares:{page_id}");
            return ok(v2_list_response(shares, &page, &sort, "share", request_id));
        }
        if path == "/api/v2/sync" {
            let since = query_value(query, "since").parse::<u64>().unwrap_or(0);
            let changes = sync(self, since).changes;
            let sort = format!("sync:{since}");
            return ok(v2_list_response(
                changes, &page, &sort, "change", request_id,
            ));
        }
        if path == "/api/v2/search" {
            let q = query_value(query, "q");
            let results = search(self, &q);
            let sort = format!("search:{q}");
            return ok(v2_list_response(
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
            let versions = page_versions(self, id);
            let sort = format!("versions:{id}");
            return ok(v2_list_response(
                versions,
                &page,
                &sort,
                "page_version",
                request_id,
            ));
        }
        Ok(None)
    }
}
