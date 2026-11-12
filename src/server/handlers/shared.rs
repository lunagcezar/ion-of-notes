//! v2 adapters for the page, workspace and sharing use cases.

use super::super::error::{HandlerError, HandlerResult, body, ok};
use super::super::response::json;
use super::super::usecases::{
    self, UseCaseError, create_page, delete_page, get_page, get_workspace, invite_member,
    replace_workspace, set_share,
};
use super::super::{ApiRequest, ApiResponse, Envelope, Method, WorkspaceServer};
use crate::collab::MemberDraft;
use crate::model::{Page, PageDraft, Workspace};

impl WorkspaceServer {
    /// v2 shared routes that are not cursor-paginated lists.
    pub(in crate::server) fn handle_v2_shared(
        &mut self,
        request: &ApiRequest,
        path: &str,
        request_id: &str,
    ) -> HandlerResult<Option<ApiResponse>> {
        match (request.method, path) {
            (Method::Get, "/api/v2/workspace") => {
                Ok(Some(json(200, Envelope::ok(get_workspace(self)))))
            }
            (Method::Put, "/api/v2/workspace") => {
                let workspace: Workspace = body(request)?;
                match replace_workspace(self, workspace) {
                    Ok(workspace) => ok(json(200, Envelope::ok(workspace))),
                    Err(error) => Err(use_case_error(error)),
                }
            }
            (Method::Post, "/api/v2/pages") => {
                let draft: PageDraft = body(request)?;
                match create_page(self, draft) {
                    Ok(page) => ok(json(201, Envelope::ok(page))),
                    Err(error) => Err(use_case_error(error)),
                }
            }
            (Method::Get, "/api/v2/export") => {
                Ok(Some(json(200, Envelope::ok(usecases::export(self)))))
            }
            (Method::Get, "/api/v2/realtime") => {
                Ok(Some(json(200, Envelope::ok(usecases::realtime_contract()))))
            }
            (Method::Post, "/api/v2/members") => {
                let draft: MemberDraft = body(request)?;
                match invite_member(self, draft) {
                    Ok(member) => ok(json(201, Envelope::ok(member))),
                    Err(error) => Err(use_case_error(error)),
                }
            }
            (Method::Put, "/api/v2/shares") => {
                let share: crate::collab::Share = body(request)?;
                match set_share(self, share) {
                    Ok(share) => ok(json(200, Envelope::ok(share))),
                    Err(error) => Err(use_case_error(error)),
                }
            }
            _ => self.handle_v2_page(request, path, request_id),
        }
    }

    /// `/api/v2/pages/{id}` and its versions; list routes stay in `lists.rs`.
    fn handle_v2_page(
        &mut self,
        request: &ApiRequest,
        path: &str,
        _request_id: &str,
    ) -> HandlerResult<Option<ApiResponse>> {
        let Some(rest) = path.strip_prefix("/api/v2/pages/") else {
            return Ok(None);
        };
        if rest.is_empty() || rest.contains('/') {
            return Ok(None);
        }
        match request.method {
            Method::Get => match get_page(self, rest) {
                Ok(page) => ok(json(200, Envelope::ok(page))),
                Err(error) => Err(use_case_error(error)),
            },
            Method::Patch => {
                let page: Page = body(request)?;
                match usecases::update_page(self, rest, page) {
                    Ok(page) => ok(json(200, Envelope::ok(page))),
                    Err(error) => Err(use_case_error(error)),
                }
            }
            Method::Delete => match delete_page(self, rest) {
                Ok(removed) => ok(json(200, Envelope::ok(removed))),
                Err(error) => Err(use_case_error(error)),
            },
            _ => Ok(None),
        }
    }
}

/// Map one application error to the typed handler error.
fn use_case_error(error: UseCaseError) -> HandlerError {
    match error {
        UseCaseError::Invalid(message) => HandlerError::validation(message),
        UseCaseError::NotFound(message) => HandlerError::not_found(message),
        UseCaseError::Storage(message) => HandlerError::internal(message),
    }
}
