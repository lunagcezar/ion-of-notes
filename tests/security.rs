//! Security regression tests for the authentication boundary (spec-073).

use ion_of_notes::auth::{Account, AuthStore, MemoryAuthStore, hash_password};
use ion_of_notes::collab::CollabState;
use ion_of_notes::model::{Workspace, now_iso};
use ion_of_notes::server::{
    API_VERSION, ApiRequest, ApiResponse, CollabTransport, Envelope, Method, OWNER_TOKEN,
    RequestTransport, WorkspaceServer,
};
use ion_of_notes::store::{NoCollabStore, NoStore};
use std::sync::{Arc, Mutex};
use std::time::Duration;

mod common;

fn owner_account(collab: &CollabState, password: &str) -> Account {
    let owner = collab
        .members
        .iter()
        .find(|member| member.role == ion_of_notes::collab::Role::Owner)
        .expect("owner");
    Account {
        id: format!("account-{}", owner.id),
        member_id: owner.id.clone(),
        email: owner.email.to_lowercase(),
        password_hash: hash_password(password).expect("hash"),
        created_at: now_iso(),
    }
}

fn server_with_account() -> WorkspaceServer {
    let workspace = Workspace::seed("security-tests");
    let collab = CollabState::new();
    let store = MemoryAuthStore::default();
    store
        .upsert_account(&owner_account(&collab, "secret"))
        .expect("seed account");
    WorkspaceServer::with_stores_and_auth(workspace, collab, NoStore, NoCollabStore, store)
}

fn login_request(email: &str, password: &str, client_ip: Option<&str>) -> ApiRequest {
    let body = serde_json::json!({ "email": email, "password": password }).to_string();
    let request = ApiRequest::new(Method::Post, "/api/v2/auth/login", Some(body))
        .with_header("ion-of-notes-version", API_VERSION);
    match client_ip {
        Some(ip) => request.with_client_ip(ip),
        None => request,
    }
}

fn v1_request(token: Option<&str>, path: &str) -> ApiRequest {
    let request = ApiRequest::new(Method::Get, path, None);
    match token {
        Some(token) => request.with_header("authorization", format!("Bearer {token}")),
        None => request,
    }
}

fn login(server: &mut WorkspaceServer, email: &str, password: &str) -> ApiResponse {
    server.handle(&login_request(email, password, Some("127.0.0.1")))
}

fn session_token(response: &ApiResponse) -> String {
    let envelope: Envelope<ion_of_notes::auth::SessionResponse> =
        serde_json::from_str(&response.body).expect("envelope");
    envelope.data.expect("session data").token
}

#[test]
fn v1_requires_a_token_when_auth_is_configured() {
    let mut server = server_with_account();

    let anonymous = server.handle(&v1_request(None, "/api/workspace"));
    assert_eq!(anonymous.status, 401);

    let demo = server.handle(&v1_request(Some(OWNER_TOKEN), "/api/workspace"));
    assert_eq!(demo.status, 401);

    let response = login(&mut server, "owner@workspace.local", "secret");
    assert_eq!(response.status, 200);
    let token = session_token(&response);
    let authorized = server.handle(&v1_request(Some(&token), "/api/workspace"));
    assert_eq!(authorized.status, 200);
}

#[test]
fn v1_stays_public_for_the_local_no_auth_store() {
    let mut server = WorkspaceServer::new(Workspace::seed("local"));
    let response = server.handle(&v1_request(None, "/api/workspace"));
    assert_eq!(response.status, 200);
    let health = server.handle(&v1_request(None, "/health"));
    assert_eq!(health.status, 200);
}

#[test]
fn owner_token_is_not_registered_for_a_real_auth_store() {
    let server = server_with_account();
    assert!(server.connection_for_token(OWNER_TOKEN).is_none());
    let local = WorkspaceServer::new(Workspace::seed("local"));
    assert!(local.connection_for_token(OWNER_TOKEN).is_some());
}

