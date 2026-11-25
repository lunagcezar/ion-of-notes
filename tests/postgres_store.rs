//! PostgreSQL adapter round-trip (opt-in, spec-137).
//!
//! Set `ION_OF_NOTES_POSTGRES_TEST_URL` to a disposable database. Without it the
//! test is a no-op so the default gate needs no external service.

#![cfg(feature = "postgres")]

use ion_of_notes::auth::{Account, AuthStore, Session};
use ion_of_notes::collab::CollabState;
use ion_of_notes::model::{Workspace, now_iso};
use ion_of_notes::store::{PostgresStore, SnapshotStore};

#[test]
fn postgres_roundtrip_when_configured() {
    let Ok(url) = std::env::var("ION_OF_NOTES_POSTGRES_TEST_URL") else {
        return;
    };
    let mut client = postgres::Client::connect(&url, postgres::NoTls).expect("postgres connect");
    client
        .batch_execute(
            "TRUNCATE workspace_meta, workspace_pages, workspace_blocks, collab, accounts, sessions",
        )
        .expect("truncate");
    drop(client);

    let store = PostgresStore::open(&url).expect("open");
    let workspace = Workspace::seed("postgres-tests");
    let collab = CollabState::new();
    store
        .persist_snapshots(&workspace, &collab)
        .expect("snapshot");
    assert_eq!(
        store.load_workspace().expect("load").expect("workspace"),
        workspace
    );
    assert_eq!(store.load_collab().expect("load").expect("collab"), collab);

    let account = Account {
        id: "account-postgres".to_string(),
        member_id: "m-owner".to_string(),
        email: "owner@workspace.local".to_string(),
        password_hash: "hash".to_string(),
        created_at: now_iso(),
    };
    store.upsert_account(&account).expect("account");
    assert_eq!(
        store
            .find_account("owner@workspace.local")
            .expect("find")
            .expect("account"),
        account
    );
    let session = Session {
        token_hash: "token-hash".to_string(),
        member_id: "m-owner".to_string(),
        created_at: now_iso(),
        expires_at: now_iso(),
    };
    store.save_session(&session).expect("session");
    assert_eq!(
        store
            .find_session("token-hash")
            .expect("find")
            .expect("session"),
        session
    );

    let mut client = postgres::Client::connect(&url, postgres::NoTls).expect("postgres connect");
    client
        .batch_execute(
            "TRUNCATE workspace_meta, workspace_pages, workspace_blocks, collab, accounts, sessions",
        )
        .expect("truncate");
}
