//! Shared response shaping for the v1 and v2 route adapters.

use super::contract::{
    ApiError, CODE_INTERNAL, CODE_INVALID_JSON, CODE_NOT_FOUND, CODE_SERVICE_UNAVAILABLE,
    CODE_VALIDATION_ERROR,
};
use super::pagination::{PageRequest, paginate};
use super::url::url_decode;
use super::{ApiRequest, ApiResponse, Envelope};
use serde::Serialize;
use serde::de::DeserializeOwned;

pub(super) fn json<T: Serialize>(status: u16, envelope: Envelope<T>) -> ApiResponse {
    ApiResponse {
        status,
        body: serde_json::to_string(&envelope)
            .unwrap_or_else(|_| r#"{"data":null,"error":"serialize"}"#.to_string()),
        request_id: None,
    }
}

pub(super) fn bad(message: &str) -> ApiResponse {
    json::<()>(400, Envelope::err(message))
}

pub(super) fn not_found(message: &str) -> ApiResponse {
    json::<()>(404, Envelope::err(message))
}

pub(super) fn server_error(message: &str) -> ApiResponse {
    json::<()>(500, Envelope::err(message))
}

pub(super) fn parse_body<T: DeserializeOwned>(request: &ApiRequest) -> Result<T, String> {
    let body = request
        .body
        .as_deref()
        .ok_or_else(|| "missing request body".to_string())?;
    serde_json::from_str(body).map_err(|e| format!("invalid JSON: {e}"))
}

pub(super) fn query_value(query: &str, key: &str) -> String {
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=')
            && k == key
        {
            return url_decode(v);
        }
    }
    String::new()
}

pub(super) fn v2_error(
    status: u16,
    code: &str,
    message: impl Into<String>,
    request_id: &str,
    additional_data: Option<serde_json::Value>,
) -> ApiResponse {
    let mut error = ApiError::new(status, code, message, request_id);
    if let Some(data) = additional_data {
        error = error.with_data(data);
    }
    ApiResponse {
        status,
        body: serde_json::to_string(&error).unwrap_or_else(|_| {
            r#"{"object":"error","status":500,"code":"internal_server_error"}"#.to_string()
        }),
        request_id: None,
    }
}

/// A saved write whose response could not be built: tell the client to read
/// the object instead of repeating the write (spec-037).
pub(super) fn write_timeout_response(response: &ApiResponse, request_id: &str) -> ApiResponse {
    let committed = serde_json::from_str::<serde_json::Value>(&response.body)
        .ok()
        .and_then(|body| body.get("data")?.get("id")?.as_str().map(str::to_string));
    let mut data = serde_json::json!({
        "retry_after": "1",
        "retry_guidance": [
            "Read the object again to confirm the saved change.",
            "Do not repeat the write."
        ]
    });
    if let Some(id) = committed {
        data["committed_resource_id"] = serde_json::json!(id);
    }
    v2_error(
        503,
        CODE_SERVICE_UNAVAILABLE,
        "The change was saved, but the response could not be built in time. Read the object again instead of repeating the write.",
        request_id,
        Some(data),
    )
}

pub(super) fn map_v1_error(v1: ApiResponse, request_id: &str) -> ApiResponse {
    let message = serde_json::from_str::<Envelope<serde_json::Value>>(&v1.body)
        .ok()
        .and_then(|envelope| envelope.error)
        .unwrap_or_else(|| v1.body.clone());
    let code = match v1.status {
        404 => CODE_NOT_FOUND,
        500 => CODE_INTERNAL,
        _ if message.contains("invalid JSON") || message.contains("missing request body") => {
            CODE_INVALID_JSON
        }
        _ => CODE_VALIDATION_ERROR,
    };
    v2_error(v1.status, code, message, request_id, None)
}

pub(super) fn v2_list_response<T: Clone + Serialize>(
    items: Vec<T>,
    page: &PageRequest,
    sort: &str,
    type_name: &str,
    request_id: &str,
) -> ApiResponse {
    match paginate(&items, page, sort, type_name) {
        Ok(list) => json(200, Envelope::ok(list)),
        Err(error) => v2_error(400, error.code(), error.message(), request_id, None),
    }
}
