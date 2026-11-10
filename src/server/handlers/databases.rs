//! Database, data-source and page-property v2 route use cases.

use super::super::contract::{
    CODE_INTERNAL, CODE_INVALID_JSON, CODE_NOT_FOUND, CODE_VALIDATION_ERROR,
};
use super::super::pagination::PageRequest;
use super::super::response::{json, parse_body, v2_error, v2_list_response};
use super::super::{ApiRequest, ApiResponse, Envelope, Method, WorkspaceServer};
use crate::collab::ChangeOp;
use crate::model::{
    DataSource, DataSourceInput, DataSourceRef, Database, DatabaseInput, PagePropertiesInput,
    ParentDto, RichText, now_iso, now_label,
};

impl WorkspaceServer {
    pub(in crate::server) fn handle_v2_database(
        &mut self,
        request: &ApiRequest,
        path: &str,
        query: &str,
        request_id: &str,
    ) -> Option<ApiResponse> {
        // Page properties: /api/v2/pages/{id}/properties
        if let Some(page_id) = path
            .strip_prefix("/api/v2/pages/")
            .and_then(|rest| rest.strip_suffix("/properties"))
            && !page_id.is_empty()
            && request.method == Method::Get
        {
            let Some(page) = self.workspace.page_or_row(page_id) else {
                return Some(v2_error(
                    404,
                    CODE_NOT_FOUND,
                    "page not found",
                    request_id,
                    None,
                ));
            };
            return Some(json(200, Envelope::ok(page.properties.clone())));
        }

        // Page property write: /api/v2/pages/{id}/properties
        if let Some(page_id) = path
            .strip_prefix("/api/v2/pages/")
            .and_then(|rest| rest.strip_suffix("/properties"))
            && !page_id.is_empty()
            && request.method == Method::Patch
        {
            let input = match parse_body::<PagePropertiesInput>(request) {
                Ok(input) => input,
                Err(message) => {
                    return Some(v2_error(400, CODE_INVALID_JSON, message, request_id, None));
                }
            };
            let dto = {
                let Some(page) = self.workspace.page_or_row_mut(page_id) else {
                    return Some(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "page not found",
                        request_id,
                        None,
                    ));
                };
                for (key, property) in input.properties {
                    page.properties.insert(key, property);
                }
                page.updated_at = now_label();
                page.clone()
            };
            if let Err(error) = self.write(ChangeOp::Update, page_id) {
                return Some(v2_error(
                    500,
                    CODE_INTERNAL,
                    error.to_string(),
                    request_id,
                    None,
                ));
            }
            return Some(json(200, Envelope::ok(dto)));
        }

