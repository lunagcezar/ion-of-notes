//! v2 adapters for the page, workspace and sharing use cases.

use super::super::authorization::ResourceAccess;
use super::super::error::{HandlerError, HandlerResult, body, ok};
use super::super::response::json;
use super::super::usecases::{
    self, UseCaseError, create_page, delete_page, get_page, invite_member, issue_invite,
    replace_workspace, set_share,
};
use super::super::{ApiRequest, ApiResponse, Connection, Envelope, Method, WorkspaceServer};
use crate::collab::MemberDraft;
use crate::model::{Page, PageDraft, Workspace};

impl WorkspaceServer {
    /// v2 shared routes that are not cursor-paginated lists.
    pub(in crate::server) fn handle_v2_shared(
        &mut self,
        request: &ApiRequest,
        path: &str,
        request_id: &str,
        connection: &Connection,
    ) -> HandlerResult<Option<ApiResponse>> {
        match (request.method, path) {
            (Method::Get, "/api/v2/workspace") => Ok(Some(json(
                200,
                Envelope::ok(self.project_workspace(connection)),
            ))),
            (Method::Post, "/api/v2/realtime/ticket") => {
                let ticket = self.issue_realtime_ticket(connection);
                ok(json(
                    200,
                    Envelope::ok(serde_json::json!({
                        "ticket": ticket,
                        "expires_in": crate::server::REALTIME_TICKET_TTL_SECS,
                    })),
                ))
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
                if let Some(parent) = draft.parent.as_deref()
                    && !self.can_access_resource(connection, Some(parent), ResourceAccess::Write)
                {
                    return Err(HandlerError::restricted(
                        "write access to the parent page is not allowed",
                        parent,
                        "write",
                    ));
                }
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
            (Method::Post, _) if path.starts_with("/api/v2/members/") => {
                let Some(member_id) = path
                    .strip_prefix("/api/v2/members/")
                    .and_then(|rest| rest.strip_suffix("/invite"))
                else {
                    return Ok(None);
                };
                if member_id.is_empty() || member_id.contains('/') {
                    return Ok(None);
                }
                match issue_invite(self, member_id) {
                    Ok((token, expires_at)) => ok(json(
                        201,
                        Envelope::ok(serde_json::json!({
                            "member_id": member_id,
                            "token": token,
                            "expires_at": expires_at,
                        })),
                    )),
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
