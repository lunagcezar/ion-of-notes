//! Principal-aware authorization tests (spec-076).

mod common;

use ion_of_notes::auth::{Account, AuthStore, MemoryAuthStore, SessionResponse, hash_password};
use ion_of_notes::collab::{Access, CollabState, Member, Role, Share};
use ion_of_notes::model::{Workspace, now_iso};
use ion_of_notes::server::{
    API_VERSION, ApiRequest, Envelope, Method, RequestTransport, WorkspaceServer,
};
use ion_of_notes::store::{NoCollabStore, NoStore};

fn account(member: &Member) -> Account {
    Account {
        id: format!("account-{}", member.id),
        member_id: member.id.clone(),
        email: member.email.to_lowercase(),
        password_hash: hash_password("secret").expect("hash"),
        created_at: now_iso(),
    }
}

fn server() -> WorkspaceServer {
    let workspace = Workspace::seed("authz");
    let mut collab = CollabState::new();
    collab.shares.clear();
    collab.members.push(Member::new(
        "m-member",
        "Morgan Member",
        "member@workspace.local",
        Role::Member,
    ));
    collab.shares.push(Share {
        page_id: "p-home".to_string(),
        subject: "m-member".to_string(),
        access: Access::Edit,
    });
    collab.shares.push(Share {
        page_id: "p-home".to_string(),
        subject: "m-guest-1".to_string(),
        access: Access::View,
    });
    let store = MemoryAuthStore::default();
    for member in &collab.members {
        store.upsert_account(&account(member)).expect("account");
    }
    WorkspaceServer::with_stores_and_auth(workspace, collab, NoStore, NoCollabStore, store)
}

fn login(server: &mut WorkspaceServer, email: &str) -> String {
    let body = serde_json::json!({ "email": email, "password": "secret" }).to_string();
    let request = ApiRequest::new(Method::Post, "/api/v2/auth/login", Some(body))
        .with_header("ion-of-notes-version", API_VERSION);
    let response = server.handle(&request);
    assert_eq!(response.status, 200);
    let envelope: Envelope<SessionResponse> =
        serde_json::from_str(&response.body).expect("envelope");
    envelope.data.expect("session").token
}

fn v2(token: &str, method: Method, path: &str, body: Option<String>) -> ApiRequest {
    let request =
        ApiRequest::new(method, path, body).with_header("ion-of-notes-version", API_VERSION);
    request.with_header("authorization", format!("Bearer {token}"))
}

#[test]
fn members_and_guests_are_limited_by_page_shares() {
    let mut server = server();
    let home = server.workspace.page("p-home").cloned().expect("home page");
    let home_json = serde_json::to_string(&home).expect("json");

    let member = login(&mut server, "member@workspace.local");
    let read = server.handle(&v2(&member, Method::Get, "/api/v2/pages/p-home", None));
    assert_eq!(read.status, 200);
    let write = server.handle(&v2(
        &member,
        Method::Patch,
        "/api/v2/pages/p-home",
        Some(home_json.clone()),
    ));
    assert_eq!(write.status, 200);
    let unshared = server.handle(&v2(&member, Method::Get, "/api/v2/pages/p-start", None));
    assert_eq!(unshared.status, 403);

    let guest = login(&mut server, "alex@workspace.local");
    let guest_read = server.handle(&v2(&guest, Method::Get, "/api/v2/pages/p-home", None));
    assert_eq!(guest_read.status, 200);
    let guest_write = server.handle(&v2(
        &guest,
        Method::Patch,
        "/api/v2/pages/p-home",
        Some(home_json),
    ));
    assert_eq!(guest_write.status, 403);
    let guest_unshared = server.handle(&v2(&guest, Method::Get, "/api/v2/pages/p-start", None));
    assert_eq!(guest_unshared.status, 403);
}

#[test]
fn only_the_owner_can_replace_the_workspace() {
    let mut server = server();
    let workspace_json = serde_json::to_string(&server.workspace).expect("json");

    let member = login(&mut server, "member@workspace.local");
    let denied = server.handle(&v2(
        &member,
        Method::Put,
        "/api/v2/workspace",
        Some(workspace_json.clone()),
    ));
    assert_eq!(denied.status, 403);

    let owner = login(&mut server, "owner@workspace.local");
    let allowed = server.handle(&v2(
        &owner,
        Method::Put,
        "/api/v2/workspace",
        Some(workspace_json),
    ));
    assert_eq!(allowed.status, 200);
}

#[test]
fn v1_page_routes_use_the_same_share_checks() {
    let mut server = server();
    let member = login(&mut server, "member@workspace.local");
    let allowed = ApiRequest::new(Method::Get, "/api/pages/p-home", None)
        .with_header("authorization", format!("Bearer {member}"));
    assert_eq!(server.handle(&allowed).status, 200);
    let denied = ApiRequest::new(Method::Get, "/api/pages/p-start", None)
        .with_header("authorization", format!("Bearer {member}"));
    assert_eq!(server.handle(&denied).status, 403);
}

