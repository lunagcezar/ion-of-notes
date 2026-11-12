//! v2 login, logout and session routes.

use super::super::contract::{
    CODE_INTERNAL, CODE_INVALID_JSON, CODE_UNAUTHORIZED, CODE_VALIDATION_ERROR,
};
use super::super::response::{json, parse_body, v2_error};
use super::super::{ApiRequest, ApiResponse, Connection, Envelope, Method, WorkspaceServer};
use crate::auth::{self, LoginInput, Session, SessionInfo, SessionResponse};
use crate::model::now_iso;
use std::sync::OnceLock;

fn dummy_password_hash() -> &'static str {
    static HASH: OnceLock<String> = OnceLock::new();
    HASH.get_or_init(|| {
        auth::hash_password("ion-of-notes-dummy-password").unwrap_or_else(|_| String::new())
    })
}

impl WorkspaceServer {
    /// `POST /api/v2/auth/login`: verify an account and create a session.
    pub(in crate::server) fn login(
        &mut self,
        request: &ApiRequest,
        request_id: &str,
    ) -> ApiResponse {
        let input = match parse_body::<LoginInput>(request) {
            Ok(input) => input,
            Err(message) => {
                return v2_error(400, CODE_INVALID_JSON, message, request_id, None);
            }
        };
        if input.email.trim().is_empty() || input.password.is_empty() {
            return v2_error(
                400,
                CODE_VALIDATION_ERROR,
                "email and password are required",
                request_id,
                None,
            );
        }
        let email = input.email.trim().to_lowercase();
        let _ = self.persistence.auth_store.prune_sessions(&now_iso());
        let account = match self.persistence.auth_store.find_account(&email) {
            Ok(Some(account)) => account,
            Ok(None) => {
                // Verify a dummy hash so an unknown email does not return
                // noticeably faster than a wrong password (user enumeration).
                let _ = auth::verify_password(dummy_password_hash(), &input.password);
                return v2_error(
                    401,
                    CODE_UNAUTHORIZED,
                    "invalid email or password",
                    request_id,
                    None,
                );
            }
            Err(error) => {
                return v2_error(500, CODE_INTERNAL, error.to_string(), request_id, None);
            }
        };
        if !auth::verify_password(&account.password_hash, &input.password) {
            return v2_error(
                401,
                CODE_UNAUTHORIZED,
                "invalid email or password",
                request_id,
                None,
            );
        }
        let Some(member) = self
            .collab
            .members
            .iter()
            .find(|member| member.id == account.member_id)
            .cloned()
        else {
            return v2_error(
                401,
                CODE_UNAUTHORIZED,
                "account has no workspace member",
                request_id,
                None,
            );
        };
        let token = auth::new_session_token();
        let session = Session {
            token_hash: auth::hash_token(&token),
            member_id: member.id.clone(),
            created_at: now_iso(),
            expires_at: auth::session_expiry(),
        };
        if let Err(error) = self.persistence.auth_store.save_session(&session) {
            return v2_error(500, CODE_INTERNAL, error.to_string(), request_id, None);
        }
        // Sessions are resolved from the auth store on every request; do not
        // retain one connection object per login (spec-073).
        json(
            200,
            Envelope::ok(SessionResponse {
                token,
                expires_at: session.expires_at,
                member_id: member.id,
                email: member.email,
                name: member.name,
            }),
        )
    }

    /// Session routes that need an already-resolved connection.
    pub(in crate::server) fn handle_v2_auth(
        &mut self,
        request: &ApiRequest,
        path: &str,
        request_id: &str,
        connection: &Connection,
    ) -> Option<ApiResponse> {
        if path == "/api/v2/auth/logout" && request.method == Method::Post {
            let token_hash = auth::hash_token(&connection.token);
            if let Err(error) = self.persistence.auth_store.delete_session(&token_hash) {
                return Some(v2_error(
                    500,
                    CODE_INTERNAL,
                    error.to_string(),
                    request_id,
                    None,
                ));
            }
            // The demo owner token is not a persisted session; keep it usable.
            if connection.id.starts_with("session-") {
                self.runtime
                    .connections
                    .retain(|existing| existing.token != connection.token);
            }
            return Some(json(200, Envelope::ok("logged_out".to_string())));
        }
        if path == "/api/v2/auth/session" && request.method == Method::Get {
            let token_hash = auth::hash_token(&connection.token);
            let session = match self.persistence.auth_store.find_session(&token_hash) {
                Ok(Some(session)) => session,
                Ok(None) => {
                    return Some(v2_error(
                        401,
                        CODE_UNAUTHORIZED,
                        "session not found",
                        request_id,
                        None,
                    ));
                }
                Err(error) => {
                    return Some(v2_error(
                        500,
                        CODE_INTERNAL,
                        error.to_string(),
                        request_id,
                        None,
                    ));
                }
            };
            if auth::session_is_expired(&session) {
                let _ = self.persistence.auth_store.delete_session(&token_hash);
                return Some(v2_error(
                    401,
                    CODE_UNAUTHORIZED,
                    "session expired",
                    request_id,
                    None,
                ));
            }
            let Some(member) = self
                .collab
                .members
                .iter()
                .find(|member| member.id == session.member_id)
            else {
                return Some(v2_error(
                    401,
                    CODE_UNAUTHORIZED,
                    "session member not found",
                    request_id,
                    None,
                ));
            };
            return Some(json(
                200,
                Envelope::ok(SessionInfo {
                    member_id: member.id.clone(),
                    email: member.email.clone(),
                    name: member.name.clone(),
                    expires_at: session.expires_at,
                }),
            ));
        }
        None
    }
}
