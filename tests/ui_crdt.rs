//! Client-side Yrs session tests (spec-043).

use ion_of_notes::model::{
    Block, BlockKind, PageProperty, PropertyType, PropertyValue, RichText, Workspace,
};
use ion_of_notes::server::WorkspaceServer;
use ion_of_notes::ui::CrdtSession;

#[test]
fn local_edit_produces_an_update_and_projects_back() {
    let workspace = Workspace::seed("ui-crdt");
    let page = workspace.pages[0].clone();
    let block_id = page.blocks[0].id.clone();

    let mut session = CrdtSession::new();
    session.open_page(&page);
    let update = session
        .local_text(&block_id, "Hello from the editor")
        .expect("local edit")
        .expect("changed");
    assert!(!update.is_empty());

    let mut merged = page.clone();
    session.project_page(&mut merged).expect("project");
    let block = merged
        .blocks
        .iter()
        .find(|block| block.id == block_id)
        .expect("block");
    assert_eq!(block.text, "Hello from the editor");
}

#[test]
fn desktop_structure_and_property_changes_plan_a_crdt_update() {
    let workspace = Workspace::seed("ui-crdt-objects");
    let page = workspace.page("p-home").expect("page").clone();
    let mut server = WorkspaceServer::new(workspace.clone());
    let (seed, _) = server.crdt_sync("p-home", None).expect("server sync");
    let mut session = CrdtSession::new();
    session.open_empty("p-home");
    session.apply_update(&seed).expect("seed");

    let mut changed = page.clone();
    changed.blocks.insert(
        0,
        Block::new("desktop-block", BlockKind::Paragraph, "From the desktop"),
    );
    changed.properties.insert(
        "priority".to_string(),
        PageProperty::new(
            "priority",
            PropertyType::RichText,
            PropertyValue::RichText(vec![RichText::text("High")]),
        ),
    );
    let mut changed_workspace = workspace.clone();
    *changed_workspace.page_mut("p-home").expect("page") = changed.clone();

    let update = session
        .plan_page_changes(&page, &changed, &workspace, &changed_workspace)
        .expect("plan")
        .expect("update");
    server
        .apply_crdt_update("p-home", &update, "m-owner")
        .expect("apply");
    let projected = server.workspace.page("p-home").expect("page");
    assert_eq!(
        projected.blocks.first().map(|block| block.id.as_str()),
        Some("desktop-block")
    );
    assert_eq!(
        projected.properties.get("priority"),
        changed.properties.get("priority")
    );
}

#[test]
fn desktop_row_changes_plan_a_crdt_update() {
    let mut workspace = Workspace::seed("ui-crdt-rows");
    workspace.ensure_data_sources();
    let page = workspace.page("p-tasks").expect("page").clone();
    let data_source_id = "ds-p-tasks";
    let row = workspace
        .data_source(data_source_id)
        .expect("data source")
        .rows[0]
        .clone();
    let mut session = CrdtSession::new();
    let mut server = WorkspaceServer::new(workspace.clone());
    let (seed, _) = server.crdt_sync("p-tasks", None).expect("server sync");
    session.open_empty("p-tasks");
    session.apply_update(&seed).expect("seed");

    let mut changed_workspace = workspace.clone();
    let changed_row = changed_workspace
        .data_source_mut(data_source_id)
        .expect("data source")
        .rows
        .iter_mut()
        .find(|candidate| candidate.id == row.id)
        .expect("row");
    changed_row.properties.insert(
        "assignee".to_string(),
        PageProperty::new(
            "assignee",
            PropertyType::RichText,
            PropertyValue::RichText(vec![RichText::text("Morgan")]),
        ),
    );
    let changed_page = changed_workspace.page("p-tasks").expect("page").clone();

    let update = session
        .plan_page_changes(&page, &changed_page, &workspace, &changed_workspace)
        .expect("plan")
        .expect("update");
    server
        .apply_crdt_update("p-tasks", &update, "m-owner")
        .expect("apply");
    let projected = server
        .workspace
        .data_source(data_source_id)
        .expect("data source")
        .rows
        .iter()
        .find(|candidate| candidate.id == row.id)
        .expect("row");
    assert_eq!(
        projected.properties.get("assignee"),
        changed_workspace
            .data_source(data_source_id)
            .expect("data source")
            .rows
            .iter()
            .find(|candidate| candidate.id == row.id)
            .expect("row")
            .properties
            .get("assignee")
    );
}

