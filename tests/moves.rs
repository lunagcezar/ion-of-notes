//! Resource-move and upload authorization regressions (spec-138).

mod common;

use common::{data, v2_request, v2_request_as};
use ion_of_notes::auth::NoAuthStore;
use ion_of_notes::collab::{Access, CollabState, Member, Role, Share};
use ion_of_notes::model::{
    FileUpload, FileUploadInput, ParentDto, UploadStatus, View, ViewType, Workspace,
};
use ion_of_notes::server::{Capability, Connection, Method, RequestTransport, WorkspaceServer};
use ion_of_notes::store::{NoCollabStore, NoStore};

fn member_server() -> WorkspaceServer {
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
    let mut server = WorkspaceServer::with_stores_and_auth(
        workspace,
        collab,
        NoStore,
        NoCollabStore,
        NoAuthStore,
    );
    server.register_connection(Connection::new(
        "c-member",
        "member-token",
        "m-member",
        "authz",
        Capability::all(),
    ));
    server
}

fn create_database(server: &mut WorkspaceServer, page_id: &str) -> String {
    let body = serde_json::json!({
        "title": "Shared table",
        "parent_page_id": page_id,
        "is_inline": true,
    })
    .to_string();
    let response = server.handle(&v2_request(Method::Post, "/api/v2/databases", Some(body)));
    assert_eq!(response.status, 201, "{}", response.body);
    let database: ion_of_notes::model::Database = data(&response);
    database.id
}

#[test]
fn database_cannot_move_to_a_page_without_write_access() {
    let mut server = member_server();
    let database_id = create_database(&mut server, "p-home");

    let body = serde_json::json!({ "parent_page_id": "p-start" }).to_string();
    let response = server.handle(&v2_request_as(
        "member-token",
        Method::Patch,
        &format!("/api/v2/databases/{database_id}"),
        Some(body),
    ));
    assert_eq!(response.status, 403, "{}", response.body);
    assert_eq!(
        server
            .workspace
            .database(&database_id)
            .map(|db| db.parent.clone()),
        Some(ParentDto::Page {
            page_id: "p-home".to_string()
        })
    );

    let body = serde_json::json!({ "parent_page_id": "p-home" }).to_string();
    let response = server.handle(&v2_request_as(
        "member-token",
        Method::Patch,
        &format!("/api/v2/databases/{database_id}"),
        Some(body),
    ));
    assert_eq!(response.status, 200, "{}", response.body);
}

#[test]
fn upload_can_only_be_sent_by_its_creator() {
    let mut server = member_server();
    server.use_uploads_dir(common::temp_dir("uploads-authz"));
    server.register_connection(Connection::new(
        "c-other",
        "other-token",
        "m-other",
        "authz",
        Capability::all(),
    ));
    server.collab.members.push(Member::new(
        "m-other",
        "Other Member",
        "other@workspace.local",
        Role::Member,
    ));
    server.collab.shares.push(Share {
        page_id: "p-home".to_string(),
        subject: "m-other".to_string(),
        access: Access::Edit,
    });

    let body = serde_json::to_string(&FileUploadInput {
        filename: Some("note.txt".to_string()),
        content_type: Some("text/plain".to_string()),
        content_length: None,
        page_id: Some("p-home".to_string()),
    })
    .unwrap();
    let response = server.handle(&v2_request_as(
        "member-token",
        Method::Post,
        "/api/v2/file_uploads",
        Some(body),
    ));
    assert_eq!(response.status, 201, "{}", response.body);
    let upload: FileUpload = data(&response);
    assert_eq!(upload.created_by.as_deref(), Some("m-member"));
    assert_eq!(upload.page_id.as_deref(), Some("p-home"));

    let response = server.handle(&v2_request_as(
        "other-token",
        Method::Post,
        &format!("/api/v2/file_uploads/{}/send", upload.id),
        Some("stolen".to_string()),
    ));
    assert_eq!(response.status, 403, "{}", response.body);
    assert!(server.upload_content_len(&upload.id).is_none());

    let response = server.handle(&v2_request_as(
        "member-token",
        Method::Post,
        &format!("/api/v2/file_uploads/{}/send", upload.id),
        Some("payload".to_string()),
    ));
    assert_eq!(response.status, 200, "{}", response.body);
    let stored: FileUpload = data(&response);
    assert_eq!(stored.status, UploadStatus::Uploaded);
    assert_eq!(server.upload_content_len(&upload.id), Some(7));
}

#[test]
fn view_cannot_move_to_a_data_source_on_an_unreadable_page() {
    let mut server = member_server();
    server.workspace.views.push(View {
        id: "view-home".to_string(),
        parent: ParentDto::Page {
            page_id: "p-home".to_string(),
        },
        data_source_id: None,
        name: "Home table".to_string(),
        r#type: ViewType::Table,
        filter: None,
        sorts: None,
        configuration: None,
        created_time: String::new(),
        created_by: None,
        last_edited_time: String::new(),
        last_edited_by: None,
        url: String::new(),
    });

    let body = serde_json::json!({ "data_source_id": "ds-p-tasks" }).to_string();
    let response = server.handle(&v2_request_as(
        "member-token",
        Method::Patch,
        "/api/v2/views/view-home",
        Some(body),
    ));
    assert_eq!(response.status, 403, "{}", response.body);
    assert_eq!(
        server
            .workspace
            .view("view-home")
            .and_then(|view| view.data_source_id.clone()),
        None,
        "the view must not point at an unreadable data source"
    );
}

#[test]
fn deleted_page_changes_stay_visible_to_a_shared_member() {
    let mut server = member_server();
    let connection = server.connection_for_token("member-token").expect("member");
    assert!(server.change_visible(&connection, "p-home"));
    assert!(!server.change_visible(&connection, "p-start"));
    assert!(
        !server.change_visible(&connection, "__workspace__"),
        "workspace-level markers stay owner-only"
    );

    let response = server.handle(&v2_request_as(
        "member-token",
        Method::Delete,
        "/api/v2/pages/p-home",
        None,
    ));
    assert_eq!(response.status, 200, "{}", response.body);
    assert!(server.workspace.page("p-home").is_none());
    assert!(
        server.change_visible(&connection, "p-home"),
        "share rules must outlive the page for delete events"
    );
}
