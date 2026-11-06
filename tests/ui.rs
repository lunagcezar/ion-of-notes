//! Tests for the UI's public visual helpers (public API only).

use not_notion::model::BlockKind;
use not_notion::ui::{block_menu_options, status_color, Palette};

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