        // Page property item: /api/v2/pages/{id}/properties/{property_id}
        if let Some(rest) = path.strip_prefix("/api/v2/pages/")
            && let Some((page_id, property_id)) = rest.split_once("/properties/")
            && !page_id.is_empty()
            && !property_id.is_empty()
            && request.method == Method::Get
        {
            let Some(page) = self.workspace.page_or_row(page_id) else {
                return Some(v2_error(
                    404,
                    CODE_NOT_FOUND,
                    "page not found",
                    request_id,
                    None,
                ));
            };
            let Some(property) = page.properties.get(property_id).or_else(|| {
                page.properties
                    .values()
                    .find(|property| property.id == property_id)
            }) else {
                return Some(v2_error(
                    404,
                    CODE_NOT_FOUND,
                    "property not found",
                    request_id,
                    None,
                ));
            };
            let page_request = match PageRequest::from_query(query) {
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
            let items = property.items();
            let sort = format!("page_property:{page_id}:{property_id}");
            return Some(v2_list_response(
                items,
                &page_request,
                &sort,
                "property_item",
                request_id,
            ));
        }

        let database_rest = path.strip_prefix("/api/v2/databases/");
        let data_source_rest = path.strip_prefix("/api/v2/data_sources/");

        match (request.method, path, database_rest, data_source_rest) {
            (Method::Get, "/api/v2/databases", _, _) => {
                Some(json(200, Envelope::ok(self.workspace.databases.clone())))
            }
            (Method::Post, "/api/v2/databases", _, _) => {
                let input = match parse_body::<DatabaseInput>(request) {
                    Ok(input) => input,
                    Err(message) => {
                        return Some(v2_error(400, CODE_INVALID_JSON, message, request_id, None));
                    }
                };
                if input.title.trim().is_empty() {
                    return Some(v2_error(
                        400,
                        CODE_VALIDATION_ERROR,
                        "database title must not be empty",
                        request_id,
                        None,
                    ));
                }
                let now = now_iso();
                let database = Database {
                    id: format!("db-{}", uuid::Uuid::new_v4()),
                    title: vec![RichText::text(input.title)],
                    data_sources: Vec::new(),
                    parent: input
                        .parent_page_id
                        .map_or(ParentDto::Workspace { workspace: true }, |page_id| {
                            ParentDto::Page { page_id }
                        }),
                    is_inline: input.is_inline,
                    created_time: now.clone(),
                    created_by: None,
                    last_edited_time: now,
                    last_edited_by: None,
                    in_trash: false,
                    url: String::new(),
                };
                self.workspace.databases.push(database.clone());
                if let Err(error) = self.write(ChangeOp::Create, "__database__") {
                    return Some(v2_error(
                        500,
                        CODE_INTERNAL,
                        error.to_string(),
                        request_id,
                        None,
                    ));
                }
                Some(json(201, Envelope::ok(database)))
            }
            (Method::Get, _, Some(id), _) if !id.is_empty() && !id.contains('/') => {
                match self.workspace.database(id) {
                    Some(database) => Some(json(200, Envelope::ok(database.clone()))),
                    None => Some(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "database not found",
                        request_id,
                        None,
                    )),
                }
            }
            (Method::Patch, _, Some(id), _) if !id.is_empty() && !id.contains('/') => {
                let input = match parse_body::<DatabaseInput>(request) {
                    Ok(input) => input,
                    Err(message) => {
                        return Some(v2_error(400, CODE_INVALID_JSON, message, request_id, None));
                    }
                };
                let dto = {
                    let Some(database) = self.workspace.database_mut(id) else {
                        return Some(v2_error(
                            404,
                            CODE_NOT_FOUND,
                            "database not found",
                            request_id,
                            None,
                        ));
                    };
                    if !input.title.trim().is_empty() {
                        database.title = vec![RichText::text(input.title)];
                    }
                    database.is_inline = input.is_inline;
                    if let Some(page_id) = input.parent_page_id {
                        database.parent = ParentDto::Page { page_id };
                    }
                    database.last_edited_time = now_iso();
                    database.clone()
                };
                if let Err(error) = self.write(ChangeOp::Update, "__database__") {
                    return Some(v2_error(
                        500,
                        CODE_INTERNAL,
                        error.to_string(),
                        request_id,
                        None,
                    ));
                }
                Some(json(200, Envelope::ok(dto)))
            }
            (Method::Delete, _, Some(id), _) if !id.is_empty() && !id.contains('/') => {
                let dto = {
                    let Some(database) = self.workspace.database_mut(id) else {
                        return Some(v2_error(
                            404,
                            CODE_NOT_FOUND,
                            "database not found",
                            request_id,
                            None,
                        ));
                    };
                    database.in_trash = true;
                    database.last_edited_time = now_iso();
                    database.clone()
                };
                if let Err(error) = self.write(ChangeOp::Delete, "__database__") {
                    return Some(v2_error(
                        500,
                        CODE_INTERNAL,
                        error.to_string(),
                        request_id,
                        None,
                    ));
                }
                Some(json(200, Envelope::ok(dto)))
            }
            (Method::Get, _, Some(id), _) if id.ends_with("/data_sources") => {
                let database_id = id.trim_end_matches("/data_sources");
                let Some(database) = self.workspace.database(database_id) else {
                    return Some(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "database not found",
                        request_id,
                        None,
                    ));
                };
                let sources: Vec<DataSource> = self
                    .workspace
                    .data_sources
                    .iter()
                    .filter(|data_source| {
                        matches!(
                            &data_source.parent,
                            ParentDto::Database { database_id: parent } if parent == database_id
                        )
                    })
                    .cloned()
                    .collect();
                let _ = database;
                Some(json(200, Envelope::ok(sources)))
            }
            (Method::Post, _, Some(id), _) if id.ends_with("/data_sources") => {
                let database_id = id.trim_end_matches("/data_sources");
                let Some(database) = self.workspace.database(database_id).cloned() else {
                    return Some(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "database not found",
                        request_id,
                        None,
                    ));
                };
                let input = match parse_body::<DataSourceInput>(request) {
                    Ok(input) => input,
                    Err(message) => {
                        return Some(v2_error(400, CODE_INVALID_JSON, message, request_id, None));
                    }
                };
                let now = now_iso();
                let title = if input.title.trim().is_empty() {
                    "Data source".to_string()
                } else {
                    input.title
                };
                let data_source = DataSource {
                    id: format!("ds-{}", uuid::Uuid::new_v4()),
                    title: vec![RichText::text(title)],
                    properties: input.properties,
                    parent: ParentDto::Database {
                        database_id: database_id.to_string(),
                    },
                    database_parent: database.parent.clone(),
                    rows: Vec::new(),
                    created_time: now.clone(),
                    created_by: None,
                    last_edited_time: now,
                    last_edited_by: None,
                    in_trash: false,
                };
                self.workspace.data_sources.push(data_source.clone());
                if let Some(database) = self.workspace.database_mut(database_id) {
                    database.data_sources.push(DataSourceRef {
                        id: data_source.id.clone(),
                        name: data_source
                            .title
                            .iter()
                            .map(RichText::plain_text)
                            .collect::<Vec<_>>()
                            .join(""),
                    });
                    database.last_edited_time = now_iso();
                }
                if let Err(error) = self.write(ChangeOp::Create, "__data_source__") {
                    return Some(v2_error(
                        500,
                        CODE_INTERNAL,
                        error.to_string(),
                        request_id,
                        None,
                    ));
                }
                Some(json(201, Envelope::ok(data_source)))
            }
            (Method::Get, _, _, Some(id)) if !id.is_empty() && !id.contains('/') => {
                match self.workspace.data_source(id) {
                    Some(data_source) => Some(json(200, Envelope::ok(data_source.clone()))),
                    None => Some(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "data source not found",
                        request_id,
                        None,
                    )),
                }
            }
            (Method::Patch, _, _, Some(id)) if !id.is_empty() && !id.contains('/') => {
                let input = match parse_body::<DataSourceInput>(request) {
                    Ok(input) => input,
                    Err(message) => {
                        return Some(v2_error(400, CODE_INVALID_JSON, message, request_id, None));
                    }
                };
                let dto = {
                    let Some(data_source) = self.workspace.data_source_mut(id) else {
                        return Some(v2_error(
                            404,
                            CODE_NOT_FOUND,
                            "data source not found",
                            request_id,
                            None,
                        ));
                    };
                    if !input.title.trim().is_empty() {
                        data_source.title = vec![RichText::text(input.title)];
                    }
                    if !input.properties.is_empty() {
                        data_source.properties = input.properties;
                    }
                    data_source.last_edited_time = now_iso();
                    data_source.clone()
                };
                if let Err(error) = self.write(ChangeOp::Update, "__data_source__") {
                    return Some(v2_error(
                        500,
                        CODE_INTERNAL,
                        error.to_string(),
                        request_id,
                        None,
                    ));
                }
                Some(json(200, Envelope::ok(dto)))
            }
            (Method::Post, _, _, Some(id)) if id.ends_with("/query") => {
                let data_source_id = id.trim_end_matches("/query");
                let Some(data_source) = self.workspace.data_source(data_source_id) else {
                    return Some(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "data source not found",
                        request_id,
                        None,
                    ));
                };
                let page_request = match PageRequest::from_query(query) {
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
                let rows = data_source.rows.clone();
                let sort = format!("data_source_rows:{data_source_id}");
                Some(v2_list_response(
                    rows,
                    &page_request,
                    &sort,
                    "page",
                    request_id,
                ))
            }
            _ => None,
        }
    }
}
