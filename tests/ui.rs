//! Tests for the UI's public visual helpers (public API only).

use ion_of_notes::client::Client;
use ion_of_notes::model::{BlockKind, Page, PageDiff, Workspace};
use ion_of_notes::server::{ApiRequest, ApiResponse, Envelope, Method, WorkspaceApi};
use ion_of_notes::ui::{
    CommitPlan, Palette, PendingEdits, SearchCache, block_menu_options, execute_plan,
    is_valid_email, member_name_from_email, pages_for_view, parse_invite_emails, plan_for,
    rows_for, section_matches, settings_matches, status_color, visible_rows,
};
use ion_of_notes::ui::{EMOJI, ICONS, catalog_matches};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

#[test]
fn palette_colors_are_distinct_per_theme() {
    let light = Palette::light();
    let dark = Palette::dark();
    assert_ne!(light.sidebar, dark.sidebar);
    assert_ne!(light.content, dark.content);
    assert_ne!(light.text, dark.text);
}

#[test]
fn block_menu_options_match_block_kinds() {
    let options = block_menu_options();
    assert_eq!(options.len(), BlockKind::all().len() - 1);
    let labels: std::collections::HashSet<&str> = options.iter().map(|(_, l)| *l).collect();
    assert_eq!(labels.len(), options.len());
    assert!(options.iter().all(|(_, l)| !l.is_empty()));
}

#[test]
fn emoji_and_icon_catalogs_search_by_name_and_keywords() {
    assert!(
        EMOJI.len() >= 400,
        "the emoji catalog should be substantial"
    );
    assert!(ICONS.iter().any(|icon| icon.name == "star"));
    assert!(ICONS.iter().any(|icon| icon.name == "lock"));
    assert!(catalog_matches("grinning face", "smile happy", "smil"));
    assert!(catalog_matches("rocket", "space launch", "launch"));
    assert!(catalog_matches("star", "", "STAR"));
    assert!(!catalog_matches("rocket", "space launch", "banana"));
    assert!(catalog_matches("rocket", "space launch", "  "));
}

#[test]
fn status_chip_color_is_stable_for_known_statuses() {
    assert_eq!(status_color("Done"), status_color("Done"));
    assert_ne!(status_color("Done"), status_color("In progress"));
    assert_ne!(status_color("Unknown"), status_color("Not started"));
}

#[test]
fn settings_search_matches_titles_and_descriptions_case_insensitively() {
    assert!(settings_matches("", "Anything", "Anything"));
    assert!(settings_matches("  ", "Anything", "Anything"));
    assert!(settings_matches("LANG", "Language", "Choose a language"));
    assert!(settings_matches(
        "appearance",
        "Preferences",
        "Appearance and input"
    ));
    assert!(!settings_matches(
        "billing",
        "People",
        "Manage who can access"
    ));
}

#[test]
fn settings_rows_filter_inside_the_open_section() {
    let all = rows_for("Preferences");
    assert!(!all.is_empty());
    assert_eq!(visible_rows("Preferences", "").len(), all.len());
    assert_eq!(visible_rows("Preferences", "   ").len(), all.len());

    let language = visible_rows("Preferences", "language");
    assert_eq!(language.len(), 1);
    assert_eq!(language[0].id, "language");

    let export = visible_rows("Sidebar", "export");
    assert!(export.iter().any(|row| row.id == "export"));
    assert!(export.iter().all(|row| row.section == "Export"));

    assert!(visible_rows("Preferences", "no-such-setting").is_empty());
    assert!(visible_rows("People", "zzz").is_empty());

    // A row match keeps its section visible even when the section description
    // does not contain the query.
    assert!(section_matches(
        "high contrast",
        "Preferences",
        "Appearance, input options, language and time"
    ));
    assert!(!section_matches(
        "no-such-setting",
        "Preferences",
        "Appearance, input options, language and time"
    ));
}

#[test]
fn every_settings_row_is_unique_and_searchable() {
    for page in [
        "Preferences",
        "General",
        "Sidebar",
        "Offline",
        "People",
        "AI assistant",
    ] {
        let rows = rows_for(page);
        assert!(!rows.is_empty(), "{page} should have rows");
        let mut ids = HashSet::new();
        for row in rows {
            assert!(ids.insert(row.id), "{page} has duplicate id {}", row.id);
            assert!(!row.title.trim().is_empty());
            assert!(!row.description.trim().is_empty());
            assert!(!row.section.trim().is_empty());
            assert!(
                visible_rows(page, row.title)
                    .iter()
                    .any(|found| found.id == row.id),
                "{page}:{} does not match its own title",
                row.id
            );
        }
    }
}

#[test]
fn commit_plan_uses_patch_for_changed_pages_and_replace_for_structure() {
    assert_eq!(plan_for(&PageDiff::default()), CommitPlan::None);

    let changed = PageDiff {
        changed: vec!["a".to_string()],
        ..PageDiff::default()
    };
    assert_eq!(
        plan_for(&changed),
        CommitPlan::Update(vec!["a".to_string()])
    );

    let added = PageDiff {
        added: vec!["b".to_string()],
        ..PageDiff::default()
    };
    assert_eq!(plan_for(&added), CommitPlan::Replace);

    let reordered = PageDiff {
        reordered: true,
        ..PageDiff::default()
    };
    assert_eq!(plan_for(&reordered), CommitPlan::Replace);
}

