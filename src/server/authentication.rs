//! One authentication seam for the HTTP adapters.
//!
//! The v1/loopback path and the v2 pipeline differ in the response shape they
//! produce, not in how a token becomes a `Connection`. Keeping the token
//! parsing and owner fallback here prevents the two adapters from drifting.

use super::contract::{AUTH_HEADER, OWNER_TOKEN, parse_bearer};
use super::{ApiRequest, Connection, WorkspaceServer};

/// Why a request could not be authenticated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthError {
    MissingToken,
    InvalidToken,
}

impl AuthError {
    /// Stable client-facing message used by the adapters.
    pub fn message(self) -> &'static str {
        match self {
            AuthError::MissingToken => "missing or malformed bearer token",
            AuthError::InvalidToken => "invalid bearer token",
        }
    }
}

impl WorkspaceServer {
    /// Authenticate a Bearer token against the session store.
    pub fn authenticate_bearer(&self, request: &ApiRequest) -> Result<Connection, AuthError> {
        let token = request
            .header(AUTH_HEADER)
            .and_then(parse_bearer)
            .ok_or(AuthError::MissingToken)?;
        self.connection_for_token(token)
            .ok_or(AuthError::InvalidToken)
    }

    /// Authenticate a v1/loopback request.
    ///
    /// A production-capable store always requires a session. The local
    /// `NoAuthStore` keeps the in-process demo owner so the desktop flow works
    /// without a login round trip.
    pub fn authenticate_local_or_session(
        &self,
        request: &ApiRequest,
    ) -> Result<Connection, AuthError> {
        if self.requires_auth() {
            return self.authenticate_bearer(request);
        }
        Ok(self.connection_for_token(OWNER_TOKEN).unwrap_or_else(|| {
            Connection::owner(self.collab.owner_id(), self.workspace.settings.id.clone())
        }))
    }
}
