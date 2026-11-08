//! Tests for the UI's public visual helpers (public API only).

use not_notion::client::Client;
use not_notion::model::{BlockKind, Page, PageDiff, Workspace};
use not_notion::server::{ApiRequest, ApiResponse, Envelope, Method, WorkspaceApi};
use not_notion::ui::{
    CommitPlan, Palette, PendingEdits, SearchCache, block_menu_options, execute_plan,
    is_valid_email, member_name_from_email, parse_invite_emails, plan_for, settings_matches,
    status_color,
};
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
        ApiResponse { status: 200, body }
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
