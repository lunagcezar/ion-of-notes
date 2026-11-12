//! Client-side Yrs session tests (spec-043).

use ion_of_notes::model::{Block, BlockKind, Workspace};
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
    session.project_page(&mut merged);
    let block = merged
        .blocks
        .iter()
        .find(|block| block.id == block_id)
        .expect("block");
    assert_eq!(block.text, "Hello from the editor");
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
    session.project_page(&mut projected);
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
    right.open_page(&page);

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
    left.project_page(&mut left_page);
    right.project_page(&mut right_page);
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
    assert_eq!(left_text, right_text);
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
    let update = session
        .local_text(&new_block.id, "typed after Enter")
        .expect("local edit")
        .expect("changed");
    assert!(!update.is_empty());

    session.project_page(&mut page);
    let projected = page
        .blocks
        .iter()
        .find(|block| block.id == new_block.id)
        .expect("new block");
    assert_eq!(projected.text, "typed after Enter");
}
