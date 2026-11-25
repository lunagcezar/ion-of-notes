//! SQLite store and JSON migration tests (spec-039).

mod common;

use ion_of_notes::app;
use ion_of_notes::auth::{
    Account, AuthStore, NoAuthStore, Session, hash_password, hash_token, new_session_token,
};
use ion_of_notes::collab::CollabState;
use ion_of_notes::model::{Block, BlockInput, BlockKind, Page, RichText, Workspace, now_iso};
use ion_of_notes::server::{Method, RequestTransport, WorkspaceServer};
use ion_of_notes::store::{SnapshotStore, SqliteStore, save, save_collab};

#[test]
fn workspace_and_collab_roundtrip_through_sqlite() {
    let path = common::temp_path("sqlite", "sqlite3");
    let workspace = Workspace::seed("sqlite-tests");
    let collab = CollabState::new();

    let store = SqliteStore::open(&path).expect("open");
    store.persist_workspace(&workspace).expect("workspace");
    store.persist_collab(&collab).expect("collab");
    drop(store);

    let reopened = SqliteStore::open(&path).expect("reopen");
    assert_eq!(
        reopened.load_workspace().expect("load").expect("workspace"),
        workspace
    );
    assert_eq!(
        reopened.load_collab().expect("load").expect("collab"),
        collab
    );

    common::remove(&path);
}

#[test]
fn incremental_page_persist_updates_one_page_and_preserves_order() {
    let path = common::temp_path("sqlite-incremental", "sqlite3");
    let mut workspace = Workspace::seed("sqlite-incremental");
    let collab = CollabState::new();
    let store = SqliteStore::open(&path).expect("open");
    store.persist_workspace(&workspace).expect("workspace");
    let page_id = workspace.pages[0].id.clone();
    let order: Vec<String> = workspace.pages.iter().map(|page| page.id.clone()).collect();

    workspace.pages[0].title = "Only this page".to_string();
    store
        .persist_page(&workspace, &collab, &page_id)
        .expect("page");
    drop(store);

    let reopened = SqliteStore::open(&path).expect("reopen");
    let loaded = reopened.load_workspace().expect("load").expect("workspace");
    assert_eq!(loaded.pages[0].title, "Only this page");
    assert_eq!(
        loaded
            .pages
            .iter()
            .map(|page| page.id.clone())
            .collect::<Vec<_>>(),
        order
    );
    assert_eq!(loaded.pages.len(), workspace.pages.len());

    let mut workspace = loaded;
    workspace.remove_page(&page_id).expect("remove");
    reopened
        .persist_page(&workspace, &collab, &page_id)
        .expect("delete page");
    drop(reopened);

    let reopened = SqliteStore::open(&path).expect("reopen");
    let loaded = reopened.load_workspace().expect("load").expect("workspace");
    assert!(!loaded.pages.iter().any(|page| page.id == page_id));

    common::remove(&path);
}

#[test]
fn block_persist_updates_one_row_and_roundtrips_nested_children() {
    let path = common::temp_path("sqlite-block", "sqlite3");
    let mut workspace = Workspace::seed("sqlite-blocks");
    let collab = CollabState::new();
    let store = SqliteStore::open(&path).expect("open");
    store
        .persist_snapshots(&workspace, &collab)
        .expect("snapshot");

    workspace
        .block_mut("h2")
        .expect("block")
        .children
        .push(Block::new("h2-child", BlockKind::Paragraph, "Nested child"));
    store
        .persist_page(&workspace, &collab, "p-home")
        .expect("page with nested child");
    workspace.block_mut("h2").expect("block").text = "Changed heading".to_string();
    store
        .persist_block(&workspace, &collab, "p-home", "h2")
        .expect("block write");
    drop(store);

    let reopened = SqliteStore::open(&path).expect("reopen");
    let loaded = reopened.load_workspace().expect("load").expect("workspace");
    let page = loaded.page("p-home").expect("page");
    let block = loaded.block("h2").expect("block");
    assert_eq!(block.text, "Changed heading");
    assert_eq!(block.children.len(), 1);
    assert_eq!(block.children[0].text, "Nested child");
    assert_eq!(
        count_blocks(&page.blocks),
        count_blocks(&workspace.page("p-home").unwrap().blocks)
    );

    let connection = rusqlite::Connection::open(&path).expect("raw");
    let json: String = connection
        .query_row(
            "SELECT json FROM workspace_pages WHERE id = 'p-home'",
            [],
            |row| row.get(0),
        )
        .expect("page row");
    let stored: Page = serde_json::from_str(&json).expect("page json");
    assert!(
        stored.blocks.is_empty(),
        "page rows must not duplicate block JSON"
    );
    let rows: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM workspace_blocks WHERE page_id = 'p-home'",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(rows as usize, count_blocks(&page.blocks));
    common::remove(&path);
}

