//! Tests for the pure workspace data model (public API only).

use ion_of_notes::model::{Block, BlockKind, Comment, Page, PageDiff, Workspace, WorkspaceError};

fn block(id: &str, kind: BlockKind, text: &str) -> Block {
    Block::new(id, kind, text)
}

fn page(id: &str, title: &str, parent: Option<&str>) -> Page {
    let mut p = Page::new(id, title, "P");
    p.parent = parent.map(|s| s.to_string());
    p
}

#[test]
fn seed_has_expected_pages() {
    let ws = Workspace::seed("ion-of-notes");
    assert_eq!(ws.name, "ion-of-notes");
    assert_eq!(ws.version, 0);
    let titles: Vec<&str> = ws.pages.iter().map(|p| p.title.as_str()).collect();
    for expected in ["Home", "Getting Started", "Projects", "Tasks", "Notes"] {
        assert!(
            titles.contains(&expected),
            "missing seeded page: {expected}"
        );
    }
    assert!(
        ws.pages.iter().any(|p| !p.rows.is_empty()),
        "Tasks page should have table rows"
    );
}

#[test]
fn duplicate_page_id_is_rejected() {
    let mut ws = Workspace::new("w");
    ws.add_page(page("a", "A", None)).unwrap();
    assert_eq!(
        ws.add_page(page("a", "B", None)),
        Err(WorkspaceError::DuplicateId("a".to_string()))
    );
    assert_eq!(ws.pages.len(), 1);
}

#[test]
fn removing_page_removes_descendants() {
    let mut ws = Workspace::new("w");
    ws.add_page(page("a", "A", None)).unwrap();
    ws.add_page(page("b", "B", Some("a"))).unwrap();
    ws.add_page(page("c", "C", Some("b"))).unwrap();
    ws.add_page(page("d", "D", None)).unwrap();
    let removed = ws.remove_page("a").unwrap();
    assert_eq!(removed.len(), 3);
    assert!(removed.contains(&"a".to_string()));
    assert!(removed.contains(&"c".to_string()));
    assert!(ws.page("a").is_none());
    assert!(ws.page("b").is_none());
    assert!(ws.page("c").is_none());
    assert!(ws.page("d").is_some());
}

#[test]
fn breadcrumbs_are_root_first() {
    let mut ws = Workspace::new("w");
    ws.add_page(page("a", "A", None)).unwrap();
    ws.add_page(page("b", "B", Some("a"))).unwrap();
    ws.add_page(page("c", "C", Some("b"))).unwrap();
    assert_eq!(ws.breadcrumbs("c"), vec!["A", "B", "C"]);
    assert!(ws.breadcrumbs("missing").is_empty());
}

#[test]
fn search_ranks_title_above_body_and_ignores_case() {
    let mut ws = Workspace::new("w");
    let mut notes = page("notes", "Notes", None);
    notes
        .blocks
        .push(block("b1", BlockKind::Paragraph, "The roadmap lives here"));
    ws.add_page(notes).unwrap();
    ws.add_page(page("road", "Roadmap", None)).unwrap();
    let results = ws.search("ROAD");
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].page_id, "road");
    assert!(results[0].score > results[1].score);
}

#[test]
fn plain_text_joins_block_text() {
    let mut p = page("p", "P", None);
    p.blocks.push(block("b1", BlockKind::Paragraph, "Hello"));
    p.blocks.push(block("b2", BlockKind::Divider, ""));
    p.blocks.push(block("b3", BlockKind::Todo, "world"));
    assert_eq!(p.plain_text(), "Hello world");
}

#[test]
fn workspace_json_roundtrip() {
    let ws = Workspace::seed("roundtrip");
    let json = serde_json::to_string(&ws).unwrap();
    let back: Workspace = serde_json::from_str(&json).unwrap();
    assert_eq!(ws, back);
}

