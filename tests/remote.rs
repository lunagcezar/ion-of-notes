//! Remote HTTP/WebSocket transport tests (spec-044).

mod common;

use ion_of_notes::app::DesktopApi;
use ion_of_notes::auth::{Account, AuthStore, MemoryAuthStore, hash_password};
use ion_of_notes::client::Client;
use ion_of_notes::collab::{ChangeOp, CollabState, Role};
use ion_of_notes::model::{PageDraft, Workspace, now_iso};
use ion_of_notes::remote::{
    RemoteApi, RemoteRest, RemoteSession, clear_session_at, load_session_at, login, save_session_at,
};
use ion_of_notes::server::WorkspaceServer;
use ion_of_notes::server::realtime::{Awareness, ServerMessage};
use ion_of_notes::server::web::router;
use ion_of_notes::store::{NoCollabStore, NoStore};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

async fn start() -> (String, JoinHandle<()>) {
    start_server(WorkspaceServer::new(Workspace::seed("remote-tests"))).await
}

async fn start_server(server: WorkspaceServer) -> (String, JoinHandle<()>) {
    let app = router(server);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{addr}"), handle)
}

fn remote_client(base: &str) -> Client<DesktopApi> {
    let remote = RemoteApi::connect(base, "owner-token").expect("remote connect");
    Client::with_token(DesktopApi::Remote(remote), "owner-token")
}

#[test]
fn remote_session_roundtrip_and_clear() {
    let path = common::temp_path("remote-session", "json");
    common::remove(&path);
    let session = RemoteSession {
        base_url: "http://127.0.0.1:8787".to_string(),
        email: "owner@workspace.local".to_string(),
        token: "sess_test".to_string(),
    };
    save_session_at(&path, &session).expect("save session");
    assert_eq!(
        load_session_at(&path, "http://127.0.0.1:8787"),
        Some(session.clone())
    );
    assert_eq!(load_session_at(&path, "http://other.example"), None);

    clear_session_at(&path).expect("clear session");
    assert_eq!(load_session_at(&path, "http://127.0.0.1:8787"), None);
    common::remove(&path);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn login_returns_a_session_token() {
    let workspace = Workspace::seed("remote-tests");
    let collab = CollabState::new();
    let owner = collab
        .members
        .iter()
        .find(|member| member.role == Role::Owner)
        .expect("owner");
    let auth = MemoryAuthStore::default();
    auth.upsert_account(&Account {
        id: "account-owner".to_string(),
        member_id: owner.id.clone(),
        email: owner.email.clone(),
        password_hash: hash_password("secret").expect("hash"),
        created_at: now_iso(),
    })
    .expect("account");
    let server =
        WorkspaceServer::with_stores_and_auth(workspace, collab, NoStore, NoCollabStore, auth);
    let (base, handle) = start_server(server).await;

    let session = login(&base, "owner@workspace.local", "secret").expect("login");
    assert!(session.token.starts_with("sess_"));
    assert!(login(&base, "owner@workspace.local", "wrong").is_err());

    handle.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_client_rest_and_crdt_roundtrip() {
    let (base, handle) = start().await;
    let mut client = remote_client(&base);

    let workspace = client.workspace().expect("workspace");
    assert_eq!(workspace.name, "remote-tests");

    let page = client
        .create_page_v2(&PageDraft {
            title: "Remote".to_string(),
            icon: "R".to_string(),
            parent: None,
        })
        .expect("create page");
    assert_eq!(client.get_page_v2(&page.id).unwrap().title, "Remote");

    let (update, state_vector) = client.crdt_sync(&page.id, None).expect("crdt sync");
    assert!(!update.is_empty());
    assert!(!state_vector.is_empty());
    let merged = client
        .apply_crdt_update(&page.id, &update, "m-owner")
        .expect("crdt update");
    assert!(!merged.is_empty());

    handle.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_client_receives_changes_and_awareness() {
    let (base, handle) = start().await;
    let mut client = remote_client(&base);
    let mut realtime = client.subscribe_realtime().expect("realtime");

    let ready = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let Ok(ServerMessage::Ready { .. }) = realtime.recv().await {
                break;
            }
        }
    })
    .await;
    assert!(ready.is_ok(), "websocket did not become ready");

    let page = client
        .create_page_v2(&PageDraft {
            title: "Live remote".to_string(),
            icon: "L".to_string(),
            parent: None,
        })
        .expect("create page");

    let change = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let Ok(ServerMessage::Change { change }) = realtime.recv().await
                && change.page_id == page.id
            {
                break change;
            }
        }
    })
    .await
    .expect("change timeout");
    assert_eq!(change.op, ChangeOp::Create);

    client
        .publish_awareness(&Awareness {
            session_id: "remote-test".to_string(),
            member_id: "m-owner".to_string(),
            name: "Remote Tester".to_string(),
            color: "#4C8BF5".to_string(),
            page_id: page.id.clone(),
            block_id: page.blocks[0].id.clone(),
            index: 1,
            selection_end: None,
        })
        .expect("awareness");

    let awareness = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let Ok(ServerMessage::Awareness { members }) = realtime.recv().await
                && members
                    .iter()
                    .any(|member| member.name == "Remote Tester" && member.index == 1)
            {
                break members;
            }
        }
    })
    .await
    .expect("awareness timeout");
    assert!(
        awareness
            .iter()
            .any(|member| member.page_id == page.id && member.block_id == page.blocks[0].id)
    );

    handle.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_rest_client_roundtrips_without_the_websocket() {
    let (base, handle) = start().await;
    let rest = RemoteRest::connect(&base, "owner-token");
    let mut client = Client::with_token(rest, "owner-token");
    let workspace = client.workspace().expect("workspace");
    let page_id = workspace.pages[0].id.clone();
    let page = client.get_page(&page_id).expect("page");
    assert_eq!(page.id, page_id);
    let results = client.search("Home").expect("search");
    assert!(!results.is_empty());
    handle.abort();
}

#[cfg(unix)]
#[test]
fn saved_session_is_private_on_unix() {
    use std::os::unix::fs::PermissionsExt;

    let path = common::temp_path("remote-session-permissions", "json");
    save_session_at(
        &path,
        &RemoteSession {
            base_url: "http://127.0.0.1:8787".to_string(),
            email: "owner@workspace.local".to_string(),
            token: "secret-token".to_string(),
        },
    )
    .expect("save");
    let mode = std::fs::metadata(&path)
        .expect("metadata")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600);
    common::remove(&path);
}
