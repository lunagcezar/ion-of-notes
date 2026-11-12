//! v2 comment routes, returning typed handler errors.

use super::super::error::{HandlerError, HandlerResult, body};
use super::super::pagination::PageRequest;
use super::super::response::{json, query_value, v2_list_response};
use super::super::usecases::{
    UseCaseError, create_comment, delete_comment, list_comments, update_comment,
};
use super::super::{ApiRequest, ApiResponse, Envelope, Method, WorkspaceServer};
use crate::model::CommentInput;

impl WorkspaceServer {
    pub(in crate::server) fn handle_v2_comments(
        &mut self,
        request: &ApiRequest,
        path: &str,
        query: &str,
        request_id: &str,
    ) -> HandlerResult<Option<ApiResponse>> {
        if path == "/api/v2/comments" && request.method == Method::Get {
            let page_filter = query_value(query, "page");
            let block_filter = query_value(query, "block");
            let page = PageRequest::from_query(query)
                .map_err(|error| HandlerError::validation(error.message()))?;
            let sort = format!("comments:{page_filter}:{block_filter}");
            return Ok(Some(v2_list_response(
                list_comments(self, &page_filter, &block_filter),
                &page,
                &sort,
                "comment",
                request_id,
            )));
        }
        if path == "/api/v2/comments" && request.method == Method::Post {
            let input: CommentInput = body(request)?;
            let comment = create_comment(self, input).map_err(use_case_error)?;
            return Ok(Some(json(201, Envelope::ok(comment))));
        }

        let Some(id) = path.strip_prefix("/api/v2/comments/") else {
            return Ok(None);
        };
        if id.is_empty() || id.contains('/') {
            return Err(HandlerError::not_found("comment not found"));
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
                let input: CommentInput = body(request)?;
                let dto = update_comment(self, id, input).map_err(use_case_error)?;
                Ok(Some(json(200, Envelope::ok(dto))))
            }
            Method::Delete => {
                let dto = delete_comment(self, id).map_err(use_case_error)?;
                Ok(Some(json(200, Envelope::ok(dto))))
            }
            _ => Err(HandlerError::not_found("unknown route")),
        }
    }
}

fn use_case_error(error: UseCaseError) -> HandlerError {
    match error {
        UseCaseError::Invalid(message) => HandlerError::validation(message),
        UseCaseError::NotFound(message) => HandlerError::not_found(message),
        UseCaseError::Storage(message) => HandlerError::internal(message),
    }
}
