//! Accounts, password hashing and opaque sessions (spec-039).
//!
//! This module is transport-neutral. The `AuthStore` port is implemented by
//! the SQLite adapter and by an in-memory adapter for tests; password and token
//! helpers are pure functions.

use argon2::Argon2;
use argon2::password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io;
use std::sync::{Arc, Mutex};

/// Development default used when `ION_OF_NOTES_OWNER_PASSWORD` is not set.
/// A production deployment must set the environment variable.
pub const DEFAULT_OWNER_PASSWORD: &str = "change-me-now";

/// Session lifetime in days.
pub const SESSION_DAYS: i64 = 30;

/// One local account. The password is an Argon2id PHC string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub member_id: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: String,
}

/// One opaque session. Only the SHA-256 token hash is persisted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    pub token_hash: String,
    pub member_id: String,
    pub created_at: String,
    pub expires_at: String,
}

/// `POST /api/v2/auth/login` body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoginInput {
    pub email: String,
    pub password: String,
}

/// Login response. The raw token is returned exactly once.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionResponse {
    pub token: String,
    pub expires_at: String,
    pub member_id: String,
    pub email: String,
    pub name: String,
}

/// `GET /api/v2/auth/session` response (no raw token).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionInfo {
    pub member_id: String,
    pub email: String,
    pub name: String,
    pub expires_at: String,
}

/// Persistence port for accounts and sessions.
pub trait AuthStore: Send + Sync {
    fn find_account(&self, email: &str) -> io::Result<Option<Account>>;
    fn upsert_account(&self, account: &Account) -> io::Result<()>;
    fn save_session(&self, session: &Session) -> io::Result<()>;
    fn find_session(&self, token_hash: &str) -> io::Result<Option<Session>>;
    fn delete_session(&self, token_hash: &str) -> io::Result<()>;
    fn prune_sessions(&self, now: &str) -> io::Result<()>;
}

/// Null adapter: the demo owner token remains the only credential.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoAuthStore;

impl AuthStore for NoAuthStore {
    fn find_account(&self, _email: &str) -> io::Result<Option<Account>> {
        Ok(None)
    }

    fn upsert_account(&self, _account: &Account) -> io::Result<()> {
        Ok(())
    }

    fn save_session(&self, _session: &Session) -> io::Result<()> {
        Ok(())
    }

    fn find_session(&self, _token_hash: &str) -> io::Result<Option<Session>> {
        Ok(None)
    }

    fn delete_session(&self, _token_hash: &str) -> io::Result<()> {
        Ok(())
    }

    fn prune_sessions(&self, _now: &str) -> io::Result<()> {
        Ok(())
    }
}

/// In-memory adapter for tests and previews.
#[derive(Debug, Clone, Default)]
pub struct MemoryAuthStore {
    accounts: Arc<Mutex<Vec<Account>>>,
    sessions: Arc<Mutex<Vec<Session>>>,
}

impl AuthStore for MemoryAuthStore {
    fn find_account(&self, email: &str) -> io::Result<Option<Account>> {
        Ok(self
            .accounts
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .find(|account| account.email.eq_ignore_ascii_case(email))
            .cloned())
    }

    fn upsert_account(&self, account: &Account) -> io::Result<()> {
        let mut accounts = self
            .accounts
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        accounts.retain(|existing| {
            existing.id != account.id && !existing.email.eq_ignore_ascii_case(&account.email)
        });
        accounts.push(account.clone());
        Ok(())
    }

    fn save_session(&self, session: &Session) -> io::Result<()> {
        let mut sessions = self
            .sessions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        sessions.retain(|existing| existing.token_hash != session.token_hash);
        sessions.push(session.clone());
        Ok(())
    }

    fn find_session(&self, token_hash: &str) -> io::Result<Option<Session>> {
        Ok(self
            .sessions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .find(|session| session.token_hash == token_hash)
            .cloned())
    }

    fn delete_session(&self, token_hash: &str) -> io::Result<()> {
        self.sessions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .retain(|session| session.token_hash != token_hash);
        Ok(())
    }

    fn prune_sessions(&self, now: &str) -> io::Result<()> {
        self.sessions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .retain(|session| !session_is_expired_at(session, now));
        Ok(())
    }
}

/// Hash a password with Argon2id (PHC string, random salt).
pub fn hash_password(password: &str) -> Result<String, String> {
    Argon2::default()
        .hash_password(password.as_bytes())
        .map(|hash| hash.to_string())
        .map_err(|error| error.to_string())
}

/// Verify a password against an Argon2id PHC string.
pub fn verify_password(password_hash: &str, password: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(password_hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

/// Create a new opaque session token (`sess_<uuid>`).
pub fn new_session_token() -> String {
    format!("sess_{}", uuid::Uuid::new_v4().simple())
}

/// Hash a session token for storage and lookup.
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    to_hex(&hasher.finalize())
}

/// Expiry timestamp for a new session.
pub fn session_expiry() -> String {
    (chrono::Utc::now() + chrono::Duration::days(SESSION_DAYS)).to_rfc3339()
}

/// Whether a session has expired relative to `now` (RFC 3339).
pub fn session_is_expired_at(session: &Session, now: &str) -> bool {
    let expires = chrono::DateTime::parse_from_rfc3339(&session.expires_at);
    let now = chrono::DateTime::parse_from_rfc3339(now);
    match (expires, now) {
        (Ok(expires), Ok(now)) => expires <= now,
        _ => true,
    }
}

/// Whether a session has expired now.
pub fn session_is_expired(session: &Session) -> bool {
    session_is_expired_at(session, &crate::model::now_iso())
}

fn to_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}