#[test]
fn remote_client_attaches_the_token_to_v1() {
    #[derive(Default)]
    struct RecordingApi {
        last: Arc<Mutex<Option<ApiRequest>>>,
    }

    impl RequestTransport for RecordingApi {
        fn handle(&mut self, request: &ApiRequest) -> ApiResponse {
            *self.last.lock().expect("lock") = Some(request.clone());
            ApiResponse {
                status: 200,
                body: r#"{"data":null,"error":"test"}"#.to_string(),
                request_id: None,
            }
        }
    }

    impl CollabTransport for RecordingApi {
        fn crdt_sync(
            &mut self,
            _page_id: &str,
            _state_vector: Option<&[u8]>,
        ) -> Result<(Vec<u8>, Vec<u8>), String> {
            Err("not available".to_string())
        }

        fn apply_crdt_update(
            &mut self,
            _page_id: &str,
            _update: &[u8],
            _actor: &str,
        ) -> Result<Vec<u8>, String> {
            Err("not available".to_string())
        }

        fn subscribe_realtime(
            &self,
        ) -> Option<tokio::sync::broadcast::Receiver<ion_of_notes::server::realtime::ServerMessage>>
        {
            None
        }

        fn publish_awareness(
            &mut self,
            _awareness: ion_of_notes::server::realtime::Awareness,
        ) -> Result<(), String> {
            Err("not available".to_string())
        }
    }

    let api = RecordingApi::default();
    let last = Arc::clone(&api.last);
    let mut client = ion_of_notes::client::Client::with_token(api, "session-token");
    let _ = client.workspace();
    let request = last.lock().expect("lock").clone().expect("request");
    assert_eq!(
        request.header("authorization"),
        Some("Bearer session-token")
    );
}

#[test]
fn login_is_limited_per_client_and_per_account() {
    let mut server = server_with_account();
    server.configure_login_rate_limits(1, 100, Duration::from_secs(60));

    let first = server.handle(&login_request(
        "owner@workspace.local",
        "wrong",
        Some("10.0.0.1"),
    ));
    assert_eq!(first.status, 401);

    let same_client = server.handle(&login_request(
        "owner@workspace.local",
        "secret",
        Some("10.0.0.1"),
    ));
    assert_eq!(same_client.status, 429);

    let other_client = server.handle(&login_request(
        "owner@workspace.local",
        "secret",
        Some("10.0.0.2"),
    ));
    assert_eq!(other_client.status, 429);

    let other_account = server.handle(&login_request(
        "other@workspace.local",
        "secret",
        Some("10.0.0.1"),
    ));
    assert_eq!(other_account.status, 429);
}

#[test]
fn request_log_is_capped() {
    let mut server = WorkspaceServer::new(Workspace::seed("log-cap"));
    server.set_request_log_cap(3);
    for _ in 0..6 {
        let request = ApiRequest::new(Method::Get, "/api/v2/workspace", None)
            .with_header("ion-of-notes-version", API_VERSION)
            .with_header("authorization", format!("Bearer {OWNER_TOKEN}"));
        let response = server.handle(&request);
        assert_eq!(response.status, 200);
    }
    assert_eq!(server.request_log().len(), 3);
}

#[test]
fn login_does_not_grow_the_connection_list() {
    let mut server = server_with_account();
    let before = server.connection_count();
    for _ in 0..3 {
        let response = login(&mut server, "owner@workspace.local", "secret");
        assert_eq!(response.status, 200);
    }
    assert_eq!(server.connection_count(), before);
}

#[test]
fn owner_password_must_be_explicit_unless_development_is_allowed() {
    assert!(
        ion_of_notes::auth::require_owner_password(None, false).is_err(),
        "production mode must fail closed"
    );
    assert_eq!(
        ion_of_notes::auth::require_owner_password(None, true).expect("development default"),
        ion_of_notes::auth::DEFAULT_OWNER_PASSWORD
    );
    assert_eq!(
        ion_of_notes::auth::require_owner_password(Some("secret".to_string()), false)
            .expect("explicit password"),
        "secret"
    );
}
