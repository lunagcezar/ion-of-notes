//! Tests for the UI's public visual helpers (public API only).

use ion_of_notes::client::Client;
use ion_of_notes::model::{BlockKind, NumberFormat, Page, PageDiff, TimeFormat, Workspace};
use ion_of_notes::server::realtime::Awareness;
use ion_of_notes::server::{
    ApiRequest, ApiResponse, CollabTransport, Envelope, Method, RequestTransport,
};
use ion_of_notes::ui::{
    CommitPlan, Palette, PendingEdits, SearchCache, block_menu_options, execute_plan,
    format_number, format_timestamp, is_valid_email, member_name_from_email, pages_for_view,
    parse_invite_emails, plan_for, rows_for, section_matches, settings_matches, status_color,
    visible_rows,
};
use ion_of_notes::ui::{
    EMOJI, ICONS, app_bar_left_width, block_outside_viewport, catalog_matches, drop_index,
    estimate_block_height, frame_may_mutate, glyph_visual_offset, index_cursors, page_icon_letter,
    search_page_matches, split_index,
};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

#[test]
fn ui_constants_are_declaration_only_and_even() {
    let source = include_str!("../src/ui/constants.rs");
    assert!(
        source.contains("//!"),
        "constants module needs a doc comment"
    );
    for forbidden in ["fn ", "struct ", "enum ", "impl "] {
        assert!(
            !source.contains(forbidden),
            "constants module must not contain `{forbidden}`"
        );
    }

    let required = [
        "SIDEBAR_CONTENT_PADDING",
        "ROW_HEIGHT",
        "ROW_RADIUS",
        "ICON_SIZE",
        "SECTION_GAP",
        "SETTINGS_RAIL_WIDTH",
        "BREADCRUMB_HEIGHT",
        "BLOCK_ROW_HEIGHT",
        "POPOVER_RADIUS",
        "FONT_BODY",
    ];
    for name in required {
        assert!(
            source.contains(&format!("const {name}:")),
            "missing shared metric {name}"
        );
    }

    for line in source.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("pub(crate) const ") else {
            continue;
        };
        let Some((declaration, value)) = rest.split_once('=') else {
            panic!("constant without a value: {line}");
        };
        let Some((name, kind)) = declaration.split_once(':') else {
            panic!("constant without a type: {line}");
        };
        let name = name.trim();
        if !matches!(kind.trim(), "f32" | "u8") {
            continue;
        }
        let value = value.trim().trim_end_matches(';').trim();
        if matches!(name, "STROKE_HAIRLINE" | "FONT_SMALL") {
            continue;
        }
        if let Ok(number) = value.parse::<f64>() {
            assert_eq!(
                number % 2.0,
                0.0,
                "{name} must use an even value, found {number}"
            );
        }
    }
}

#[test]
fn shared_sidebar_metrics_are_used_by_both_sidebars() {
    let main = include_str!("../src/ui/render/sidebar.rs");
    let settings = include_str!("../src/ui/dialogs/settings.rs");
    assert!(main.contains("SIDEBAR_CONTENT_PADDING"));
    assert!(settings.contains("SIDEBAR_CONTENT_PADDING"));
}

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
fn glyphs_are_lifted_in_proportion_to_their_font_size() {
    assert!(glyph_visual_offset(20.0) > 0.0);
    assert!(glyph_visual_offset(40.0) > glyph_visual_offset(20.0));
    assert_eq!(glyph_visual_offset(0.0), 0.0);
}

#[test]
fn catalog_metadata_is_lowercase_for_normalized_search() {
    for entry in EMOJI {
        assert_eq!(entry.name, entry.name.to_lowercase(), "emoji name");
        assert_eq!(
            entry.keywords,
            entry.keywords.to_lowercase(),
            "emoji keywords"
        );
    }
    for entry in ICONS {
        assert_eq!(entry.name, entry.name.to_lowercase(), "icon name");
    }
}

#[test]
fn idle_and_popup_frames_skip_the_undo_snapshot() {
    assert!(!frame_may_mutate(true, false, false, false));
    assert!(!frame_may_mutate(true, false, true, true));
    assert!(frame_may_mutate(true, false, true, false));
    assert!(frame_may_mutate(true, true, false, true));
    assert!(frame_may_mutate(false, false, false, true));
}

#[test]
fn cursor_index_groups_by_page_and_block() {
    let cursor = |page: &str, block: &str| Awareness {
        session_id: "s".to_string(),
        member_id: "m".to_string(),
        name: "n".to_string(),
        color: "#000000".to_string(),
        page_id: page.to_string(),
        block_id: block.to_string(),
        index: 0,
        selection_end: None,
    };
    let cursors = vec![
        cursor("page-a", "block-1"),
        cursor("page-a", "block-1"),
        cursor("page-a", "block-2"),
        cursor("page-b", "block-1"),
    ];
    let index = index_cursors(&cursors, "page-a");
    assert_eq!(index.get("block-1").map(Vec::len), Some(2));
    assert_eq!(index.get("block-2").map(Vec::len), Some(1));
    assert!(!index.contains_key("page-b"));
}

#[test]
fn number_formats_group_thousands_and_decimals() {
    assert_eq!(format_number(0.0, NumberFormat::CommaThousands), "0");
    assert_eq!(format_number(1234.0, NumberFormat::CommaThousands), "1,234");
    assert_eq!(
        format_number(1234.5, NumberFormat::CommaThousands),
        "1,234.5"
    );
    assert_eq!(format_number(1234.5, NumberFormat::DotThousands), "1.234,5");
    assert_eq!(
        format_number(-1234.5, NumberFormat::CommaThousands),
        "-1,234.5"
    );
}

