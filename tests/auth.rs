//! Session authentication tests (spec-039).

mod common;

use ion_of_notes::auth::{
    Account, AuthStore, LoginInput, MemoryAuthStore, Session, SessionInfo, SessionResponse,
    hash_password, hash_token, new_session_token, read_password_file, verify_password,
};
use ion_of_notes::collab::{CollabState, Role};
use ion_of_notes::model::{Workspace, now_iso};
use ion_of_notes::server::{ApiError, ApiResponse, Method, RequestTransport, WorkspaceServer};
use ion_of_notes::store::{NoCollabStore, NoStore, SqliteStore};

fn owner_account(collab: &CollabState, password: &str) -> Account {
    let owner = collab
        .members
        .iter()
        .find(|member| member.role == Role::Owner)
        .expect("seed owner");
    Account {
        id: format!("account-{}", owner.id),
        member_id: owner.id.clone(),
        email: owner.email.to_lowercase(),
        password_hash: hash_password(password).expect("hash"),
        created_at: now_iso(),
    }
}

fn server_with_account() -> (WorkspaceServer, MemoryAuthStore) {
    let workspace = Workspace::seed("auth-tests");
    let collab = CollabState::new();
    let store = MemoryAuthStore::default();
    store
        .upsert_account(&owner_account(&collab, "secret"))
        .expect("seed account");
    let server = WorkspaceServer::with_stores_and_auth(
        workspace,
        collab,
        NoStore,
        NoCollabStore,
        store.clone(),
    );
    (server, store)
}

fn login(server: &mut WorkspaceServer, email: &str, password: &str) -> ApiResponse {
    let body = serde_json::to_string(&LoginInput {
        email: email.to_string(),
        password: password.to_string(),
    })
    .unwrap();
    server.handle(&common::v2_request(
        Method::Post,
        "/api/v2/auth/login",
        Some(body),
    ))
}

fn error(response: &ApiResponse) -> ApiError {
    serde_json::from_str(&response.body).expect("structured error")
}

#[test]
fn argon2id_hash_roundtrip_rejects_wrong_passwords() {
    let hash = hash_password("secret").expect("hash");
    assert!(hash.starts_with("$argon2id$"));
    assert!(verify_password(&hash, "secret"));
    assert!(!verify_password(&hash, "wrong"));
}

#[test]
fn login_rejects_bad_credentials() {
    let (mut server, _store) = server_with_account();

    let unknown = login(&mut server, "nobody@workspace.local", "secret");
    assert_eq!(unknown.status, 401);
    assert_eq!(error(&unknown).code, "unauthorized");

    let wrong = login(&mut server, "owner@workspace.local", "wrong");
    assert_eq!(wrong.status, 401);
    assert_eq!(error(&wrong).code, "unauthorized");
}

#[test]
fn session_token_authenticates_v2_and_logout_revokes_it() {
    let (mut server, _store) = server_with_account();

    let response = login(&mut server, "owner@workspace.local", "secret");
    assert_eq!(response.status, 200);
    let session: SessionResponse = common::data(&response);
    assert!(session.token.starts_with("sess_"));
    assert!(!session.expires_at.is_empty());
    assert_eq!(session.email, "owner@workspace.local");

    let me = server.handle(&common::v2_request_as(
        &session.token,
        Method::Get,
        "/api/v2/auth/session",
        None,
    ));
    assert_eq!(me.status, 200);
    let info: SessionInfo = common::data(&me);
    assert_eq!(info.member_id, session.member_id);

    let workspace = server.handle(&common::v2_request_as(
        &session.token,
        Method::Get,
        "/api/v2/workspace",
        None,
    ));
    assert_eq!(workspace.status, 200);

    let logout = server.handle(&common::v2_request_as(
        &session.token,
        Method::Post,
        "/api/v2/auth/logout",
        None,
    ));
    assert_eq!(logout.status, 200);

    let after = server.handle(&common::v2_request_as(
        &session.token,
        Method::Get,
        "/api/v2/workspace",
        None,
    ));
    assert_eq!(after.status, 401);
}

