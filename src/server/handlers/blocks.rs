//! Block read, append, update and delete v2 route use cases.

use super::super::contract::{CODE_INTERNAL, CODE_INVALID_JSON, CODE_NOT_FOUND};
use super::super::error::{HandlerResult, fail, ok};
use super::super::pagination::PageRequest;
use super::super::response::{json, parse_body, v2_error, v2_list_response};
use super::super::{ApiRequest, ApiResponse, Envelope, Method, WorkspaceServer};
use crate::collab::ChangeOp;
use crate::model::{
    Block, BlockChildrenRequest, BlockDto, BlockInput, BlockKind, RichText, now_iso,
};

impl WorkspaceServer {
    pub(in crate::server) fn handle_v2_block(
        &mut self,
        request: &ApiRequest,
        path: &str,
        query: &str,
        request_id: &str,
    ) -> HandlerResult<Option<ApiResponse>> {
        let Some(rest) = path.strip_prefix("/api/v2/blocks/") else {
            return Ok(None);
        };
        let (id, is_children) = match rest.strip_suffix("/children") {
            Some(id) => (id, true),
            None => (rest, false),
        };
        if id.is_empty() {
            return fail(v2_error(
                404,
                CODE_NOT_FOUND,
                "block not found",
                request_id,
                None,
            ));
        }

        match (request.method, is_children) {
            (Method::Get, false) => {
                let Some(page_id) = self.workspace.page_id_for_block(id) else {
                    return fail(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "block not found",
                        request_id,
                        None,
                    ));
                };
                let Some(block) = self.workspace.block(id) else {
                    return fail(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "block not found",
                        request_id,
                        None,
                    ));
                };
                ok(json(200, Envelope::ok(block.to_dto(&page_id))))
            }
            (Method::Get, true) => {
                let page_id = self
                    .workspace
                    .page_id_for_block(id)
                    .or_else(|| self.workspace.page(id).map(|page| page.id.clone()));
                let Some(page_id) = page_id else {
                    return fail(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "block not found",
                        request_id,
                        None,
                    ));
                };
                let Some(children) = self.workspace.block_children(id) else {
                    return fail(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "block not found",
                        request_id,
                        None,
                    ));
                };
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
                let items: Vec<BlockDto> = children
                    .iter()
                    .map(|block| block.to_dto(&page_id))
                    .collect();
                let sort = format!("block_children:{id}");
                ok(v2_list_response(items, &page, &sort, "block", request_id))
            }
            (Method::Patch, true) => {
                let body = match parse_body::<BlockChildrenRequest>(request) {
                    Ok(body) => body,
                    Err(message) => {
                        return fail(v2_error(400, CODE_INVALID_JSON, message, request_id, None));
                    }
                };
                let is_page_parent = self.workspace.page(id).is_some();
                let page_id = self
                    .workspace
                    .page_id_for_block(id)
                    .or_else(|| is_page_parent.then(|| id.to_string()));
                let Some(page_id) = page_id else {
                    return fail(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "block not found",
                        request_id,
                        None,
                    ));
                };
                let parent_block = (!is_page_parent).then(|| id.to_string());
                let actor = self.collab.owner_id();
                let mut created = Vec::new();
                {
                    let Some(children) = self.workspace.block_children_mut(id) else {
                        return fail(v2_error(
                            404,
                            CODE_NOT_FOUND,
                            "block not found",
                            request_id,
                            None,
                        ));
                    };
                    for input in body.children {
                        let child_id = uuid::Uuid::new_v4().to_string();
                        let child = Block::from_input(
                            input,
                            child_id,
                            parent_block.clone(),
                            Some(actor.clone()),
                        );
                        children.push(child.clone());
                        created.push(child);
                    }
                }
                if let Err(error) = self.write(ChangeOp::Update, &page_id) {
                    return fail(v2_error(
                        500,
                        CODE_INTERNAL,
                        error.to_string(),
                        request_id,
                        None,
                    ));
                }
                let items: Vec<BlockDto> =
                    created.iter().map(|block| block.to_dto(&page_id)).collect();
                ok(json(200, Envelope::ok(items)))
            }
            (Method::Patch, false) => {
                let input = match parse_body::<BlockInput>(request) {
                    Ok(input) => input,
                    Err(message) => {
                        return fail(v2_error(400, CODE_INVALID_JSON, message, request_id, None));
                    }
                };
                let Some(page_id) = self.workspace.page_id_for_block(id) else {
                    return fail(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "block not found",
                        request_id,
                        None,
                    ));
                };
                let actor = self.collab.owner_id();
                let dto = {
                    let Some(block) = self.workspace.block_mut(id) else {
                        return fail(v2_error(
                            404,
                            CODE_NOT_FOUND,
                            "block not found",
                            request_id,
                            None,
                        ));
                    };
                    let type_name = if input.r#type.is_empty() {
                        block.kind.as_api_type(block.unsupported_type.as_deref())
                    } else {
                        input.r#type.clone()
                    };
                    block.kind = BlockKind::from_api_type(&type_name);
                    block.unsupported_type =
                        (block.kind == BlockKind::Unsupported).then_some(type_name);
                    if !input.rich_text.is_empty() || input.text.is_some() {
                        block.rich_text = input.rich_text;
                        if block.rich_text.is_empty()
                            && let Some(text) = input.text
                        {
                            block.rich_text = vec![RichText::text(text)];
                        }
                        block.sync_text_from_rich_text();
                    }
                    block.checked = input.checked;
                    block.collapsed = input.collapsed;
                    block.table_width = input.table_width;
                    block.has_column_header = input.has_column_header;
                    block.has_row_header = input.has_row_header;
                    block.cells = input.cells;
                    block.child_page_title = input.child_page_title;
                    block.last_edited_time = now_iso();
                    block.last_edited_by = Some(actor);
                    block.to_dto(&page_id)
                };
                if let Err(error) = self.write(ChangeOp::Update, &page_id) {
                    return fail(v2_error(
                        500,
                        CODE_INTERNAL,
                        error.to_string(),
                        request_id,
                        None,
                    ));
                }
                ok(json(200, Envelope::ok(dto)))
            }
            (Method::Delete, false) => {
                let Some(page_id) = self.workspace.page_id_for_block(id) else {
                    return fail(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "block not found",
                        request_id,
                        None,
                    ));
                };
                let actor = self.collab.owner_id();
                let dto = {
                    let Some(block) = self.workspace.block_mut(id) else {
                        return fail(v2_error(
                            404,
                            CODE_NOT_FOUND,
                            "block not found",
                            request_id,
                            None,
                        ));
                    };
                    block.in_trash = true;
                    block.last_edited_time = now_iso();
                    block.last_edited_by = Some(actor);
                    block.to_dto(&page_id)
                };
                if let Err(error) = self.write(ChangeOp::Update, &page_id) {
                    return fail(v2_error(
                        500,
                        CODE_INTERNAL,
                        error.to_string(),
                        request_id,
                        None,
                    ));
                }
                ok(json(200, Envelope::ok(dto)))
            }
            _ => fail(v2_error(
                404,
                CODE_NOT_FOUND,
                "unknown route",
                request_id,
                None,
            )),
        }
    }
}
