//! v2 login, logout and session routes.

use super::super::contract::{
    CODE_CONFLICT, CODE_INTERNAL, CODE_INVALID_JSON, CODE_UNAUTHORIZED, CODE_VALIDATION_ERROR,
};
use super::super::error::{HandlerResult, fail, ok};
use super::super::response::{json, parse_body, v2_error};
use super::super::{ApiRequest, ApiResponse, Connection, Envelope, Method, WorkspaceServer};
use crate::auth::{self, Account, LoginInput, Session, SessionInfo, SessionResponse};
use crate::collab::Member;
use crate::model::now_iso;
use serde::Deserialize;

/// A fixed Argon2id PHC hash used to spend the same time on an unknown email
/// as on a wrong password. Keeping a constant avoids the old fallback to an
/// empty string, which verified instantly and leaked the user's existence.
const DUMMY_PASSWORD_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$eNIxuRokkbdzdfoxHu4L5A$4E/l20JT6e/N/w/QRO3naDcbmqS7GDcU2FvibgIbQOc";

fn dummy_password_hash() -> &'static str {
    DUMMY_PASSWORD_HASH
}

fn validate_new_password(password: &str) -> Result<(), &'static str> {
    if password.chars().count() < 8 {
        return Err("password must be at least 8 characters");
    }
    Ok(())
}

#[derive(Deserialize)]
struct RegisterInput {
    email: String,
    password: String,
    #[serde(default)]
    invite_token: String,
}

#[derive(Deserialize)]
struct ChangePasswordInput {
    current_password: String,
    new_password: String,
}

#[derive(Deserialize)]
struct ResetPasswordInput {
    email: String,
    new_password: String,
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

    /// Create one persisted session and its response payload.
    fn issue_session(&mut self, member: &Member) -> Result<SessionResponse, String> {
        let token = auth::new_session_token();
        let session = Session {
            token_hash: auth::hash_token(&token),
            member_id: member.id.clone(),
            created_at: now_iso(),
            expires_at: auth::session_expiry(),
        };
        self.persistence
            .auth_store
            .save_session(&session)
            .map_err(|error| error.to_string())?;
        Ok(SessionResponse {
            token,
            expires_at: session.expires_at,
            member_id: member.id.clone(),
            email: member.email.clone(),
            name: member.name.clone(),
        })
    }

    /// `POST /api/v2/auth/register`: an invited member claims an account with a
    /// first password and is logged in (spec-152).
    pub(in crate::server) fn register(
        &mut self,
        request: &ApiRequest,
        request_id: &str,
    ) -> ApiResponse {
        let input = match parse_body::<RegisterInput>(request) {
            Ok(input) => input,
            Err(message) => {
                return v2_error(400, CODE_INVALID_JSON, message, request_id, None);
            }
        };
        if !self.requires_auth() {
            return v2_error(
                400,
                CODE_VALIDATION_ERROR,
                "registration is disabled for the local demo server",
                request_id,
                None,
            );
        }
        if let Err(message) = validate_new_password(&input.password) {
            return v2_error(400, CODE_VALIDATION_ERROR, message, request_id, None);
        }
        let email = input.email.trim().to_lowercase();
        let Some(member) = self
            .collab
            .members
            .iter()
            .find(|member| member.email.eq_ignore_ascii_case(&email))
            .cloned()
        else {
            return v2_error(
                403,
                CODE_UNAUTHORIZED,
                "registration is not available for this email",
                request_id,
                None,
            );
        };
        match self.persistence.auth_store.find_account(&email) {
            Ok(Some(_)) => {
                return v2_error(
                    409,
                    CODE_CONFLICT,
                    "an account already exists for this email",
                    request_id,
                    None,
                );
            }
            Ok(None) => {}
            Err(error) => {
                return v2_error(500, CODE_INTERNAL, error.to_string(), request_id, None);
            }
        }
        if !self.collab.invite_valid(&input.invite_token, &email) {
            return v2_error(
                403,
                CODE_UNAUTHORIZED,
                "a valid invitation token is required",
                request_id,
                None,
            );
        }
        let account = Account {
            id: format!("account-{}", uuid::Uuid::new_v4()),
            member_id: member.id.clone(),
            email: email.clone(),
            password_hash: match auth::hash_password(&input.password) {
                Ok(hash) => hash,
                Err(error) => {
                    return v2_error(500, CODE_INTERNAL, error, request_id, None);
                }
            },
            created_at: now_iso(),
        };
        if let Err(error) = self.persistence.auth_store.upsert_account(&account) {
            return v2_error(500, CODE_INTERNAL, error.to_string(), request_id, None);
        }
        self.collab.consume_invite(&input.invite_token, &email);
        if let Err(error) = self.persist_collab() {
            return v2_error(500, CODE_INTERNAL, error.to_string(), request_id, None);
        }
        match self.issue_session(&member) {
            Ok(session) => json(200, Envelope::ok(session)),
            Err(error) => v2_error(500, CODE_INTERNAL, error, request_id, None),
        }
    }