#[test]
fn page_json_with_blocks_is_migrated_into_block_rows() {
    let path = common::temp_path("sqlite-block-migration", "sqlite3");
    let workspace = Workspace::seed("sqlite-migration");
    let collab = CollabState::new();
    let store = SqliteStore::open(&path).expect("open");
    store
        .persist_snapshots(&workspace, &collab)
        .expect("snapshot");
    drop(store);

    let connection = rusqlite::Connection::open(&path).expect("raw");
    connection
        .execute("DELETE FROM workspace_blocks", [])
        .expect("clear blocks");
    let page_json = serde_json::to_string(workspace.page("p-home").unwrap()).expect("page json");
    connection
        .execute(
            "UPDATE workspace_pages SET json = ?1 WHERE id = 'p-home'",
            [page_json],
        )
        .expect("old page row");
    drop(connection);

    let reopened = SqliteStore::open(&path).expect("reopen");
    let loaded = reopened.load_workspace().expect("load").expect("workspace");
    assert_eq!(
        loaded.page("p-home").unwrap().blocks,
        workspace.page("p-home").unwrap().blocks
    );
    let connection = rusqlite::Connection::open(&path).expect("raw");
    let rows: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM workspace_blocks WHERE page_id = 'p-home'",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert!(rows > 0);
    common::remove(&path);
}

fn count_blocks(blocks: &[Block]) -> usize {
    blocks
        .iter()
        .map(|block| 1 + count_blocks(&block.children))
        .sum()
}

#[test]
fn v2_block_patch_uses_the_block_write_path() {
    let path = common::temp_path("sqlite-block-api", "sqlite3");
    let workspace = Workspace::seed("sqlite-block-api");
    let collab = CollabState::new();
    let store = SqliteStore::open(&path).expect("open");
    store
        .persist_snapshots(&workspace, &collab)
        .expect("snapshot");
    let mut server = WorkspaceServer::with_stores_and_auth(
        workspace,
        collab,
        store.clone(),
        store.clone(),
        NoAuthStore,
    );
    server.use_snapshot_store(store.clone());
    let input = BlockInput {
        rich_text: vec![RichText::text("Changed through v2")],
        ..BlockInput::default()
    };
    let response = server.handle(&common::v2_request(
        Method::Patch,
        "/api/v2/blocks/h2",
        Some(serde_json::to_string(&input).expect("body")),
    ));
    assert_eq!(response.status, 200);
    drop(server);
    drop(store);

    let reopened = SqliteStore::open(&path).expect("reopen");
    let loaded = reopened.load_workspace().expect("load").expect("workspace");
    let block = loaded.block("h2").expect("block");
    assert_eq!(block.rich_text[0].plain_text(), "Changed through v2");
    common::remove(&path);
}

#[test]
fn accounts_and_sessions_roundtrip_and_prune() {
    let path = common::temp_path("sqlite-auth", "sqlite3");
    let store = SqliteStore::open(&path).expect("open");
    let account = Account {
        id: "account-owner".to_string(),
        member_id: "m-owner".to_string(),
        email: "owner@workspace.local".to_string(),
        password_hash: hash_password("secret").expect("hash"),
        created_at: now_iso(),
    };
    store.upsert_account(&account).expect("account");
    assert_eq!(
        store
            .find_account("OWNER@workspace.local")
            .expect("find")
            .expect("account"),
        account
    );

    let token = new_session_token();
    let session = Session {
        token_hash: hash_token(&token),
        member_id: "m-owner".to_string(),
        created_at: now_iso(),
        expires_at: (chrono::Utc::now() + chrono::Duration::days(1)).to_rfc3339(),
    };
    store.save_session(&session).expect("session");
    assert_eq!(
        store
            .find_session(&session.token_hash)
            .expect("find")
            .expect("session"),
        session
    );

    store
        .prune_sessions(&(chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339())
        .expect("prune");
    assert!(
        store
            .find_session(&session.token_hash)
            .expect("find")
            .is_none()
    );

    common::remove(&path);
}

#[test]
fn empty_sqlite_database_migrates_json_documents() {
    let db = common::temp_path("migration", "sqlite3");
    let workspace_path = common::temp_path("migration-workspace", "json");
    let collab_path = common::temp_path("migration-collab", "json");
    // A reused process id can collide with an artifact from an earlier run;
    // start from a clean database so the migration assertion is deterministic.
    common::remove(&db);
    common::remove(&workspace_path);
    common::remove(&collab_path);
    let workspace = Workspace::seed("migrated");
    let collab = CollabState::new();
    save(&workspace_path, &workspace).expect("save workspace");
    save_collab(&collab_path, &collab).expect("save collab");

    let server = app::build_server_sqlite_at_with_password(
        db.clone(),
        workspace_path.clone(),
        collab_path.clone(),
        Some("test-password"),
    )
    .expect("build");
    drop(server);

    let store = SqliteStore::open(&db).expect("open");
    assert_eq!(
        store.load_workspace().expect("load").expect("workspace"),
        workspace
    );
    assert_eq!(store.load_collab().expect("load").expect("collab"), collab);

    common::remove(&db);
    common::remove(&workspace_path);
    common::remove(&collab_path);
}