#[test]
fn toggle_todo_flips_only_that_block() {
    let mut p = page("p", "P", None);
    p.blocks.push(block("t", BlockKind::Todo, "task"));
    p.blocks.push(block("x", BlockKind::Paragraph, "other"));
    let other = p.blocks[1].clone();
    assert!(!p.blocks[0].checked);
    assert!(p.blocks[0].toggle_todo());
    assert!(p.blocks[0].checked);
    assert!(!p.blocks[0].toggle_todo());
    assert_eq!(p.blocks[1], other);
}

#[test]
fn set_kind_preserves_text_and_id() {
    let mut b = block("b", BlockKind::Paragraph, "keep me");
    b.set_kind(BlockKind::Heading2);
    assert_eq!(b.kind, BlockKind::Heading2);
    assert_eq!(b.text, "keep me");
    assert_eq!(b.id, "b");
}

#[test]
fn bump_version_increments_once() {
    let mut ws = Workspace::new("w");
    assert_eq!(ws.version, 0);
    ws.bump_version();
    assert_eq!(ws.version, 1);
}

#[test]
fn children_and_favorites_are_filtered() {
    let mut ws = Workspace::new("w");
    let mut fav = page("a", "A", None);
    fav.favorite = true;
    ws.add_page(fav).unwrap();
    ws.add_page(page("b", "B", Some("a"))).unwrap();
    ws.add_page(page("c", "C", None)).unwrap();
    assert_eq!(ws.children(None).len(), 2);
    assert_eq!(ws.children(Some("a")).len(), 1);
    assert_eq!(ws.favorites().len(), 1);
    assert_eq!(ws.favorites()[0].title, "A");
}

#[test]
fn workspace_diff_detects_changed_added_removed_and_reordered_pages() {
    let mut before = Workspace::new("w");
    before.add_page(page("a", "A", None)).unwrap();
    before.add_page(page("b", "B", None)).unwrap();

    let mut changed = before.clone();
    changed.page_mut("a").unwrap().title = "A2".to_string();
    let diff = before.diff(&changed);
    assert_eq!(diff.changed, vec!["a"]);
    assert!(diff.added.is_empty());
    assert!(diff.removed.is_empty());
    assert!(!diff.reordered);
    assert!(!diff.requires_full_replace());

    let mut added = before.clone();
    added.add_page(page("c", "C", None)).unwrap();
    let diff = before.diff(&added);
    assert_eq!(diff.added, vec!["c"]);
    assert!(diff.requires_full_replace());

    let mut removed = before.clone();
    removed.remove_page("b").unwrap();
    let diff = before.diff(&removed);
    assert_eq!(diff.removed, vec!["b"]);
    assert!(diff.requires_full_replace());

    let mut reordered = before.clone();
    reordered.pages.swap(0, 1);
    let diff = before.diff(&reordered);
    assert!(diff.reordered);
    assert!(diff.requires_full_replace());

    assert_eq!(before.diff(&before), PageDiff::default());
    assert!(before.diff(&before).is_empty());
}

#[test]
fn page_action_flags_and_comments_roundtrip() {
    let mut page = page("p", "P", None);
    assert!(!page.show_toc);
    assert!(!page.discussions);
    assert!(!page.inline_comments);
    assert!(!page.archived);
    page.show_toc = true;
    page.discussions = true;
    page.inline_comments = true;
    page.archived = true;
    page.comments.push(Comment::new(
        "c1",
        "Local user",
        "A comment",
        "Nov 08, 2026 10:00",
    ));

    let json = serde_json::to_string(&page).unwrap();
    let back: Page = serde_json::from_str(&json).unwrap();
    assert_eq!(back, page);
}

#[test]
fn headings_are_filtered_for_the_table_of_contents() {
    let mut page = page("p", "P", None);
    page.blocks = vec![
        Block::new("h1", BlockKind::Heading1, "Title"),
        Block::new("p", BlockKind::Paragraph, "Body"),
        Block::new("h2", BlockKind::Heading2, ""),
    ];
    let headings = page.headings();
    assert_eq!(headings.len(), 1);
    assert_eq!(headings[0].text, "Title");
}

