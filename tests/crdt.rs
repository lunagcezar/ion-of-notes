//! Yrs CRDT merge and WebSocket sync tests (spec-042).

mod common;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use futures_util::{SinkExt, StreamExt};
use ion_of_notes::model::{
    Block, BlockKind, PageProperty, PropertyType, PropertyValue, RichText, Workspace,
};
use ion_of_notes::server::crdt::{BlockStructure, RowRegister};
use ion_of_notes::server::realtime::{ClientMessage, ServerMessage};
use ion_of_notes::server::web::router_with_server;
use ion_of_notes::server::{CrdtStore, Method, RequestTransport, WorkspaceServer};
use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};
use yrs::updates::decoder::Decode;
use yrs::{Doc, GetString, Map, ReadTxn, StateVector, Text, Transact, Update};

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

    let mut request = format!("{base}/ws/workspace/{workspace_id}")
        .into_client_request()
        .expect("request");
    request.headers_mut().insert(
        "authorization",
        HeaderValue::from_static("Bearer owner-token"),
    );
    let (mut socket, _) = connect_async(request).await.expect("connect");

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

fn remote_doc(store: &CrdtStore, workspace: &Workspace, page_id: &str) -> Doc {
    let (update, _) = store.sync(workspace, page_id, None).expect("initial sync");
    let doc = Doc::new();
    apply(&doc, &update);
    doc
}

#[test]
fn structure_registers_project_inserts_moves_and_deletes() {
    let workspace = Workspace::seed("crdt-structure");
    let store = CrdtStore::new();
    let remote = remote_doc(&store, &workspace, "p-home");
    let structure = remote.get_or_insert_map("structure");

    let mut inserted = Block::new("crdt-new", BlockKind::Paragraph, "Inserted");
    inserted.children.clear();
    let h1 = workspace.block("h1").expect("h1").clone();
    let mut moved = h1.clone();
    moved.children.clear();
    let update = {
        let mut txn = remote.transact_mut();
        structure.insert(
            &mut txn,
            "crdt-new",
            serde_json::to_string(&BlockStructure {
                parent_id: None,
                position: 0,
                block: inserted,
            })
            .expect("inserted json"),
        );
        structure.insert(
            &mut txn,
            "h1",
            serde_json::to_string(&BlockStructure {
                parent_id: None,
                position: 99,
                block: moved,
            })
            .expect("moved json"),
        );
        structure.remove(&mut txn, "h2");
        txn.encode_diff_v1(&StateVector::default())
    };

    store
        .apply_update(&workspace, "p-home", &update)
        .expect("apply");
    let mut projected = workspace.clone();
    store.project(&mut projected, "p-home").expect("project");
    let page = projected.page("p-home").expect("page");
    assert_eq!(
        page.blocks.first().map(|block| block.id.as_str()),
        Some("crdt-new")
    );
    assert!(page.blocks.iter().any(|block| block.id == "h1"));
    assert!(!page.blocks.iter().any(|block| block.id == "h2"));
    assert_eq!(
        page.blocks.last().map(|block| block.id.as_str()),
        Some("h1")
    );
}

#[test]
fn property_registers_project_and_remove_page_properties() {
    let workspace = Workspace::seed("crdt-properties");
    let store = CrdtStore::new();
    let remote = remote_doc(&store, &workspace, "p-home");
    let properties = remote.get_or_insert_map("properties");
    let property = PageProperty::new(
        "priority",
        PropertyType::RichText,
        PropertyValue::RichText(vec![RichText::text("High")]),
    );

    let update = {
        let mut txn = remote.transact_mut();
        properties.insert(
            &mut txn,
            "priority",
            serde_json::to_string(&property).expect("property json"),
        );
        txn.encode_diff_v1(&StateVector::default())
    };
    store
        .apply_update(&workspace, "p-home", &update)
        .expect("apply");
    let mut projected = workspace.clone();
    store.project(&mut projected, "p-home").expect("project");
    assert_eq!(
        projected
            .page("p-home")
            .expect("page")
            .properties
            .get("priority"),
        Some(&property)
    );

    let update = {
        let mut txn = remote.transact_mut();
        properties.remove(&mut txn, "priority");
        txn.encode_diff_v1(&StateVector::default())
    };
    store
        .apply_update(&projected, "p-home", &update)
        .expect("apply remove");
    store
        .project(&mut projected, "p-home")
        .expect("project remove");
    assert!(
        !projected
            .page("p-home")
            .expect("page")
            .properties
            .contains_key("priority")
    );
}

#[test]
fn row_registers_project_database_cells_and_explicit_deletion() {
    let mut workspace = Workspace::seed("crdt-rows");
    workspace.ensure_data_sources();
    let data_source_id = "ds-p-tasks";
    let row = workspace
        .data_source(data_source_id)
        .expect("data source")
        .rows[0]
        .clone();
    let store = CrdtStore::new();
    let remote = remote_doc(&store, &workspace, "p-tasks");
    let rows = remote.get_or_insert_map("rows");
    let key = format!("{data_source_id}/{}", row.id);
    let mut edited = row.clone();
    edited.properties.insert(
        "assignee".to_string(),
        PageProperty::new(
            "assignee",
            PropertyType::RichText,
            PropertyValue::RichText(vec![RichText::text("Alex")]),
        ),
    );

    let update = {
        let mut txn = remote.transact_mut();
        rows.insert(
            &mut txn,
            key.clone(),
            serde_json::to_string(&RowRegister {
                data_source_id: data_source_id.to_string(),
                page: edited.clone(),
                deleted: false,
            })
            .expect("row json"),
        );
        txn.encode_diff_v1(&StateVector::default())
    };
    store
        .apply_update(&workspace, "p-tasks", &update)
        .expect("apply");
    let mut projected = workspace.clone();
    store.project(&mut projected, "p-tasks").expect("project");
    let saved = projected
        .data_source(data_source_id)
        .expect("data source")
        .rows
        .iter()
        .find(|candidate| candidate.id == row.id)
        .expect("row");
    assert_eq!(
        saved.properties.get("assignee"),
        edited.properties.get("assignee")
    );

    let update = {
        let mut txn = remote.transact_mut();
        rows.insert(
            &mut txn,
            key,
            serde_json::to_string(&RowRegister {
                data_source_id: data_source_id.to_string(),
                page: edited,
                deleted: true,
            })
            .expect("deleted json"),
        );
        txn.encode_diff_v1(&StateVector::default())
    };
    store
        .apply_update(&projected, "p-tasks", &update)
        .expect("apply delete");
    store
        .project(&mut projected, "p-tasks")
        .expect("project delete");
    assert!(
        !projected
            .data_source(data_source_id)
            .expect("data source")
            .rows
            .iter()
            .any(|candidate| candidate.id == row.id)
    );
}