#[test]
fn nested_block_routes_resolve_the_owning_page() {
    let mut server = server();
    let member = login(&mut server, "member@workspace.local");

    let readable = server.handle(&v2(&member, Method::Get, "/api/v2/blocks/h1", None));
    assert_eq!(readable.status, 200);

    let unreadable = server.handle(&v2(&member, Method::Get, "/api/v2/blocks/s1", None));
    assert_eq!(unreadable.status, 403);

    let body = serde_json::json!({ "text": "changed" }).to_string();
    let writable = server.handle(&v2(
        &member,
        Method::Patch,
        "/api/v2/blocks/h1",
        Some(body.clone()),
    ));
    assert_eq!(writable.status, 200);

    let forbidden = server.handle(&v2(&member, Method::Patch, "/api/v2/blocks/s1", Some(body)));
    assert_eq!(forbidden.status, 403);
    let deleted = server.handle(&v2(&member, Method::Delete, "/api/v2/blocks/s1", None));
    assert_eq!(deleted.status, 403);
}

#[test]
fn nested_comment_routes_resolve_the_owning_page() {
    let mut server = server();
    let member = login(&mut server, "member@workspace.local");

    let filtered = server.handle(&v2(
        &member,
        Method::Get,
        "/api/v2/comments?page=p-start",
        None,
    ));
    assert_eq!(filtered.status, 403);

    let body = serde_json::to_string(&ion_of_notes::model::CommentInput {
        parent: ion_of_notes::model::ParentDto::Page {
            page_id: "p-start".to_string(),
        },
        discussion_id: None,
        rich_text: vec![ion_of_notes::model::RichText::text("hello")],
        attachments: Vec::new(),
        display_name: None,
        property_id: None,
        anchor: None,
    })
    .expect("body");
    let denied = server.handle(&v2(&member, Method::Post, "/api/v2/comments", Some(body)));
    assert_eq!(denied.status, 403);

    let allowed_body = serde_json::to_string(&ion_of_notes::model::CommentInput {
        parent: ion_of_notes::model::ParentDto::Page {
            page_id: "p-home".to_string(),
        },
        discussion_id: None,
        rich_text: vec![ion_of_notes::model::RichText::text("hello")],
        attachments: Vec::new(),
        display_name: None,
        property_id: None,
        anchor: None,
    })
    .expect("body");
    let allowed = server.handle(&v2(
        &member,
        Method::Post,
        "/api/v2/comments",
        Some(allowed_body),
    ));
    assert_eq!(allowed.status, 201);
}

#[test]
fn list_search_and_workspace_reads_are_projected() {
    let mut server = server();
    let member = login(&mut server, "member@workspace.local");

    let pages = server.handle(&v2(&member, Method::Get, "/api/v2/pages", None));
    assert_eq!(pages.status, 200);
    let envelope: Envelope<ion_of_notes::ListResponse<ion_of_notes::model::PageSummary>> =
        serde_json::from_str(&pages.body).expect("pages");
    let ids: Vec<String> = envelope
        .data
        .expect("data")
        .results
        .into_iter()
        .map(|page| page.id)
        .collect();
    assert_eq!(ids, vec!["p-home".to_string()]);

    let search = server.handle(&v2(&member, Method::Get, "/api/v2/search?q=Getting", None));
    assert_eq!(search.status, 200);
    let envelope: Envelope<ion_of_notes::ListResponse<ion_of_notes::model::SearchResult>> =
        serde_json::from_str(&search.body).expect("search");
    assert!(envelope.data.expect("data").results.is_empty());

    let workspace = server.handle(&v2(&member, Method::Get, "/api/v2/workspace", None));
    assert_eq!(workspace.status, 200);
    let envelope: Envelope<Workspace> = serde_json::from_str(&workspace.body).expect("workspace");
    let workspace = envelope.data.expect("data");
    assert_eq!(workspace.pages.len(), 1);
    assert_eq!(workspace.pages[0].id, "p-home");
    assert!(workspace.databases.is_empty());
    assert!(workspace.comments.is_empty());
}

#[test]
fn database_view_and_export_routes_require_page_access() {
    let mut server = server();
    server.workspace.ensure_data_sources();
    server.workspace.ensure_views();
    let member = login(&mut server, "member@workspace.local");

    for path in [
        "/api/v2/databases/db-p-tasks",
        "/api/v2/data_sources/ds-p-tasks",
        "/api/v2/views/view-ds-p-tasks",
    ] {
        let response = server.handle(&v2(&member, Method::Get, path, None));
        assert_eq!(response.status, 403, "{path}");
    }

    let databases = server.handle(&v2(&member, Method::Get, "/api/v2/databases", None));
    assert_eq!(databases.status, 200);
    let envelope: Envelope<Vec<ion_of_notes::model::Database>> =
        serde_json::from_str(&databases.body).expect("databases");
    assert!(envelope.data.expect("data").is_empty());

    let exported = server.handle(&v2(&member, Method::Get, "/api/v2/export", None));
    assert_eq!(exported.status, 403);
}

#[test]
fn crdt_and_webhook_routes_are_scoped_to_the_connection() {
    let mut server = server();
    let member = login(&mut server, "member@workspace.local");

    let sync_body = serde_json::json!({ "page_id": "p-start" }).to_string();
    let denied = server.handle(&v2(
        &member,
        Method::Post,
        "/api/v2/crdt/sync",
        Some(sync_body),
    ));
    assert_eq!(denied.status, 403);

    let allowed_body = serde_json::json!({ "page_id": "p-home" }).to_string();
    let allowed = server.handle(&v2(
        &member,
        Method::Post,
        "/api/v2/crdt/sync",
        Some(allowed_body),
    ));
    assert_eq!(allowed.status, 200);

    let webhooks = server.handle(&v2(&member, Method::Get, "/api/v2/webhooks", None));
    assert_eq!(webhooks.status, 403);
}
