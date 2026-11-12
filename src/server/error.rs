//! Typed handler errors for the v2 resource adapters.
//!
//! A handler returns `HandlerResult<T>`; the dispatcher attaches the request ID
//! once when it turns the error into an `ApiError` response.

use super::contract::{CODE_INTERNAL, CODE_INVALID_JSON, CODE_NOT_FOUND, CODE_VALIDATION_ERROR};
use super::response::{parse_body, v2_error};
use super::{ApiRequest, ApiResponse};
use serde::de::DeserializeOwned;

#[derive(Debug, Clone)]
pub struct HandlerError {
    status: u16,
    code: &'static str,
    message: String,
    extra: Option<serde_json::Value>,
}

impl HandlerError {
    pub fn new(status: u16, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            extra: None,
        }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(400, CODE_VALIDATION_ERROR, message)
    }

    pub fn invalid_json(message: impl Into<String>) -> Self {
        Self::new(400, CODE_INVALID_JSON, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(404, CODE_NOT_FOUND, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(500, CODE_INTERNAL, message)
    }
}

pub type HandlerResult<T = ApiResponse> = Result<T, HandlerError>;

/// Parse a JSON request body, turning a syntax error into `invalid_json`.
pub fn body<T: DeserializeOwned>(request: &ApiRequest) -> HandlerResult<T> {
    parse_body(request).map_err(HandlerError::invalid_json)
}

/// Turn one handler error into the v2 response, attaching the request ID.
pub fn handler_error(error: HandlerError, request_id: &str) -> ApiResponse {
    v2_error(
        error.status,
        error.code,
        error.message,
        request_id,
        error.extra,
    )
}