#[test]
fn library_views_filter_pages() {
    let mut favorite = Page::new("fav", "Favorite", "F");
    favorite.favorite = true;
    let shared_page = Page::new("shared", "Shared", "S");
    let private = Page::new("private", "Private", "P");
    let mut archived = Page::new("archived", "Archived", "A");
    archived.archived = true;
    let pages = vec![favorite, shared_page, private, archived];
    let shared = HashSet::from(["shared".to_string()]);

    let favorites = pages_for_view(&pages, "Favorites", &shared);
    assert_eq!(favorites.len(), 1);
    assert_eq!(favorites[0].id, "fav");

    let shared_pages = pages_for_view(&pages, "Shared", &shared);
    assert_eq!(shared_pages.len(), 1);
    assert_eq!(shared_pages[0].id, "shared");

    assert_eq!(pages_for_view(&pages, "Private", &shared).len(), 2);
    assert_eq!(pages_for_view(&pages, "Recents", &shared).len(), 3);
    assert_eq!(pages_for_view(&pages, "Archive", &shared).len(), 1);
    assert_eq!(pages_for_view(&pages, "Archive", &shared)[0].id, "archived");
    assert!(pages_for_view(&pages, "Agents", &shared).is_empty());
}

#[test]
fn invite_helpers_parse_validate_and_name_emails() {
    assert_eq!(
        parse_invite_emails("a@b.com, c@d.org, , "),
        vec!["a@b.com", "c@d.org"]
    );
    assert!(is_valid_email("a@b.com"));
    assert!(!is_valid_email("nope"));
    assert!(!is_valid_email("a@b"));
    assert_eq!(member_name_from_email("jane.doe@example.com"), "Jane Doe");
}

#[test]
fn pending_edits_debounces_and_forces_flush() {
    let mut pending = PendingEdits::new(0.25);
    assert!(!pending.should_flush(0.0));

    pending.mark(1.0);
    assert!(!pending.should_flush(1.1));
    assert!(pending.should_flush(1.25));

    pending.force();
    assert!(pending.should_flush(1.26));
    pending.flushed();
    assert!(!pending.should_flush(2.0));
}

#[derive(Clone, Default)]
struct RecordingApi {
    requests: Arc<Mutex<Vec<ApiRequest>>>,
}

impl WorkspaceApi for RecordingApi {
    fn handle(&mut self, request: &ApiRequest) -> ApiResponse {
        self.requests.lock().unwrap().push(request.clone());
        let body = match request.method {
            Method::Patch => {
                let page: Page = serde_json::from_str(request.body.as_deref().unwrap()).unwrap();
                serde_json::to_string(&Envelope::ok(page)).unwrap()
            }
            Method::Put => {
                let workspace: Workspace =
                    serde_json::from_str(request.body.as_deref().unwrap()).unwrap();
                serde_json::to_string(&Envelope::ok(workspace)).unwrap()
            }
            _ => serde_json::to_string(&Envelope::ok(())).unwrap(),
        };
        ApiResponse {
            status: 200,
            body,
            request_id: None,
        }
    }
}

#[test]
fn execute_plan_uses_patch_for_one_page_and_put_for_replace() {
    let workspace = Workspace::seed("x");
    let page = workspace.pages[0].clone();

    let api = RecordingApi::default();
    let mut client = Client::new(api.clone());
    assert!(execute_plan(
        &mut client,
        &workspace,
        &CommitPlan::Update(vec![page.id.clone()])
    ));
    let requests = api.requests.lock().unwrap().clone();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, Method::Patch);
    assert_eq!(requests[0].path, format!("/api/pages/{}", page.id));

    let api = RecordingApi::default();
    let mut client = Client::new(api.clone());
    assert!(execute_plan(&mut client, &workspace, &CommitPlan::Replace));
    let requests = api.requests.lock().unwrap().clone();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, Method::Put);
    assert_eq!(requests[0].path, "/api/workspace");
}

#[test]
fn pending_edits_does_not_postpone_the_debounce_for_an_unchanged_diff() {
    let diff = PageDiff {
        changed: vec!["a".to_string()],
        ..PageDiff::default()
    };
    let mut pending = PendingEdits::new(0.25);
    pending.observe(&diff, 1.0);
    pending.observe(&diff, 1.2);
    assert!(pending.should_flush(1.25));
}

#[test]
fn search_cache_only_refreshes_when_query_or_version_changes() {
    let mut cache = SearchCache::default();
    assert!(cache.needs_refresh("road", 1));
    assert!(!cache.needs_refresh("road", 1));
    assert!(cache.needs_refresh("roadmap", 1));
    assert!(cache.needs_refresh("roadmap", 2));
    assert!(!cache.needs_refresh("roadmap", 2));
}

#[cfg(feature = "desktop")]
#[test]
fn desktop_feature_exposes_the_screenshot_writer() {
    let _ = ion_of_notes::screenshot::color_image_to_rgba;
}
