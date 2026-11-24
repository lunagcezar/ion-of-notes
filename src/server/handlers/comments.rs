//! v2 comment routes, returning typed handler errors.

use super::super::authorization::ResourceAccess;
use super::super::error::{HandlerError, HandlerResult, body};
use super::super::pagination::PageRequest;
use super::super::response::{json, query_value, v2_list_response};
use super::super::usecases::{
    UseCaseError, create_comment, delete_comment, list_comments, update_comment,
};
use super::super::{ApiRequest, ApiResponse, Connection, Envelope, Method, WorkspaceServer};
use crate::model::CommentInput;

impl WorkspaceServer {
    pub(in crate::server) fn handle_v2_comments(
        &mut self,
        request: &ApiRequest,
        path: &str,
        query: &str,
        request_id: &str,
        connection: &Connection,
    ) -> HandlerResult<Option<ApiResponse>> {
        if path == "/api/v2/comments" && request.method == Method::Get {
            let page_filter = query_value(query, "page");
            let block_filter = query_value(query, "block");
            if !page_filter.is_empty() && !self.can_read_page(connection, &page_filter) {
                return Err(restricted(&page_filter, "read"));
            }
            if !block_filter.is_empty()
                && let Some(page_id) = self.page_for_block(&block_filter)
                && !self.can_read_page(connection, &page_id)
            {
                return Err(restricted(&page_id, "read"));
            }
            let page = PageRequest::from_query(query)
                .map_err(|error| HandlerError::validation(error.message()))?;
            let sort = format!("comments:{page_filter}:{block_filter}");
            let comments: Vec<_> = list_comments(self, &page_filter, &block_filter)
                .into_iter()
                .filter(|comment| {
                    self.page_for_comment(comment)
                        .is_some_and(|page_id| self.can_read_page(connection, &page_id))
                })
                .collect();
            return Ok(Some(v2_list_response(
                comments, &page, &sort, "comment", request_id,
            )));
        }
        if path == "/api/v2/comments" && request.method == Method::Post {
            let input: CommentInput = body(request)?;
            let page_id = self.page_for_parent(&input.parent).ok_or_else(|| {
                HandlerError::validation("comment parent page or block not found")
            })?;
            if !self.can_access_resource(connection, Some(&page_id), ResourceAccess::Write) {
                return Err(restricted(&page_id, "write"));
            }
            let comment = create_comment(self, input).map_err(use_case_error)?;
            return Ok(Some(json(201, Envelope::ok(comment))));
        }

        let Some(id) = path.strip_prefix("/api/v2/comments/") else {
            return Ok(None);
        };
        if id.is_empty() || id.contains('/') {
            return Err(HandlerError::not_found("comment not found"));
        }
        let page_id = self
            .workspace
            .comments
            .iter()
            .find(|comment| comment.id == id)
            .and_then(|comment| self.page_for_comment(comment));
        if let Some(page_id) = &page_id
            && !self.can_read_page(connection, page_id)
        {
            return Err(restricted(page_id, "read"));
        }
        match request.method {
            Method::Get => match self
                .workspace
                .comments
                .iter()
                .find(|comment| comment.id == id)
            {
                Some(comment) => Ok(Some(json(200, Envelope::ok(comment.clone())))),
                None => Err(HandlerError::not_found("comment not found")),
            },
            Method::Patch => {
                if let Some(page_id) = &page_id
                    && !self.can_access_resource(connection, Some(page_id), ResourceAccess::Write)
                {
                    return Err(restricted(page_id, "write"));
                }
                let input: CommentInput = body(request)?;
                let dto = update_comment(self, id, input).map_err(use_case_error)?;
                Ok(Some(json(200, Envelope::ok(dto))))
            }
            Method::Delete => {
                if let Some(page_id) = &page_id
                    && !self.can_access_resource(connection, Some(page_id), ResourceAccess::Write)
                {
                    return Err(restricted(page_id, "write"));
                }
                let dto = delete_comment(self, id).map_err(use_case_error)?;
                Ok(Some(json(200, Envelope::ok(dto))))
            }
            _ => Err(HandlerError::not_found("unknown route")),
        }
    }
}

fn restricted(page_id: &str, access: &str) -> HandlerError {
    HandlerError::restricted(
        format!("{access} access to this comment's page is not allowed"),
        page_id,
        access,
    )
}

fn use_case_error(error: UseCaseError) -> HandlerError {
    match error {
        UseCaseError::Invalid(message) => HandlerError::validation(message),
        UseCaseError::NotFound(message) => HandlerError::not_found(message),
        UseCaseError::Storage(message) => HandlerError::internal(message),
    }
}
