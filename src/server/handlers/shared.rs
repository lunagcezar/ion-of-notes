//! v2 adapters for the page, workspace and sharing use cases.

use super::super::response::{json, parse_body, v2_error};
use super::super::usecases::{
    self, UseCaseError, create_page, delete_page, get_page, get_workspace, invite_member,
    replace_workspace, set_share,
};
use super::super::{
    ApiRequest, ApiResponse, CODE_INTERNAL, CODE_INVALID_JSON, CODE_NOT_FOUND,
    CODE_VALIDATION_ERROR, Envelope, Method, WorkspaceServer,
};
use crate::collab::MemberDraft;
use crate::model::{Page, PageDraft, Workspace};

impl WorkspaceServer {
    /// v2 shared routes that are not cursor-paginated lists.
    pub(in crate::server) fn handle_v2_shared(
        &mut self,
        request: &ApiRequest,
        path: &str,
        request_id: &str,
    ) -> Option<ApiResponse> {
        match (request.method, path) {
            (Method::Get, "/api/v2/workspace") => {
                Some(json(200, Envelope::ok(get_workspace(self))))
            }
            (Method::Put, "/api/v2/workspace") => Some(match parse_body::<Workspace>(request) {
                Ok(workspace) => match replace_workspace(self, workspace) {
                    Ok(workspace) => json(200, Envelope::ok(workspace)),
                    Err(error) => v2_use_case_error(error, request_id),
                },
                Err(message) => v2_error(400, CODE_INVALID_JSON, message, request_id, None),
            }),
            (Method::Post, "/api/v2/pages") => Some(match parse_body::<PageDraft>(request) {
                Ok(draft) => match create_page(self, draft) {
                    Ok(page) => json(201, Envelope::ok(page)),
                    Err(error) => v2_use_case_error(error, request_id),
                },
                Err(message) => v2_error(400, CODE_INVALID_JSON, message, request_id, None),
            }),
            (Method::Get, "/api/v2/export") => {
                Some(json(200, Envelope::ok(usecases::export(self))))
            }
            (Method::Get, "/api/v2/realtime") => {
                Some(json(200, Envelope::ok(usecases::realtime_contract())))
            }
            (Method::Post, "/api/v2/members") => Some(match parse_body::<MemberDraft>(request) {
                Ok(draft) => match invite_member(self, draft) {
                    Ok(member) => json(201, Envelope::ok(member)),
                    Err(error) => v2_use_case_error(error, request_id),
                },
                Err(message) => v2_error(400, CODE_INVALID_JSON, message, request_id, None),
            }),
            (Method::Put, "/api/v2/shares") => {
                Some(match parse_body::<crate::collab::Share>(request) {
                    Ok(share) => match set_share(self, share) {
                        Ok(share) => json(200, Envelope::ok(share)),
                        Err(error) => v2_use_case_error(error, request_id),
                    },
                    Err(message) => v2_error(400, CODE_INVALID_JSON, message, request_id, None),
                })
            }
            _ => self.handle_v2_page(request, path, request_id),
        }
    }

    /// `/api/v2/pages/{id}` and its versions; list routes stay in `lists.rs`.
    fn handle_v2_page(
        &mut self,
        request: &ApiRequest,
        path: &str,
        request_id: &str,
    ) -> Option<ApiResponse> {
        let rest = path.strip_prefix("/api/v2/pages/")?;
        if rest.is_empty() || rest.contains('/') {
            return None;
        }
        match request.method {
            Method::Get => Some(match get_page(self, rest) {
                Ok(page) => json(200, Envelope::ok(page)),
                Err(error) => v2_use_case_error(error, request_id),
            }),
            Method::Patch => Some(match parse_body::<Page>(request) {
                Ok(page) => match usecases::update_page(self, rest, page) {
                    Ok(page) => json(200, Envelope::ok(page)),
                    Err(error) => v2_use_case_error(error, request_id),
                },
                Err(message) => v2_error(400, CODE_INVALID_JSON, message, request_id, None),
            }),
            Method::Delete => Some(match delete_page(self, rest) {
                Ok(removed) => json(200, Envelope::ok(removed)),
                Err(error) => v2_use_case_error(error, request_id),
            }),
            _ => None,
        }
    }
}

/// Map one application error to the v2 structured error shape.
fn v2_use_case_error(error: UseCaseError, request_id: &str) -> ApiResponse {
    match error {
        UseCaseError::Invalid(message) => {
            v2_error(400, CODE_VALIDATION_ERROR, message, request_id, None)
        }
        UseCaseError::NotFound(message) => v2_error(404, CODE_NOT_FOUND, message, request_id, None),
        UseCaseError::Storage(message) => v2_error(500, CODE_INTERNAL, message, request_id, None),
    }
}