#[test]
fn time_formats_convert_between_24_and_12_hour_clocks() {
    assert_eq!(
        format_timestamp("Nov 09, 2026 15:05", TimeFormat::Hour24),
        "Nov 09, 2026 15:05"
    );
    assert_eq!(
        format_timestamp("Nov 09, 2026 15:05", TimeFormat::Hour12),
        "Nov 09, 2026 3:05 PM"
    );
    assert_eq!(
        format_timestamp("not a date", TimeFormat::Hour12),
        "not a date"
    );
}

#[test]
fn app_bar_follows_the_sidebar_but_keeps_a_compact_width() {
    assert_eq!(app_bar_left_width(266.0, 1.0), 266.0);
    assert_eq!(app_bar_left_width(266.0, 0.0), 92.0);
    assert_eq!(app_bar_left_width(266.0, 0.5), 133.0);
}

#[test]
fn page_icons_fall_back_to_the_uppercase_title_letter() {
    assert_eq!(page_icon_letter("", "home"), "H");
    assert_eq!(page_icon_letter("  ", "  getting started"), "G");
    assert_eq!(page_icon_letter("★", "home"), "★");
    assert_eq!(page_icon_letter("", ""), "?");
    assert_eq!(page_icon_letter("", "éclair"), "É");
}

#[test]
fn search_filters_match_title_creator_scope_and_comments() {
    let mut ws = Workspace::new("w");
    let mut parent = Page::new("parent", "Home", "");
    parent.created_by = "Owner".to_string();
    let mut child = Page::new("child", "Child page", "");
    child.parent = Some("parent".to_string());
    child.created_by = "Guest".to_string();
    child.comments.push(ion_of_notes::model::Comment::new(
        "c",
        "Guest",
        "hello",
        "Nov 12, 2026 10:00",
    ));
    ws.add_page(parent).unwrap();
    ws.add_page(child).unwrap();

    let child_page = ws.page("child").unwrap();
    assert!(search_page_matches(
        &ws,
        child_page,
        "child",
        true,
        Some("Guest"),
        Some("parent"),
        true
    ));
    assert!(!search_page_matches(
        &ws, child_page, "home", true, None, None, false
    ));
    assert!(!search_page_matches(
        &ws,
        child_page,
        "",
        false,
        Some("Owner"),
        None,
        false
    ));
    assert!(search_page_matches(
        &ws,
        child_page,
        "",
        false,
        None,
        Some("parent"),
        false
    ));
}

#[test]
fn block_drop_and_split_indices_cover_the_edges() {
    assert_eq!(drop_index(0, None, false), 0);
    assert_eq!(drop_index(3, Some(0), true), 0);
    assert_eq!(drop_index(3, Some(0), false), 1);
    assert_eq!(drop_index(3, Some(2), false), 3);
    assert_eq!(drop_index(3, None, false), 3);
    assert_eq!(drop_index(3, Some(99), true), 3);
    assert_eq!(split_index(3, 0), Some(1));
    assert_eq!(split_index(3, 2), Some(3));
    assert_eq!(split_index(0, 0), None);
}

#[test]
fn block_viewport_culling_keeps_the_overscan_band() {
    let viewport = (100.0, 500.0);
    assert!(block_outside_viewport(-200.0, 40.0, viewport, 100.0));
    assert!(!block_outside_viewport(90.0, 40.0, viewport, 100.0));
    assert!(!block_outside_viewport(480.0, 40.0, viewport, 100.0));
    assert!(block_outside_viewport(650.0, 40.0, viewport, 100.0));
}

#[test]
fn uncached_block_estimates_grow_with_kind_and_text() {
    assert!(
        estimate_block_height(BlockKind::Heading1, 0)
            > estimate_block_height(BlockKind::Paragraph, 0)
    );
    assert!(
        estimate_block_height(BlockKind::Paragraph, 400)
            > estimate_block_height(BlockKind::Paragraph, 0)
    );
    assert!(
        estimate_block_height(BlockKind::Table, 0) > estimate_block_height(BlockKind::TableRow, 0)
    );
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

    let export = visible_rows("General", "export");
    assert!(export.iter().any(|row| row.id == "data_export"));
    assert!(export.iter().all(|row| row.section == "Data export"));

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

#[test]
fn pending_edits_waiting_clears_force_but_keeps_the_change() {
    let mut pending = PendingEdits::new(0.25);
    pending.mark(1.0);
    pending.force();
    pending.waiting();
    assert!(pending.is_pending());
    assert!(!pending.should_flush(1.1));
    assert!(pending.should_flush(1.25));
}

#[derive(Clone, Default)]
struct RecordingApi {
    requests: Arc<Mutex<Vec<ApiRequest>>>,
}

impl RequestTransport for RecordingApi {
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

impl CollabTransport for RecordingApi {
    fn crdt_sync(
        &mut self,
        _page_id: &str,
        _state_vector: Option<&[u8]>,
    ) -> Result<(Vec<u8>, Vec<u8>), String> {
        Err("not available".to_string())
    }

    fn apply_crdt_update(
        &mut self,
        _page_id: &str,
        _update: &[u8],
        _actor: &str,
    ) -> Result<Vec<u8>, String> {
        Err("not available".to_string())
    }

    fn subscribe_realtime(
        &self,
    ) -> Option<tokio::sync::broadcast::Receiver<ion_of_notes::server::realtime::ServerMessage>>
    {
        None
    }

    fn publish_awareness(
        &mut self,
        _awareness: ion_of_notes::server::realtime::Awareness,
    ) -> Result<(), String> {
        Err("not available".to_string())
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
