//! Legacy v1 route dispatch. Every route keeps the v1 envelope and status and
//! delegates the actual work to the shared use cases.

use super::super::response::{bad, json, not_found, parse_body, query_value, server_error};
use super::super::usecases::{
    self, UseCaseError, create_page, delete_page, get_page, get_workspace, invite_member,
    list_members, list_pages, list_shares, page_versions, replace_workspace, search, set_share,
};
use super::super::{ApiRequest, ApiResponse, Envelope, Method, WorkspaceServer};
use crate::collab::MemberDraft;
use crate::model::{Page, PageDraft, Workspace};

impl WorkspaceServer {
    pub(super) fn handle_v1(&mut self, request: &ApiRequest) -> ApiResponse {
        let (path, query) = match request.path.split_once('?') {
            Some((p, q)) => (p, q),
            None => (request.path.as_str(), ""),
        };

        match (request.method, path) {
            (Method::Get, "/health") => json(200, Envelope::ok("ok".to_string())),

            (Method::Get, "/api/workspace") => json(200, Envelope::ok(get_workspace(self))),
            (Method::Put, "/api/workspace") => match parse_body::<Workspace>(request) {
                Ok(workspace) => match replace_workspace(self, workspace) {
                    Ok(workspace) => json(200, Envelope::ok(workspace)),
                    Err(error) => v1_error(error),
                },
                Err(message) => bad(&message),
            },

            (Method::Get, "/api/pages") => json(200, Envelope::ok(list_pages(self))),
            (Method::Post, "/api/pages") => match parse_body::<PageDraft>(request) {
                Ok(draft) => match create_page(self, draft) {
                    Ok(page) => json(201, Envelope::ok(page)),
                    Err(error) => v1_error(error),
                },
                Err(message) => bad(&message),
            },

            (Method::Get, "/api/search") => {
                let q = query_value(query, "q");
                json(200, Envelope::ok(search(self, &q)))
            }

            // ---- decided collaborative API (see docs/adr-003-*) ----------
            (Method::Get, "/api/members") => json(200, Envelope::ok(list_members(self))),
            (Method::Post, "/api/members") => match parse_body::<MemberDraft>(request) {
                Ok(draft) => match invite_member(self, draft) {
                    Ok(member) => json(201, Envelope::ok(member)),
                    Err(error) => v1_error(error),
                },
                Err(message) => bad(&message),
            },
            (Method::Get, "/api/shares") => {
                let page = query_value(query, "page");
                json(200, Envelope::ok(list_shares(self, Some(&page))))
            }
            (Method::Put, "/api/shares") => match parse_body::<crate::collab::Share>(request) {
                Ok(share) => match set_share(self, share) {
                    Ok(share) => json(200, Envelope::ok(share)),
                    Err(error) => v1_error(error),
                },
                Err(message) => bad(&message),
            },
            (Method::Get, "/api/sync") => {
                let since = query_value(query, "since").parse::<u64>().unwrap_or(0);
                json(200, Envelope::ok(usecases::sync(self, since)))
            }
            (Method::Get, "/api/export") => json(200, Envelope::ok(usecases::export(self))),
            (Method::Get, "/api/realtime") => {
                json(200, Envelope::ok(usecases::realtime_contract()))
            }
            // ---------------------------------------------------------------
            (method, page_path) if page_path.starts_with("/api/pages/") => {
                let rest = &page_path["/api/pages/".len()..];
                if rest.is_empty() {
                    return not_found("unknown route");
                }
                if let Some(id) = rest.strip_suffix("/versions") {
                    if id.is_empty() {
                        return not_found("unknown route");
                    }
                    return match method {
                        Method::Get => json(200, Envelope::ok(page_versions(self, id))),
                        _ => not_found("unknown route"),
                    };
                }
                let id = rest;
                match method {
                    Method::Get => match get_page(self, id) {
                        Ok(page) => json(200, Envelope::ok(page)),
                        Err(error) => v1_error(error),
                    },
                    Method::Patch => match parse_body::<Page>(request) {
                        Ok(page) => match usecases::update_page(self, id, page) {
                            Ok(page) => json(200, Envelope::ok(page)),
                            Err(error) => v1_error(error),
                        },
                        Err(message) => bad(&message),
                    },
                    Method::Delete => match delete_page(self, id) {
                        Ok(removed) => json(200, Envelope::ok(removed)),
                        Err(error) => v1_error(error),
                    },
                    _ => not_found("unknown route"),
                }
            }

            _ => not_found("unknown route"),
        }
    }
}

/// Map one application error to the v1 envelope and status.
fn v1_error(error: UseCaseError) -> ApiResponse {
    match error {
        UseCaseError::Invalid(message) => bad(&message),
        UseCaseError::NotFound(message) => not_found(&message),
        UseCaseError::Storage(message) => server_error(&message),
    }
}
