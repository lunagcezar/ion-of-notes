//! Tests for the starter template catalog (spec-027).

use not_notion::model::BlockKind;
use not_notion::templates::{catalog, find, template_for_purpose};
use std::collections::HashSet;

#[test]
fn template_ids_are_unique_and_non_empty() {
    let ids: HashSet<&str> = catalog().iter().map(|template| template.id).collect();
    assert_eq!(ids.len(), catalog().len());
    assert!(catalog().iter().all(|template| !template.name.is_empty()));
    assert!(catalog().iter().all(|template| !template.title.is_empty()));
}

#[test]
fn template_builds_a_page_with_blocks_and_rows() {
    let template = find("work-project").expect("work project template");
    let page = template.to_page("page-1");
    assert_eq!(page.id, "page-1");
    assert_eq!(page.title, "Project tracker");
    assert_eq!(
        page.blocks.first().map(|block| block.kind),
        Some(BlockKind::Heading1)
    );
    assert_eq!(page.rows.len(), 2);
    assert!(
        page.blocks
            .iter()
            .all(|block| block.id.starts_with("page-1-b"))
    );
    assert!(page.rows.iter().all(|row| row.id.starts_with("page-1-r")));
}

#[test]
fn onboarding_purposes_map_to_templates() {
    assert_eq!(
        template_for_purpose("work").map(|t| t.id),
        Some("work-project")
    );
    assert_eq!(
        template_for_purpose("personal").map(|t| t.id),
        Some("personal-notes")
    );
    assert_eq!(
        template_for_purpose("school").map(|t| t.id),
        Some("school-course")
    );
    assert!(template_for_purpose("unknown").is_none());
    assert!(find("missing").is_none());
}
