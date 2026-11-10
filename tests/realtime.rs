//! Real-time WebSocket change feed and presence tests (spec-040).

mod common;

use futures_util::{SinkExt, StreamExt};
use ion_of_notes::collab::{Change, ChangeOp, realtime_contract};
use ion_of_notes::model::{Page, Workspace};
use ion_of_notes::server::realtime::{ClientMessage, PresenceMember, RealtimeHub, ServerMessage};
use ion_of_notes::server::web::{router, router_with_server};
use ion_of_notes::server::{Method, WorkspaceApi, WorkspaceServer};
use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

fn workspace() -> Workspace {
    Workspace::seed("realtime-tests")
}

async fn start_server(server: WorkspaceServer) -> (String, String, JoinHandle<()>) {
    let workspace_id = server.workspace.settings.id.clone();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = router(server);
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("ws://{addr}"), workspace_id, handle)
}

async fn start_shared(server: Arc<Mutex<WorkspaceServer>>) -> (String, String, JoinHandle<()>) {
    let workspace_id = server.lock().unwrap().workspace.settings.id.clone();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = router_with_server(server);
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("ws://{addr}"), workspace_id, handle)
}

async fn next_message(socket: &mut Socket) -> ServerMessage {
    loop {
        let message = socket
            .next()
            .await
            .expect("socket closed")
            .expect("socket error");
        match message {
            Message::Text(text) => {
                return serde_json::from_str(text.as_str()).expect("server message");
            }
            Message::Ping(_) | Message::Pong(_) | Message::Binary(_) | Message::Frame(_) => {}
            Message::Close(_) => panic!("socket closed"),
        }
    }
}

async fn next_of<F>(socket: &mut Socket, matches: F) -> ServerMessage
where
    F: Fn(&ServerMessage) -> bool,
{
    for _ in 0..20 {
        let message = next_message(socket).await;
        if matches(&message) {
            return message;
        }
    }
    panic!("expected message was not received");
}

#[test]
fn hub_broadcasts_changes_and_presence() {
    let hub = RealtimeHub::new(16);
    let mut receiver = hub.subscribe();

    hub.join(PresenceMember {
        session_id: "s1".to_string(),
        member_id: "m-owner".to_string(),
        name: "Workspace Owner".to_string(),
        state: "active".to_string(),
    });
    let presence = receiver.try_recv().expect("presence");
    match presence {
        ServerMessage::Presence { members } => {
            assert_eq!(members.len(), 1);
            assert_eq!(members[0].session_id, "s1");
        }
        other => panic!("expected presence, got {other:?}"),
    }

    let change = Change {
        seq: 7,
        page_id: "p-home".to_string(),
        op: ChangeOp::Update,
        actor: "m-owner".to_string(),
    };
    hub.publish_change(change.clone());
    match receiver.try_recv().expect("change") {
        ServerMessage::Change { change: received } => assert_eq!(received, change),
        other => panic!("expected change, got {other:?}"),
    }

    hub.leave("s1");
    match receiver.try_recv().expect("leave") {
        ServerMessage::Presence { members } => assert!(members.is_empty()),
        other => panic!("expected presence, got {other:?}"),
    }
}

#[tokio::test]
async fn websocket_rejects_missing_tokens_and_wrong_workspaces() {
    let (base, workspace_id, handle) = start_server(WorkspaceServer::new(workspace())).await;

    let missing = connect_async(format!("{base}/ws/workspace/{workspace_id}")).await;
    match missing {
        Err(tokio_tungstenite::tungstenite::Error::Http(response)) => {
            assert_eq!(response.status().as_u16(), 401);
        }
        other => panic!("expected 401, got {other:?}"),
    }

    let wrong = connect_async(format!(
        "{base}/ws/workspace/not-this-workspace?token=owner-token"
    ))
    .await;
    match wrong {
        Err(tokio_tungstenite::tungstenite::Error::Http(response)) => {
            assert_eq!(response.status().as_u16(), 404);
        }
        other => panic!("expected 404, got {other:?}"),
    }

    handle.abort();
}