#[test]
fn demo_owner_token_survives_logout() {
    let mut server = WorkspaceServer::new(Workspace::seed("auth-tests"));
    let logout = server.handle(&common::v2_request(
        Method::Post,
        "/api/v2/auth/logout",
        None,
    ));
    assert_eq!(logout.status, 200);
    let workspace = server.handle(&common::v2_request(Method::Get, "/api/v2/workspace", None));
    assert_eq!(workspace.status, 200);
}

#[test]
fn session_table_stores_hashes_not_raw_tokens() {
    let store = MemoryAuthStore::default();
    let token = new_session_token();
    let hash = hash_token(&token);
    assert_ne!(token, hash);
    let session = Session {
        token_hash: hash.clone(),
        member_id: "m-owner".to_string(),
        created_at: now_iso(),
        expires_at: (chrono::Utc::now() + chrono::Duration::days(1)).to_rfc3339(),
    };
    store.save_session(&session).expect("save");
    assert!(store.find_session(&hash).expect("find").is_some());
    assert!(store.find_session(&token).expect("find raw").is_none());
}

#[test]
fn session_survives_a_sqlite_reopen() {
    let db = common::temp_path("auth", "sqlite3");
    let workspace = Workspace::seed("auth-tests");
    let collab = CollabState::new();
    let account = owner_account(&collab, "secret");

    let store = SqliteStore::open(&db).expect("open");
    store.persist_workspace(&workspace).expect("workspace");
    store.persist_collab(&collab).expect("collab");
    store.upsert_account(&account).expect("account");
    let mut server = WorkspaceServer::with_stores_and_auth(
        workspace.clone(),
        collab.clone(),
        store.clone(),
        store.clone(),
        store.clone(),
    );

    let response = login(&mut server, "owner@workspace.local", "secret");
    assert_eq!(response.status, 200);
    let session: SessionResponse = common::data(&response);
    drop(server);

    let reopened = SqliteStore::open(&db).expect("reopen");
    let workspace = reopened.load_workspace().expect("load").expect("workspace");
    let collab = reopened.load_collab().expect("load").expect("collab");
    let mut server = WorkspaceServer::with_stores_and_auth(
        workspace,
        collab,
        reopened.clone(),
        reopened.clone(),
        reopened,
    );
    let response = server.handle(&common::v2_request_as(
        &session.token,
        Method::Get,
        "/api/v2/workspace",
        None,
    ));
    assert_eq!(response.status, 200);

    common::remove(&db);
}

#[test]
fn owner_password_file_is_trimmed_and_empty_is_rejected() {
    let path = common::temp_path("owner-password", "txt");
    std::fs::write(&path, "secret\r\n").expect("write secret");
    assert_eq!(read_password_file(&path).expect("read secret"), "secret");

    std::fs::write(&path, "\r\n").expect("write empty secret");
    assert!(read_password_file(&path).is_err());

    common::remove(&path);
}

