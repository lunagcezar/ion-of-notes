//! REST CRDT sync routes used by remote clients.

use super::super::contract::{CODE_INVALID_JSON, CODE_VALIDATION_ERROR};
use super::super::response::{json, parse_body, v2_error};
use super::super::{
    ApiRequest, ApiResponse, Connection, CrdtSyncInput, CrdtSyncOutput, CrdtUpdateInput,
    CrdtUpdateOutput, Envelope, Method, WorkspaceServer,
};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;

impl WorkspaceServer {
    /// REST CRDT routes used by remote clients (the desktop can also use the
    /// in-process methods directly).
    pub(in crate::server) fn handle_v2_crdt(
        &mut self,
        request: &ApiRequest,
        path: &str,
        request_id: &str,
        connection: &Connection,
    ) -> Option<ApiResponse> {
        if path == "/api/v2/crdt/sync" && request.method == Method::Post {
            let input = match parse_body::<CrdtSyncInput>(request) {
                Ok(input) => input,
                Err(message) => {
                    return Some(v2_error(400, CODE_INVALID_JSON, message, request_id, None));
                }
            };
            let state_vector = match input.state_vector {
                Some(value) => match BASE64.decode(value) {
                    Ok(bytes) => Some(bytes),
                    Err(error) => {
                        return Some(v2_error(
                            400,
                            CODE_VALIDATION_ERROR,
                            format!("invalid state_vector: {error}"),
                            request_id,
                            None,
                        ));
                    }
                },
                None => None,
            };
            return Some(
                match self.crdt_sync(&input.page_id, state_vector.as_deref()) {
                    Ok((update, state_vector)) => json(
                        200,
                        Envelope::ok(CrdtSyncOutput {
                            update: BASE64.encode(update),
                            state_vector: BASE64.encode(state_vector),
                        }),
                    ),
                    Err(message) => v2_error(400, CODE_VALIDATION_ERROR, message, request_id, None),
                },
            );
        }
        if path == "/api/v2/crdt/update" && request.method == Method::Post {
            let input = match parse_body::<CrdtUpdateInput>(request) {
                Ok(input) => input,
                Err(message) => {
                    return Some(v2_error(400, CODE_INVALID_JSON, message, request_id, None));
                }
            };
            let update = match BASE64.decode(input.update) {
                Ok(bytes) => bytes,
                Err(error) => {
                    return Some(v2_error(
                        400,
                        CODE_VALIDATION_ERROR,
                        format!("invalid update: {error}"),
                        request_id,
                        None,
                    ));
                }
            };
            let actor = input.actor.unwrap_or_else(|| connection.member_id.clone());
            return Some(
                match self.apply_crdt_update(&input.page_id, &update, &actor) {
                    Ok(merged) => json(
                        200,
                        Envelope::ok(CrdtUpdateOutput {
                            update: BASE64.encode(merged),
                        }),
                    ),
                    Err(message) => v2_error(400, CODE_VALIDATION_ERROR, message, request_id, None),
                },
            );
        }
        None
    }
}