#[tokio::test]
async fn websocket_sends_ready_presence_and_pong() {
    let (base, workspace_id, handle) = start_server(WorkspaceServer::new(workspace())).await;
    let (mut socket, _) = connect_async(format!(
        "{base}/ws/workspace/{workspace_id}?token=owner-token"
    ))
    .await
    .expect("connect");

    match next_message(&mut socket).await {
        ServerMessage::Ready {
            workspace_id: ready_id,
            session_id,
            seq,
        } => {
            assert_eq!(ready_id, workspace_id);
            assert!(session_id.starts_with("ws-"));
            assert_eq!(seq, 0);
        }
        other => panic!("expected ready, got {other:?}"),
    }

    let presence = next_of(&mut socket, |message| {
        matches!(message, ServerMessage::Presence { .. })
    })
    .await;
    match presence {
        ServerMessage::Presence { members } => {
            assert!(members.iter().any(|member| member.member_id == "m-owner"));
        }
        other => panic!("expected presence, got {other:?}"),
    }

    socket
        .send(Message::Text(
            serde_json::to_string(&ClientMessage::Ping).unwrap().into(),
        ))
        .await
        .expect("ping");
    let pong = next_of(&mut socket, |message| {
        matches!(message, ServerMessage::Pong)
    })
    .await;
    assert_eq!(pong, ServerMessage::Pong);

    handle.abort();
}

#[tokio::test]
async fn websocket_replays_backlog_after_a_cursor() {
    let mut server = WorkspaceServer::new(workspace());
    let response = server.handle(&common::v2_request(
        Method::Post,
        "/api/v2/pages",
        Some(common::page_draft_json("Backlog", None)),
    ));
    assert_eq!(response.status, 201);
    let page: Page = common::data(&response);

    let (base, workspace_id, handle) = start_server(server).await;
    let (mut socket, _) = connect_async(format!(
        "{base}/ws/workspace/{workspace_id}?token=owner-token&since=0"
    ))
    .await
    .expect("connect");

    assert!(matches!(
        next_message(&mut socket).await,
        ServerMessage::Ready { .. }
    ));
    let change = next_of(&mut socket, |message| {
        matches!(message, ServerMessage::Change { .. })
    })
    .await;
    match change {
        ServerMessage::Change { change } => {
            assert_eq!(change.page_id, page.id);
            assert_eq!(change.op, ChangeOp::Create);
            assert_eq!(change.seq, 1);
        }
        other => panic!("expected change, got {other:?}"),
    }

    handle.abort();
}

#[tokio::test]
async fn websocket_receives_a_live_change_after_a_rest_write() {
    let shared = Arc::new(Mutex::new(WorkspaceServer::new(workspace())));
    let (base, workspace_id, handle) = start_shared(shared.clone()).await;
    let (mut socket, _) = connect_async(format!(
        "{base}/ws/workspace/{workspace_id}?token=owner-token"
    ))
    .await
    .expect("connect");
    assert!(matches!(
        next_message(&mut socket).await,
        ServerMessage::Ready { .. }
    ));
    next_of(&mut socket, |message| {
        matches!(message, ServerMessage::Presence { .. })
    })
    .await;

    let response = shared.lock().unwrap().handle(&common::v2_request(
        Method::Post,
        "/api/v2/pages",
        Some(common::page_draft_json("Live", None)),
    ));
    assert_eq!(response.status, 201);
    let page: Page = common::data(&response);

    let change = next_of(&mut socket, |message| {
        matches!(message, ServerMessage::Change { .. })
    })
    .await;
    match change {
        ServerMessage::Change { change } => {
            assert_eq!(change.page_id, page.id);
            assert_eq!(change.op, ChangeOp::Create);
        }
        other => panic!("expected change, got {other:?}"),
    }

    handle.abort();
}

#[test]
fn realtime_contract_describes_the_running_socket() {
    let contract = realtime_contract();
    assert!(contract.channel.contains("/ws/workspace/"));
    assert!(contract.channel.contains("since="));
    assert!(contract.messages.contains(&"ready".to_string()));
    assert!(contract.messages.contains(&"change".to_string()));
    assert!(contract.messages.contains(&"presence".to_string()));
    assert!(contract.resume.contains("since"));
}
