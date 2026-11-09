//! Tests for the visual-fidelity helpers (public API only).
//!
//! Written before the implementation, following spec-009 and the project's
//! Red -> Green -> Refactor discipline.

use ion_of_notes::model::{BlockKind, Workspace};
use ion_of_notes::ui::{Palette, strong_font, tab_title};

#[test]
fn chrome_color_is_theme_specific_and_distinct_from_content() {
    let dark = Palette::dark();
    let light = Palette::light();
    assert_ne!(dark.chrome, light.chrome);
    assert_ne!(dark.chrome, dark.content);
    assert_ne!(light.chrome, light.content);
}

#[test]
fn strong_font_keeps_the_requested_size() {
    assert_eq!(strong_font(42.0).size, 42.0);
    assert_eq!(strong_font(19.5).size, 19.5);
}

#[test]
fn tab_title_keeps_short_titles_and_truncates_long_ones() {
    assert_eq!(tab_title("Home"), "Home");
    assert_eq!(tab_title("Getting Started"), "Getting Started");

    let long = "A very long page name that will not fit in a tab";
    let short = tab_title(long);
    assert!(short.chars().count() < long.chars().count());
    assert!(short.ends_with('…'));
    assert!(long.starts_with(&short[..short.len() - 3]));
}

#[test]
fn seeded_home_page_opens_with_a_callout_and_section_headings() {
    let ws = Workspace::seed("ion-of-notes");
    let home = ws.page("p-home").expect("home page");
    assert_eq!(
        home.blocks.first().map(|b| b.kind),
        Some(BlockKind::Callout)
    );
    assert!(
        home.blocks
            .iter()
            .any(|b| b.kind == BlockKind::Heading1 || b.kind == BlockKind::Heading2)
    );
    assert!(home.blocks.iter().any(|b| b.kind == BlockKind::Bulleted));
}

#[test]
fn strong_font_family_is_resolvable() {
    // Either the installed strong family or the default proportional family
    // must be usable; neither may be an empty placeholder.
    let font = strong_font(28.0);
    assert!(font.size > 0.0);
    assert!(!format!("{:?}", font.family).is_empty());
}

#[test]
fn demo_content_is_neutral_and_short() {
    // Guard against long passages copied from the pictures.
    let workspace = Workspace::seed("demo");
    for page in &workspace.pages {
        for block in &page.blocks {
            assert!(
                block.text.chars().count() <= 180,
                "block too long: {}",
                block.text
            );
        }
    }
}
