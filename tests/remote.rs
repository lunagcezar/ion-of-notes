//! Remote HTTP/WebSocket transport tests (spec-044).

mod common;

use ion_of_notes::app::DesktopApi;
use ion_of_notes::client::Client;
use ion_of_notes::collab::ChangeOp;
use ion_of_notes::model::{PageDraft, Workspace};
use ion_of_notes::remote::RemoteApi;
use ion_of_notes::server::WorkspaceServer;
use ion_of_notes::server::realtime::{Awareness, ServerMessage};
use ion_of_notes::server::web::router;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

async fn start() -> (String, JoinHandle<()>) {
    let server = WorkspaceServer::new(Workspace::seed("remote-tests"));
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