    /// `POST /api/v2/auth/password`: change the caller's password, revoke every
    /// other session and return a fresh session (spec-152).
    pub(in crate::server) fn change_password(
        &mut self,
        request: &ApiRequest,
        connection: &Connection,
        request_id: &str,
    ) -> ApiResponse {
        let input = match parse_body::<ChangePasswordInput>(request) {
            Ok(input) => input,
            Err(message) => {
                return v2_error(400, CODE_INVALID_JSON, message, request_id, None);
            }
        };
        if let Err(message) = validate_new_password(&input.new_password) {
            return v2_error(400, CODE_VALIDATION_ERROR, message, request_id, None);
        }
        let Some(member) = self
            .collab
            .members
            .iter()
            .find(|member| member.id == connection.member_id)
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
        let mut account = match self.persistence.auth_store.find_account(&member.email) {
            Ok(Some(account)) => account,
            Ok(None) => {
                return v2_error(
                    401,
                    CODE_UNAUTHORIZED,
                    "this member has no password account",
                    request_id,
                    None,
                );
            }
            Err(error) => {
                return v2_error(500, CODE_INTERNAL, error.to_string(), request_id, None);
            }
        };
        if !auth::verify_password(&account.password_hash, &input.current_password) {
            return v2_error(
                401,
                CODE_UNAUTHORIZED,
                "current password is incorrect",
                request_id,
                None,
            );
        }
        account.password_hash = match auth::hash_password(&input.new_password) {
            Ok(hash) => hash,
            Err(error) => {
                return v2_error(500, CODE_INTERNAL, error, request_id, None);
            }
        };
        if let Err(error) = self.persistence.auth_store.upsert_account(&account) {
            return v2_error(500, CODE_INTERNAL, error.to_string(), request_id, None);
        }
        if let Err(error) = self.revoke_member_sessions(&member.id) {
            return v2_error(500, CODE_INTERNAL, error.to_string(), request_id, None);
        }
        match self.issue_session(&member) {
            Ok(session) => json(200, Envelope::ok(session)),
            Err(error) => v2_error(500, CODE_INTERNAL, error, request_id, None),
        }
    }

