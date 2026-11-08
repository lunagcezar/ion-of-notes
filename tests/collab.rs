//! Integration tests for the collaboration domain and the decided API
//! (public API only, following the Red -> Green -> Refactor discipline).

use not_notion::client::Client;
use not_notion::collab::{
    Access, ChangeOp, CollabState, MAX_VERSIONS_PER_PAGE, MemberDraft, Role, Share, export_tree,
    page_markdown, people_csv, people_markdown, realtime_contract,
};
use not_notion::model::{Block, BlockKind, Page, PageDraft, TableRow, Workspace};
use not_notion::server::WorkspaceServer;

#[test]
fn roles_and_access_have_stable_labels() {
    assert_eq!(Role::Owner.label(), "Workspace owner");
    assert_eq!(Role::Guest.label(), "Guest");
    assert_eq!(Access::Full.label(), "Full access");
    assert_eq!(Access::View.label(), "Can view");
    assert!(Access::Full.can_write());
    assert!(Access::Edit.can_write());
    assert!(!Access::View.can_write());
    assert!(!Access::None.can_write());
}

#[test]
fn seeded_state_has_one_owner_and_general_full_access() {
    let state = CollabState::new();
    assert_eq!(state.members.len(), 3);
    assert_eq!(state.owner_id(), "m-owner");
    assert_eq!(state.members[0].role, Role::Owner);
    // The general "everyone" share grants full access by default.
    assert!(state.can_write("any-page", "anyone"));
}

#[test]
fn invite_adds_a_member_and_access_resolves_most_specific_first() {
    let mut state = CollabState::new();
    let member = state.invite(MemberDraft {
        name: "New Person".to_string(),
        email: "new@workspace.local".to_string(),
        role: Role::Guest,
    });
    assert_eq!(member.role, Role::Guest);
    assert_eq!(state.members.len(), 4);

    state.set_access(Share {
        page_id: "p1".to_string(),
        subject: member.id.clone(),
        access: Access::View,
    });
    assert_eq!(state.access_for("p1", &member.id), Access::View);
    assert!(!state.can_write("p1", &member.id));
    // An unrelated subject still inherits the workspace-general rule.
    assert!(state.can_write("p1", "stranger"));

    // Upserting the same (page, subject) grant replaces it, never duplicates.
    state.set_access(Share {
        page_id: "p1".to_string(),
        subject: member.id.clone(),
        access: Access::Full,
    });
    assert_eq!(
        state
            .shares_for("p1")
            .iter()
            .filter(|s| s.subject == member.id)
            .count(),
        1
    );
    assert!(state.can_write("p1", &member.id));
}

#[test]
fn sync_log_returns_only_changes_after_the_cursor() {
    let mut state = CollabState::new();
    state.record(ChangeOp::Create, "p1", "m-owner");
    state.record(ChangeOp::Update, "p1", "m-owner");
    let batch = state.changes_since(0);
    assert_eq!(batch.changes.len(), 2);
    assert_eq!(batch.cursor, 2);
    assert_eq!(batch.changes[0].op, ChangeOp::Create);
    assert_eq!(state.changes_since(1).changes.len(), 1);
    assert!(state.changes_since(batch.cursor).changes.is_empty());
}

#[test]
fn version_retention_keeps_the_newest_snapshots_per_page() {
    let mut state = CollabState::new();
    let mut page = Page::new("p-retain", "Retain", "R");
    let total = MAX_VERSIONS_PER_PAGE + 5;
    for index in 0..total {
        page.title = format!("revision {index}");
        state.record_version(&page, "m-owner");
    }

    let versions = state.versions_for("p-retain");
    assert_eq!(versions.len(), MAX_VERSIONS_PER_PAGE);
    assert_eq!(versions[0].version, total as u64);
    assert_eq!(versions.last().unwrap().version, 6);

    let other = Page::new("p-other", "Other", "O");
    state.record_version(&other, "m-owner");
    assert_eq!(state.versions_for("p-other").len(), 1);
}

#[test]
fn page_markdown_renders_block_kinds_and_table_rows() {
    let mut page = Page::new("x", "Doc", "D");
    page.blocks = vec![
        Block::new("b1", BlockKind::Heading1, "Title"),
        Block::new("b2", BlockKind::Todo, "Task"),
        Block::new("b3", BlockKind::Divider, ""),
    ];
    page.rows = vec![TableRow::new("r", "Row", "Done", "Ava", "Nov")];
    let md = page_markdown(&page);
    assert!(md.starts_with("# Doc"));
    assert!(md.contains("# Title"));
    assert!(md.contains("- [ ] Task"));
    assert!(md.contains("---"));
    assert!(md.contains("| Name | Status | Assignee | Due |"));
    assert!(md.contains("| Row | Done | Ava | Nov |"));
}