#[test]
fn empty_session_applies_server_update_without_duplicating_text() {
    let workspace = Workspace::seed("ui-crdt");
    let page = workspace.pages[0].clone();
    let mut server = WorkspaceServer::new(workspace);
    let (update, _) = server.crdt_sync(&page.id, None).expect("server sync");

    let mut session = CrdtSession::new();
    session.open_empty(&page.id);
    session.apply_update(&update).expect("apply server update");
    session.baseline_page(&page);

    let mut projected = page.clone();
    session.project_page(&mut projected).expect("project");
    assert_eq!(projected.blocks[0].text, page.blocks[0].text);
}

#[test]
fn two_client_sessions_converge() {
    let workspace = Workspace::seed("ui-crdt");
    let page = workspace.pages[0].clone();
    let block_id = page.blocks[0].id.clone();

    let mut left = CrdtSession::new();
    let mut right = CrdtSession::new();
    left.open_page(&page);
    // Real clients start empty and sync the server document before editing;
    // seed the second session from the first so incremental updates share
    // their dependencies.
    right.open_empty(&page.id);
    let seed = left.full_update();
    right.apply_update(&seed).expect("seed right");
    right.baseline_page(&page);

    let update_left = left
        .local_text(&block_id, "left")
        .expect("left edit")
        .expect("changed");
    let update_right = right
        .local_text(&block_id, "right")
        .expect("right edit")
        .expect("changed");

    left.apply_update(&update_right).expect("left merge");
    right.apply_update(&update_left).expect("right merge");

    let mut left_page = page.clone();
    let mut right_page = page.clone();
    left.project_page(&mut left_page).expect("project");
    right.project_page(&mut right_page).expect("project");
    let left_text = left_page
        .blocks
        .iter()
        .find(|block| block.id == block_id)
        .unwrap()
        .text
        .clone();
    let right_text = right_page
        .blocks
        .iter()
        .find(|block| block.id == block_id)
        .unwrap()
        .text
        .clone();
    assert_eq!(
        left_text,
        right_text,
        "seed update {} left {:?} right {:?}",
        update_left.len(),
        left.state_vector(),
        right.state_vector()
    );
}

#[test]
fn unchanged_text_does_not_emit_an_update() {
    let workspace = Workspace::seed("ui-crdt");
    let page = workspace.pages[0].clone();
    let block_id = page.blocks[0].id.clone();
    let mut session = CrdtSession::new();
    session.open_page(&page);
    let text = page.blocks[0].plain_text();
    assert!(
        session
            .local_text(&block_id, &text)
            .expect("no-op")
            .is_none()
    );
}

#[test]
fn a_block_created_by_split_can_sync_its_text() {
    let workspace = Workspace::seed("ui-crdt");
    let mut page = workspace.pages[0].clone();
    let mut session = CrdtSession::new();
    session.open_page(&page);

    let new_block = Block::new("split-block", BlockKind::Paragraph, "");
    page.blocks.insert(1, new_block.clone());
    session.sync_new_blocks(&page);
    let update = session
        .local_text(&new_block.id, "typed after Enter")
        .expect("local edit")
        .expect("changed");
    assert!(!update.is_empty());

    session.project_page(&mut page).expect("project");
    let projected = page
        .blocks
        .iter()
        .find(|block| block.id == new_block.id)
        .expect("new block");
    assert_eq!(projected.text, "typed after Enter");
}