    /// `POST /api/v2/auth/reset`: owner resets a member's password and revokes
    /// that member's sessions (spec-152).
    pub(in crate::server) fn reset_password(
        &mut self,
        request: &ApiRequest,
        connection: &Connection,
        request_id: &str,
    ) -> ApiResponse {
        if !self.is_owner(connection) {
            return v2_error(
                403,
                crate::server::contract::CODE_RESTRICTED_RESOURCE,
                "owner access is required to reset a password",
                request_id,
                None,
            );
        }
        let input = match parse_body::<ResetPasswordInput>(request) {
            Ok(input) => input,
            Err(message) => {
                return v2_error(400, CODE_INVALID_JSON, message, request_id, None);
            }
        };
        if let Err(message) = validate_new_password(&input.new_password) {
            return v2_error(400, CODE_VALIDATION_ERROR, message, request_id, None);
        }
        let email = input.email.trim().to_lowercase();
        let Some(member) = self
            .collab
            .members
            .iter()
            .find(|member| member.email.eq_ignore_ascii_case(&email))
            .cloned()
        else {
            return v2_error(
                404,
                crate::server::contract::CODE_NOT_FOUND,
                "member not found",
                request_id,
                None,
            );
        };
        let mut account = match self.persistence.auth_store.find_account(&email) {
            Ok(Some(account)) => account,
            Ok(None) => {
                return v2_error(
                    404,
                    crate::server::contract::CODE_NOT_FOUND,
                    "account not found",
                    request_id,
                    None,
                );
            }
            Err(error) => {
                return v2_error(500, CODE_INTERNAL, error.to_string(), request_id, None);
            }
        };
        account.password_hash = match auth::hash_password(&input.new_password) {
            Ok(hash) => hash,
            Err(error) => {
                return v2_error(500, CODE_INTERNAL, error, request_id, None);
            }
        };
        if let Err(error) = self.persistence.auth_store.upsert_account(&account) {
            return v2_error(500, CODE_INTERNAL, error.to_string(), request_id, None);
        }
        if let Err(error) = self.revoke_member_sessions(&member.id) {
            return v2_error(500, CODE_INTERNAL, error.to_string(), request_id, None);
        }
        json(
            200,
            Envelope::ok(serde_json::json!({
                "status": "password_reset",
                "member_id": member.id,
            })),
        )
    }

    /// Session routes that need an already-resolved connection.
    pub(in crate::server) fn handle_v2_auth(
        &mut self,
        request: &ApiRequest,
        path: &str,
        request_id: &str,
        connection: &Connection,
    ) -> HandlerResult<Option<ApiResponse>> {
        if path == "/api/v2/auth/password" && request.method == Method::Post {
            return ok(self.change_password(request, connection, request_id));
        }
        if path == "/api/v2/auth/reset" && request.method == Method::Post {
            return ok(self.reset_password(request, connection, request_id));
        }
        if path == "/api/v2/auth/logout" && request.method == Method::Post {
            let token_hash = auth::hash_token(&connection.token);
            if let Err(error) = self.persistence.auth_store.delete_session(&token_hash) {
                return fail(v2_error(
                    500,
                    CODE_INTERNAL,
                    error.to_string(),
                    request_id,
                    None,
                ));
            }
            // The demo owner token is not a persisted session; keep it usable.
            if connection.kind == crate::server::contract::ConnectionKind::Session {
                self.invalidate_session(&connection.token);
                self.runtime
                    .connections
                    .retain(|existing| existing.token != connection.token);
            }
            return ok(json(200, Envelope::ok("logged_out".to_string())));
        }
        if path == "/api/v2/auth/session" && request.method == Method::Get {
            let token_hash = auth::hash_token(&connection.token);
            let session = match self.persistence.auth_store.find_session(&token_hash) {
                Ok(Some(session)) => session,
                Ok(None) => {
                    return fail(v2_error(
                        401,
                        CODE_UNAUTHORIZED,
                        "session not found",
                        request_id,
                        None,
                    ));
                }
                Err(error) => {
                    return fail(v2_error(
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
                return fail(v2_error(
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
                return fail(v2_error(
                    401,
                    CODE_UNAUTHORIZED,
                    "session member not found",
                    request_id,
                    None,
                ));
            };
            return ok(json(
                200,
                Envelope::ok(SessionInfo {
                    member_id: member.id.clone(),
                    email: member.email.clone(),
                    name: member.name.clone(),
                    expires_at: session.expires_at,
                }),
            ));
        }
        Ok(None)
    }
}
