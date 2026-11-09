//! Session authentication tests (spec-039).

mod common;

use ion_of_notes::auth::{
    Account, AuthStore, LoginInput, MemoryAuthStore, Session, SessionInfo, SessionResponse,
    hash_password, hash_token, new_session_token, verify_password,
};
use ion_of_notes::collab::{CollabState, Role};
use ion_of_notes::model::{Workspace, now_iso};
use ion_of_notes::server::{ApiError, ApiResponse, Method, WorkspaceApi, WorkspaceServer};
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