#[test]
fn people_files_record_membership_and_view_definition() {
    let members = CollabState::new().members;
    let csv = people_csv(&members);
    assert!(csv.starts_with("Name,Email,Membership Type"));
    assert!(csv.contains("Workspace owner"));
    assert!(csv.contains("alex@workspace.local"));

    let md = people_markdown(&members);
    assert!(md.starts_with("# People"));
    assert!(md.contains("People_all.csv"));
    assert!(md.contains("filters:"));
    assert!(md.contains("Name: ascending"));
    assert!(md.contains("%20"));
}

#[test]
fn export_tree_has_folder_pages_child_links_and_a_people_folder() {
    let workspace = Workspace::seed("demo");
    let members = CollabState::new().members;
    let files = export_tree(&workspace, &members);

    // The workspace root "folder" page links to every top-level page.
    let root = files
        .iter()
        .find(|f| f.path == "demo workspace.md")
        .expect("root folder page");
    assert!(root.content.starts_with("# demo"));
    assert!(root.content.contains("[Home]"));

    // A page with children links to them inside its own markdown ("folder page").
    let projects = files
        .iter()
        .find(|f| f.path == "demo/Projects p-projects.md")
        .expect("Projects folder page");
    assert!(projects.content.contains("[Tasks]"));
    assert!(
        files
            .iter()
            .any(|f| f.path == "demo/Projects/Tasks p-tasks.md")
    );

    // People folder: folder markdown, companion CSV, and one file per member.
    assert!(files.iter().any(|f| f.path == "People.md"));
    assert!(files.iter().any(|f| f.path == "People_all.csv"));
    let owner = files
        .iter()
        .find(|f| f.path.ends_with("m-owner.md"))
        .expect("owner person file");
    assert!(owner.content.contains("Membership Type: Workspace owner"));
    assert!(owner.content.contains("Person: Workspace Owner"));
}

#[test]
fn realtime_contract_describes_a_websocket_crdt_channel() {
    let contract = realtime_contract();
    assert!(contract.transport.contains("WebSocket"));
    assert!(contract.channel.contains("/ws/workspace/"));
    assert!(contract.events.contains(&"edit".to_string()));
    assert!(contract.conflict_resolution.contains("CRDT"));
}

#[test]
fn collaborative_api_roundtrip_through_the_client() {
    let mut client = Client::new(WorkspaceServer::new(Workspace::seed("collab")));

    let before = client.list_members().unwrap();
    assert!(!before.is_empty());
    let invited = client
        .invite_member(&MemberDraft {
            name: "Invited Guest".to_string(),
            email: "invited@workspace.local".to_string(),
            role: Role::Guest,
        })
        .unwrap();
    assert_eq!(invited.name, "Invited Guest");
    assert_eq!(client.list_members().unwrap().len(), before.len() + 1);

    let share = Share {
        page_id: "p-home".to_string(),
        subject: invited.id.clone(),
        access: Access::View,
    };
    client.set_share(&share).unwrap();
    let shares = client.list_shares(Some("p-home")).unwrap();
    assert!(
        shares
            .iter()
            .any(|s| s.subject == invited.id && s.access == Access::View)
    );

    // An edit is reflected in the versioned sync log, and the cursor catches up.
    let page = client
        .create_page(&PageDraft {
            title: "Synced document".to_string(),
            icon: "S".to_string(),
            parent: None,
        })
        .unwrap();
    let batch = client.sync(0).unwrap();
    assert!(!batch.changes.is_empty());
    assert!(batch.changes.iter().any(|c| c.page_id == page.id));
    assert!(client.sync(batch.cursor).unwrap().changes.is_empty());

    let files = client.export_tree().unwrap();
    assert!(files.iter().any(|f| f.path == "People.md"));
    assert!(files.iter().any(|f| f.path.ends_with("_all.csv")));

    assert!(
        client
            .realtime_contract()
            .unwrap()
            .transport
            .contains("WebSocket")
    );
}

#[test]
fn empty_member_draft_is_rejected_by_the_server() {
    use not_notion::server::{ApiRequest, Method, WorkspaceApi};
    let mut server = WorkspaceServer::new(Workspace::new("x"));
    let response = server.handle(&ApiRequest::new(
        Method::Post,
        "/api/members",
        Some("{\"name\":\"\",\"email\":\"\",\"role\":\"Guest\"}".to_string()),
    ));
    assert_eq!(response.status, 400);
}