#[test]
fn favorites_and_search_ignore_archived_pages() {
    let mut ws = Workspace::new("w");
    let mut archived = page("a", "Archived", None);
    archived.favorite = true;
    archived.archived = true;
    archived
        .blocks
        .push(block("b", BlockKind::Paragraph, "needle"));
    ws.add_page(archived).unwrap();
    assert!(ws.favorites().is_empty());
    assert!(ws.search("needle").is_empty());
}

#[test]
fn workspace_settings_roundtrip_through_json() {
    let mut ws = Workspace::new("w");
    ws.settings.analytics = true;
    ws.settings.allowed_domains.push("example.com".to_string());
    ws.settings.landing_page = Some("p1".to_string());
    ws.settings.sidebar_library = false;
    ws.settings.onboarding_completed = true;
    ws.settings.workspace_purpose = Some("work".to_string());

    let json = serde_json::to_string(&ws).unwrap();
    let back: Workspace = serde_json::from_str(&json).unwrap();
    assert_eq!(back.settings, ws.settings);
}

#[test]
fn workspace_settings_change_requires_full_replace() {
    let mut before = Workspace::new("w");
    before.add_page(page("a", "A", None)).unwrap();

    let mut settings_changed = before.clone();
    settings_changed.settings.analytics = true;
    let diff = before.diff(&settings_changed);
    assert!(diff.settings_changed);
    assert!(diff.requires_full_replace());

    let mut name_changed = before.clone();
    name_changed.name = "renamed".to_string();
    let diff = before.diff(&name_changed);
    assert!(diff.settings_changed);
    assert!(diff.requires_full_replace());
}

#[test]
fn workspace_index_finds_pages_children_and_breadcrumbs() {
    let mut ws = Workspace::new("w");
    ws.add_page(page("a", "A", None)).unwrap();
    ws.add_page(page("b", "B", Some("a"))).unwrap();
    ws.add_page(page("c", "C", Some("b"))).unwrap();

    assert_eq!(ws.page("a").unwrap().title, "A");
    assert_eq!(ws.page("c").unwrap().title, "C");
    assert_eq!(ws.children(None).len(), 1);
    assert_eq!(ws.children(Some("a")).len(), 1);
    assert_eq!(ws.children(Some("b")).len(), 1);
    assert_eq!(ws.breadcrumbs("c"), vec!["A", "B", "C"]);

    ws.remove_page("a").unwrap();
    assert!(ws.page("a").is_none());
    assert!(ws.page("c").is_none());

    ws.add_page(page("d", "D", None)).unwrap();
    let json = serde_json::to_string(&ws).unwrap();
    let back: Workspace = serde_json::from_str(&json).unwrap();
    assert_eq!(back, ws);
    assert_eq!(back.page("d").unwrap().title, "D");
    assert!(back.page("a").is_none());
}

#[test]
fn workspace_index_falls_back_after_direct_page_mutation() {
    let mut ws = Workspace::new("w");
    ws.pages.push(page("direct", "Direct", None));
    assert_eq!(ws.page("direct").unwrap().title, "Direct");
    ws.page_mut("direct").unwrap().title = "Changed".to_string();
    assert_eq!(ws.page("direct").unwrap().title, "Changed");
}

#[test]
fn workspace_index_survives_a_page_reorder() {
    let mut ws = Workspace::new("w");
    ws.add_page(page("a", "A", None)).unwrap();
    ws.add_page(page("b", "B", None)).unwrap();
    ws.pages.swap(0, 1);
    assert_eq!(ws.page("a").unwrap().title, "A");
    assert_eq!(ws.page("b").unwrap().title, "B");
}

#[test]
fn block_kind_labels_are_unique() {
    let labels: Vec<&str> = BlockKind::all().iter().map(|k| k.label()).collect();
    let unique: std::collections::HashSet<&str> = labels.iter().copied().collect();
    assert_eq!(labels.len(), unique.len());
    assert_eq!(BlockKind::Divider.label(), "Divider");
}
