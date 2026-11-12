//! Saved-view CRUD, query and v2 route use cases.

use super::super::contract::{
    CODE_INTERNAL, CODE_INVALID_JSON, CODE_NOT_FOUND, CODE_VALIDATION_ERROR,
};
use super::super::error::{HandlerResult, fail, ok};
use super::super::pagination::{PageRequest, paginate};
use super::super::response::{json, parse_body, query_value, v2_error};
use super::super::{ApiRequest, ApiResponse, Envelope, Method, WorkspaceServer};
use crate::collab::ChangeOp;
use crate::model::{View, ViewConfig, ViewInput, ViewQueryInput, ViewQueryResult, now_iso};
use crate::webhooks::WebhookEventType;

impl WorkspaceServer {
    pub(in crate::server) fn handle_v2_view(
        &mut self,
        request: &ApiRequest,
        path: &str,
        query: &str,
        request_id: &str,
    ) -> HandlerResult<Option<ApiResponse>> {
        if path == "/api/v2/views" && request.method == Method::Get {
            let data_source = query_value(query, "data_source");
            let views: Vec<View> = self
                .workspace
                .views
                .iter()
                .filter(|view| {
                    data_source.is_empty()
                        || view.data_source_id.as_deref() == Some(data_source.as_str())
                })
                .cloned()
                .collect();
            return ok(json(200, Envelope::ok(views)));
        }
        if path == "/api/v2/views" && request.method == Method::Post {
            let input = match parse_body::<ViewInput>(request) {
                Ok(input) => input,
                Err(message) => {
                    return fail(v2_error(400, CODE_INVALID_JSON, message, request_id, None));
                }
            };
            let Some(data_source_id) = input.data_source_id.clone() else {
                return fail(v2_error(
                    400,
                    CODE_VALIDATION_ERROR,
                    "data_source_id is required",
                    request_id,
                    None,
                ));
            };
            let Some(data_source) = self.workspace.data_source(&data_source_id) else {
                return fail(v2_error(
                    400,
                    CODE_VALIDATION_ERROR,
                    "data source not found",
                    request_id,
                    None,
                ));
            };
            let now = now_iso();
            let view = View {
                id: format!("view-{}", uuid::Uuid::new_v4()),
                parent: data_source.parent.clone(),
                data_source_id: Some(data_source_id),
                name: if input.name.trim().is_empty() {
                    "View".to_string()
                } else {
                    input.name
                },
                r#type: input.r#type,
                filter: input.filter,
                sorts: input.sorts,
                configuration: input.configuration.or(Some(ViewConfig::Table)),
                created_time: now.clone(),
                created_by: None,
                last_edited_time: now,
                last_edited_by: None,
                url: String::new(),
            };
            self.workspace.views.push(view.clone());
            if let Err(error) = self.write(ChangeOp::Create, "__view__") {
                return fail(v2_error(
                    500,
                    CODE_INTERNAL,
                    error.to_string(),
                    request_id,
                    None,
                ));
            }
            self.emit_webhook(
                WebhookEventType::ViewCreated,
                "view",
                &view.id,
                serde_json::json!({ "data_source_id": view.data_source_id }).to_string(),
            );
            return ok(json(201, Envelope::ok(view)));
        }

        let Some(rest) = path.strip_prefix("/api/v2/views/") else {
            return Ok(None);
        };
        if rest.is_empty() {
            return fail(v2_error(
                404,
                CODE_NOT_FOUND,
                "view not found",
                request_id,
                None,
            ));
        }
        if let Some(id) = rest.strip_suffix("/query")
            && request.method == Method::Post
        {
            let Some(view) = self.workspace.view(id) else {
                return fail(v2_error(
                    404,
                    CODE_NOT_FOUND,
                    "view not found",
                    request_id,
                    None,
                ));
            };
            let input = if request.body.is_some() {
                match parse_body::<ViewQueryInput>(request) {
                    Ok(input) => input,
                    Err(message) => {
                        return fail(v2_error(400, CODE_INVALID_JSON, message, request_id, None));
                    }
                }
            } else {
                ViewQueryInput::default()
            };
            let filter = input.filter.as_ref().or(view.filter.as_ref());
            let sorts = input.sorts.as_deref().or(view.sorts.as_deref());
            let rows = self.workspace.query_view_rows(view, filter, sorts);
            let page_request = match PageRequest::from_query(query) {
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
            let sort = format!("view_query:{}", view.id);
            let page = match paginate(&rows, &page_request, &sort, "page") {
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
            let groups = self.workspace.group_view_rows(view, &rows);
            let result =
                ViewQueryResult::new(view, groups, page.results, page.has_more, page.next_cursor);
            return ok(json(200, Envelope::ok(result)));
        }

        match request.method {
            Method::Get => match self.workspace.view(rest) {
                Some(view) => ok(json(200, Envelope::ok(view.clone()))),
                None => fail(v2_error(
                    404,
                    CODE_NOT_FOUND,
                    "view not found",
                    request_id,
                    None,
                )),
            },
            Method::Patch => {
                let input = match parse_body::<ViewInput>(request) {
                    Ok(input) => input,
                    Err(message) => {
                        return fail(v2_error(400, CODE_INVALID_JSON, message, request_id, None));
                    }
                };
                let new_parent = input.data_source_id.as_ref().and_then(|data_source_id| {
                    self.workspace
                        .data_source(data_source_id)
                        .map(|data_source| (data_source_id.clone(), data_source.parent.clone()))
                });
                let dto = {
                    let Some(view) = self.workspace.view_mut(rest) else {
                        return fail(v2_error(
                            404,
                            CODE_NOT_FOUND,
                            "view not found",
                            request_id,
                            None,
                        ));
                    };
                    if !input.name.trim().is_empty() {
                        view.name = input.name;
                    }
                    view.r#type = input.r#type;
                    view.filter = input.filter;
                    view.sorts = input.sorts;
                    if input.configuration.is_some() {
                        view.configuration = input.configuration;
                    }
                    if let Some((data_source_id, parent)) = new_parent {
                        view.data_source_id = Some(data_source_id);
                        view.parent = parent;
                    }
                    view.last_edited_time = now_iso();
                    view.clone()
                };
                if let Err(error) = self.write(ChangeOp::Update, "__view__") {
                    return fail(v2_error(
                        500,
                        CODE_INTERNAL,
                        error.to_string(),
                        request_id,
                        None,
                    ));
                }
                self.emit_webhook(
                    WebhookEventType::ViewUpdated,
                    "view",
                    &dto.id,
                    serde_json::json!({ "data_source_id": dto.data_source_id }).to_string(),
                );
                ok(json(200, Envelope::ok(dto)))
            }
            Method::Delete => {
                if self.workspace.view(rest).is_none() {
                    return fail(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "view not found",
                        request_id,
                        None,
                    ));
                }
                let deleted_id = rest.to_string();
                self.workspace.views.retain(|view| view.id != rest);
                if let Err(error) = self.write(ChangeOp::Delete, "__view__") {
                    return fail(v2_error(
                        500,
                        CODE_INTERNAL,
                        error.to_string(),
                        request_id,
                        None,
                    ));
                }
                self.emit_webhook(
                    WebhookEventType::ViewDeleted,
                    "view",
                    &deleted_id,
                    serde_json::json!({}).to_string(),
                );
                ok(json(200, Envelope::ok(deleted_id)))
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