#[derive(Default)]
struct CountingAuthStore {
    inner: MemoryAuthStore,
    lookups: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl AuthStore for CountingAuthStore {
    fn find_account(&self, email: &str) -> std::io::Result<Option<Account>> {
        self.inner.find_account(email)
    }
    fn upsert_account(&self, account: &Account) -> std::io::Result<()> {
        self.inner.upsert_account(account)
    }
    fn save_session(&self, session: &Session) -> std::io::Result<()> {
        self.inner.save_session(session)
    }
    fn find_session(&self, token_hash: &str) -> std::io::Result<Option<Session>> {
        self.lookups
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.inner.find_session(token_hash)
    }
    fn delete_session(&self, token_hash: &str) -> std::io::Result<()> {
        self.inner.delete_session(token_hash)
    }
    fn delete_member_sessions(&self, member_id: &str) -> std::io::Result<()> {
        self.inner.delete_member_sessions(member_id)
    }
    fn prune_sessions(&self, now: &str) -> std::io::Result<()> {
        self.inner.prune_sessions(now)
    }
}

#[test]
fn resolved_sessions_are_cached_and_invalidated_on_logout() {
    use std::sync::Arc;
    use std::sync::atomic::Ordering;

    let workspace = Workspace::seed("session-cache");
    let collab = CollabState::new();
    let store = CountingAuthStore::default();
    let lookups = Arc::clone(&store.lookups);
    let token = "sess_cache_test";
    store
        .save_session(&Session {
            token_hash: hash_token(token),
            member_id: collab.owner_id(),
            created_at: now_iso(),
            expires_at: (chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339(),
        })
        .expect("session");
    let server =
        WorkspaceServer::with_stores_and_auth(workspace, collab, NoStore, NoCollabStore, store);

    assert!(server.resolve_connection(token).expect("first").is_some());
    assert!(server.resolve_connection(token).expect("cached").is_some());
    assert_eq!(
        lookups.load(Ordering::Relaxed),
        1,
        "second lookup is cached"
    );

    server.invalidate_session(token);
    assert!(
        server
            .resolve_connection(token)
            .expect("after logout")
            .is_some()
    );
    assert_eq!(
        lookups.load(Ordering::Relaxed),
        2,
        "invalidated session goes back to the store"
    );
}

#[test]
fn invited_member_can_register_change_and_reset_password() {
    let (mut server, _store) = server_with_account();
    server.configure_login_rate_limits(100, 100, std::time::Duration::from_secs(60));

    let without_token = serde_json::json!({
        "email": "alex@workspace.local",
        "password": "alexpass1",
    })
    .to_string();
    let response = server.handle(&common::v2_request(
        Method::Post,
        "/api/v2/auth/register",
        Some(without_token),
    ));
    assert_eq!(response.status, 403, "registration needs an invitation");

    let owner = login(&mut server, "owner@workspace.local", "secret");
    assert_eq!(owner.status, 200);
    let owner: SessionResponse = common::data(&owner);
    let invite = server.handle(&common::v2_request_as(
        &owner.token,
        Method::Post,
        "/api/v2/members/m-guest-1/invite",
        None,
    ));
    assert_eq!(invite.status, 201, "{}", invite.body);
    let invite: serde_json::Value = common::data(&invite);
    let invite_token = invite["token"].as_str().expect("invite token").to_string();

    let body = serde_json::json!({
        "email": "alex@workspace.local",
        "password": "alexpass1",
        "invite_token": invite_token,
    })
    .to_string();
    let response = server.handle(&common::v2_request(
        Method::Post,
        "/api/v2/auth/register",
        Some(body),
    ));
    assert_eq!(response.status, 200, "{}", response.body);
    let session: SessionResponse = common::data(&response);
    assert_eq!(session.email, "alex@workspace.local");

    let repeated = serde_json::json!({
        "email": "alex@workspace.local",
        "password": "alexpass1",
        "invite_token": invite_token,
    })
    .to_string();
    let response = server.handle(&common::v2_request(
        Method::Post,
        "/api/v2/auth/register",
        Some(repeated),
    ));
    assert_eq!(response.status, 409, "{}", response.body);

    let change = serde_json::json!({
        "current_password": "alexpass1",
        "new_password": "alexpass2",
    })
    .to_string();
    let response = server.handle(&common::v2_request_as(
        &session.token,
        Method::Post,
        "/api/v2/auth/password",
        Some(change),
    ));
    assert_eq!(response.status, 200, "{}", response.body);
    let fresh: SessionResponse = common::data(&response);
    assert!(
        server
            .resolve_connection(&session.token)
            .expect("resolve")
            .is_none(),
        "changing the password revokes the previous session"
    );

    let owner = login(&mut server, "owner@workspace.local", "secret");
    assert_eq!(owner.status, 200);
    let owner: SessionResponse = common::data(&owner);
    let reset = serde_json::json!({
        "email": "alex@workspace.local",
        "new_password": "alexpass3",
    })
    .to_string();
    let response = server.handle(&common::v2_request_as(
        &owner.token,
        Method::Post,
        "/api/v2/auth/reset",
        Some(reset),
    ));
    assert_eq!(response.status, 200, "{}", response.body);
    assert!(
        server
            .resolve_connection(&fresh.token)
            .expect("resolve")
            .is_none(),
        "owner reset revokes member sessions"
    );
    let final_login = login(&mut server, "alex@workspace.local", "alexpass3");
    assert_eq!(final_login.status, 200, "{}", final_login.body);
}
