//! Yrs CRDT merge and WebSocket sync tests (spec-042).

mod common;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use futures_util::{SinkExt, StreamExt};
use ion_of_notes::model::Workspace;
use ion_of_notes::server::realtime::{ClientMessage, ServerMessage};
use ion_of_notes::server::web::router_with_server;
use ion_of_notes::server::{Method, WorkspaceApi, WorkspaceServer};
use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};
use yrs::updates::decoder::Decode;
use yrs::{Doc, GetString, ReadTxn, StateVector, Text, Transact, Update};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

fn page_and_block(server: &WorkspaceServer) -> (String, String) {
    let page = &server.workspace.pages[0];
    (page.id.clone(), page.blocks[0].id.clone())
}

fn apply(doc: &Doc, update: &[u8]) {
    let mut txn = doc.transact_mut();
    txn.apply_update(Update::decode_v1(update).expect("update"))
        .expect("apply");
}

fn edit(doc: &Doc, block_id: &str, index: usize, text: &str) -> Vec<u8> {
    let block = doc.get_or_insert_text(format!("block:{block_id}"));
    let mut txn = doc.transact_mut();
    block.insert(&mut txn, index as u32, text);
    txn.encode_diff_v1(&StateVector::default())
}

#[test]
fn two_clients_converge_and_the_server_projects_the_text() {
    let mut server = WorkspaceServer::new(Workspace::seed("crdt-tests"));
    let (page_id, block_id) = page_and_block(&server);

    let (initial, _) = server.crdt_sync(&page_id, None).expect("sync");
    let client_a = Doc::new();
    let client_b = Doc::new();
    apply(&client_a, &initial);
    apply(&client_b, &initial);

    let length = client_a
        .get_or_insert_text(format!("block:{block_id}"))
        .get_string(&client_a.transact())
        .len();
    let update_a = edit(&client_a, &block_id, length, " A");
    let update_b = edit(&client_b, &block_id, 0, "B ");

    server
        .apply_crdt_update(&page_id, &update_a, "m-owner")
        .expect("apply A");
    server
        .apply_crdt_update(&page_id, &update_b, "m-owner")
        .expect("apply B");

    apply(&client_a, &update_b);
    apply(&client_b, &update_a);

    let text_a = client_a
        .get_or_insert_text(format!("block:{block_id}"))
        .get_string(&client_a.transact());
    let text_b = client_b
        .get_or_insert_text(format!("block:{block_id}"))
        .get_string(&client_b.transact());
    assert_eq!(text_a, text_b);

    let page = server.workspace.page(&page_id).expect("page");
    let block = page
        .blocks
        .iter()
        .find(|block| block.id == block_id)
        .expect("block");
    assert_eq!(block.text, text_a);
}

async fn start_shared(
    server: Arc<Mutex<WorkspaceServer>>,
) -> (String, String, String, JoinHandle<()>) {
    let (workspace_id, page_id, block_id) = {
        let server = server.lock().unwrap();
        let page = &server.workspace.pages[0];
        (
            server.workspace.settings.id.clone(),
            page.id.clone(),
            page.blocks[0].id.clone(),
        )
    };
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = router_with_server(server);
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (
        format!("ws://{addr}"),
        workspace_id,
        format!("{page_id}:{block_id}"),
        handle,
    )
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

async fn next_crdt_sync(socket: &mut Socket) -> (String, Vec<u8>) {
    loop {
        if let ServerMessage::CrdtSync {
            page_id, update, ..
        } = next_message(socket).await
        {
            return (page_id, BASE64.decode(update).expect("base64 update"));
        }
    }
}

async fn next_crdt_update(socket: &mut Socket) -> (String, Vec<u8>) {
    loop {
        if let ServerMessage::CrdtUpdate {
            page_id, update, ..
        } = next_message(socket).await
        {
            return (page_id, BASE64.decode(update).expect("base64 update"));
        }
    }
}

#[tokio::test]
async fn websocket_crdt_sync_and_update_converge() {
    let shared = Arc::new(Mutex::new(WorkspaceServer::new(Workspace::seed(
        "crdt-tests",
    ))));
    let (base, workspace_id, page_block, handle) = start_shared(shared.clone()).await;
    let (page_id, block_id) = page_block.split_once(':').expect("page:block");

    let (mut socket, _) = connect_async(format!(
        "{base}/ws/workspace/{workspace_id}?token=owner-token"
    ))
    .await
    .expect("connect");

    socket
        .send(Message::Text(
            serde_json::to_string(&ClientMessage::CrdtSync {
                page_id: page_id.to_string(),
                state_vector: None,
            })
            .unwrap()
            .into(),
        ))
        .await
        .expect("sync request");
    let (_, initial) = next_crdt_sync(&mut socket).await;

    let client_a = Doc::new();
    let client_b = Doc::new();
    apply(&client_a, &initial);
    apply(&client_b, &initial);
    let length = client_a
        .get_or_insert_text(format!("block:{block_id}"))
        .get_string(&client_a.transact())
        .len();
    let update = edit(&client_a, block_id, length, " live");

    socket
        .send(Message::Text(
            serde_json::to_string(&ClientMessage::CrdtUpdate {
                page_id: page_id.to_string(),
                update: BASE64.encode(&update),
            })
            .unwrap()
            .into(),
        ))
        .await
        .expect("crdt update");
    let (_, broadcast) = next_crdt_update(&mut socket).await;
    apply(&client_b, &broadcast);

    let text_a = client_a
        .get_or_insert_text(format!("block:{block_id}"))
        .get_string(&client_a.transact());
    let text_b = client_b
        .get_or_insert_text(format!("block:{block_id}"))
        .get_string(&client_b.transact());
    assert_eq!(text_a, text_b);
    assert!(text_a.ends_with(" live"));

    let page = shared
        .lock()
        .unwrap()
        .workspace
        .page(page_id)
        .expect("page")
        .clone();
    let block = page
        .blocks
        .iter()
        .find(|block| block.id == block_id)
        .expect("block");
    assert_eq!(block.text, text_a);

    handle.abort();
}

#[test]
fn crdt_sync_is_available_through_the_server_api() {
    let mut server = WorkspaceServer::new(Workspace::seed("crdt-tests"));
    let (page_id, _) = page_and_block(&server);
    let response = server.handle(&common::v2_request(
        Method::Get,
        &format!("/api/v2/pages/{page_id}"),
        None,
    ));
    assert_eq!(response.status, 200);
    assert!(server.crdt_sync(&page_id, None).is_ok());
}
