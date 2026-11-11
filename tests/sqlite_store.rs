//! SQLite store and JSON migration tests (spec-039).

mod common;

use ion_of_notes::app;
use ion_of_notes::auth::{
    Account, AuthStore, Session, hash_password, hash_token, new_session_token,
};
use ion_of_notes::collab::CollabState;
use ion_of_notes::model::{Workspace, now_iso};
use ion_of_notes::store::{SqliteStore, save, save_collab};

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

    let server =
        app::build_server_sqlite_at(db.clone(), workspace_path.clone(), collab_path.clone())
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
