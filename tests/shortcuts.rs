//! Tests for the pure keyboard-shortcut table (public API only).
//!
//! Bindings are taken from the app menu (reference/picture-6..11) and
//! <https://www.notion.com/help/keyboard-shortcuts>.

use ion_of_notes::shortcuts::{Action, Combo, SHORTCUT_TABLE, lookup, shortcut_context_ok};

fn combo(ctrl: bool, shift: bool, alt: bool, key: &'static str) -> Combo {
    Combo::new(ctrl, shift, alt, key)
}

#[test]
fn file_and_window_menu_bindings_resolve() {
    assert_eq!(lookup(combo(true, false, false, "t")), Some(Action::NewTab));
    assert_eq!(
        lookup(combo(true, true, false, "n")),
        Some(Action::NewWindow)
    );
    assert_eq!(
        lookup(combo(true, true, false, "t")),
        Some(Action::ReopenClosedTab)
    );
    assert_eq!(
        lookup(combo(true, false, false, "w")),
        Some(Action::CloseTab)
    );
    assert_eq!(lookup(combo(true, true, false, "q")), Some(Action::Exit));
    assert_eq!(
        lookup(combo(true, false, false, "m")),
        Some(Action::Minimize)
    );
    assert_eq!(
        lookup(combo(true, false, false, "tab")),
        Some(Action::NextTab)
    );
    assert_eq!(
        lookup(combo(true, true, false, "tab")),
        Some(Action::PreviousTab)
    );
}

#[test]
fn edit_and_view_menu_bindings_resolve() {
    assert_eq!(lookup(combo(true, false, false, "z")), Some(Action::Undo));
    assert_eq!(lookup(combo(true, true, false, "z")), Some(Action::Redo));
    assert_eq!(
        lookup(combo(true, false, false, "l")),
        Some(Action::CopyLink)
    );
    assert_eq!(
        lookup(combo(true, false, true, "l")),
        Some(Action::CopyPageName)
    );
    assert_eq!(lookup(combo(true, false, false, "r")), Some(Action::Reload));
    assert_eq!(
        lookup(combo(true, true, false, "r")),
        Some(Action::ForceReload)
    );
    assert_eq!(
        lookup(combo(true, false, false, "\\")),
        Some(Action::ToggleSidebar)
    );
    assert_eq!(lookup(combo(true, false, false, "+")), Some(Action::ZoomIn));
    assert_eq!(
        lookup(combo(true, false, false, "-")),
        Some(Action::ZoomOut)
    );
    assert_eq!(
        lookup(combo(true, false, false, "0")),
        Some(Action::ZoomReset)
    );
    assert_eq!(
        lookup(combo(false, false, false, "f11")),
        Some(Action::ToggleFullscreen)
    );
}

#[test]
fn history_and_help_site_bindings_resolve() {
    assert_eq!(lookup(combo(true, false, false, "[")), Some(Action::GoBack));
    assert_eq!(
        lookup(combo(true, false, false, "]")),
        Some(Action::GoForward)
    );
    assert_eq!(
        lookup(combo(true, false, false, "k")),
        Some(Action::QuickSearch)
    );
    assert_eq!(
        lookup(combo(true, false, false, "p")),
        Some(Action::QuickSearch)
    );
    assert_eq!(
        lookup(combo(true, true, false, "k")),
        Some(Action::CommandSearch)
    );
    assert_eq!(
        lookup(combo(true, false, false, "n")),
        Some(Action::NewPage)
    );
    assert_eq!(
        lookup(combo(true, true, false, "l")),
        Some(Action::ToggleTheme)
    );
    assert_eq!(lookup(combo(true, true, false, "j")), Some(Action::AskAi));
    assert_eq!(
        lookup(combo(true, false, false, "d")),
        Some(Action::Duplicate)
    );
    assert_eq!(
        lookup(combo(true, false, false, "f")),
        Some(Action::FindInPage)
    );
    assert_eq!(
        lookup(combo(true, false, false, ",")),
        Some(Action::OpenSettings)
    );
}

#[test]
fn popover_keys_and_escape_resolve() {
    assert_eq!(
        lookup(combo(true, false, false, "enter")),
        Some(Action::OpenInNewTab)
    );
    assert_eq!(
        lookup(combo(false, false, false, "enter")),
        Some(Action::Confirm)
    );
    assert_eq!(
        lookup(combo(false, false, false, "arrowdown")),
        Some(Action::NextChoice)
    );
    assert_eq!(
        lookup(combo(false, false, false, "arrowup")),
        Some(Action::PreviousChoice)
    );
    assert_eq!(
        lookup(combo(false, false, false, "escape")),
        Some(Action::CloseOverlay)
    );
    assert_eq!(
        lookup(combo(false, false, false, "f1")),
        Some(Action::OpenShortcuts)
    );
}

#[test]
fn modifier_matching_is_exact() {
    // Ctrl+Shift+K must not fall through to the plain Ctrl+K binding.
    assert_eq!(
        lookup(combo(true, true, false, "k")),
        Some(Action::CommandSearch)
    );
    assert_ne!(
        lookup(combo(true, true, false, "k")),
        Some(Action::QuickSearch)
    );
    // A stray Alt must not match a Ctrl-only binding.
    assert_eq!(lookup(combo(true, false, true, "t")), None);
    // Clipboard keys stay with the native editor (not consumed globally).
    assert_eq!(lookup(combo(true, false, false, "c")), None);
    assert_eq!(lookup(combo(true, false, false, "v")), None);
    assert_eq!(lookup(combo(true, false, false, "a")), None);
    assert_eq!(lookup(combo(false, false, false, "q")), None);
}

#[test]
fn shortcuts_require_a_present_pointer_and_window_focus() {
    // The reported bug: key events arriving after the pointer left the
    // window must not fire global commands ("keyboard hijacking").
    assert!(!shortcut_context_ok(
        true,
        false,
        (true, false, false),
        (true, false, false)
    ));
    assert!(!shortcut_context_ok(
        false,
        true,
        (false, false, false),
        (false, false, false)
    ));
    assert!(shortcut_context_ok(
        true,
        true,
        (false, false, false),
        (false, false, false)
    ));
}

#[test]
fn stale_modifiers_are_rejected_but_real_shortcuts_are_allowed() {
    // A key event claiming Ctrl while no Ctrl is held (focus was lost while
    // the modifier was down) would otherwise turn typing into shortcuts.
    assert!(!shortcut_context_ok(
        true,
        true,
        (true, false, false),
        (false, false, false)
    ));
    assert!(!shortcut_context_ok(
        true,
        true,
        (false, true, false),
        (false, false, false)
    ));
    assert!(!shortcut_context_ok(
        true,
        true,
        (false, false, true),
        (false, false, false)
    ));
    // A genuine Ctrl+K and a plain key are both fine.
    assert!(shortcut_context_ok(
        true,
        true,
        (true, false, false),
        (true, false, false)
    ));
    assert!(shortcut_context_ok(
        true,
        true,
        (true, true, false),
        (true, true, false)
    ));
}

#[test]
fn documented_table_is_complete_and_consistent() {
    assert!(SHORTCUT_TABLE.len() >= 25);
    let labels: Vec<&str> = SHORTCUT_TABLE.iter().map(|d| d.label).collect();
    assert!(labels.contains(&"New tab"));
    assert!(labels.contains(&"Toggle dark / light mode"));
    assert!(labels.contains(&"Copy link to current page"));
    assert!(
        SHORTCUT_TABLE
            .iter()
            .all(|d| !d.keys.is_empty() && !d.source.is_empty())
    );
}
