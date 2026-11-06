//! Keyboard shortcuts, kept as a pure, testable table (SRP).
//!
//! The bindings come from two sources: the application menu shown in
//! `reference/picture-6.png` .. `reference/picture-11.png` (File / Edit / View /
//! History / Window / Help) and the published list at
//! <https://www.notion.com/help/keyboard-shortcuts>. This module knows nothing
//! about egui; `ui::app` translates `egui::Key` events into `Combo`s and turns
//! the resulting `Action`s into `Command`s.

/// One exact key combination. `key` is a small lowercase name (see `ui::app`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Combo {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub key: &'static str,
}

impl Combo {
    pub const fn new(ctrl: bool, shift: bool, alt: bool, key: &'static str) -> Self {
        Self { ctrl, shift, alt, key }
    }
}

/// A user-visible operation reachable from a shortcut.
///
/// Clipboard editing (Cut / Copy / Paste / Select All) and the inline text
/// styling keys are deliberately **not** listed here: `egui::TextEdit` already
/// handles them natively, and consuming those events would break text editing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    // File menu (reference/picture-6.png)
    NewTab,
    NewWindow,
    ReopenClosedTab,
    CloseTab,
    Exit,
    // Edit menu (reference/picture-7.png)
    Undo,
    Redo,
    CopyLink,
    CopyPageName,
    // View menu (reference/picture-8.png)
    Reload,
    ForceReload,
    ToggleSidebar,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    ToggleFullscreen,
    // History menu (reference/picture-9.png)
    GoBack,
    GoForward,
    // Window menu (reference/picture-10.png)
    Minimize,
    PreviousTab,
    NextTab,
    // Help / misc
    OpenShortcuts,
    // notion.com/help/keyboard-shortcuts ("Most popular", "Desktop", etc.)
    NewPage,
    QuickSearch,
    CommandSearch,
    FindInPage,
    ToggleTheme,
    AskAi,
    Duplicate,
    MoveTo,
    GoUpLevel,
    OpenSettings,
    // Search popovers (attachments/picture-1.png footer)
    OpenInNewTab,
    CloseOverlay,
    Confirm,
    NextChoice,
    PreviousChoice,
}

/// Resolve one exact combination into an action.
///
/// Matching is exact on all three modifiers, so `Ctrl+Shift+K` never also
/// triggers the `Ctrl+K` binding. Most-specific entries are checked first.
pub fn lookup(combo: Combo) -> Option<Action> {
    use Action::*;
    let Combo { ctrl, shift, alt, key } = combo;

    // Ctrl+Shift+<key> (checked before the plain Ctrl variants).
    if ctrl && shift && !alt {
        match key {
            "n" => return Some(NewWindow),
            "t" => return Some(ReopenClosedTab),
            "z" => return Some(Redo),
            "r" => return Some(ForceReload),
            "k" => return Some(CommandSearch),
            "j" => return Some(AskAi),
            "l" => return Some(ToggleTheme),
            "p" => return Some(MoveTo),
            "u" => return Some(GoUpLevel),
            "q" => return Some(Exit),
            "tab" => return Some(PreviousTab),
            _ => {}
        }
    }
    // Ctrl+Alt+<key>.
    if ctrl && alt && !shift && key == "l" {
        return Some(CopyPageName);
    }
    // Ctrl+<key> (no Shift, no Alt).
    if ctrl && !shift && !alt {
        match key {
            "t" => return Some(NewTab),
            "w" => return Some(CloseTab),
            "n" => return Some(NewPage),
            "z" => return Some(Undo),
            "l" => return Some(CopyLink),
            "r" => return Some(Reload),
            "\\" => return Some(ToggleSidebar),
            "+" | "=" => return Some(ZoomIn),
            "-" => return Some(ZoomOut),
            "0" => return Some(ZoomReset),
            "[" => return Some(GoBack),
            "]" => return Some(GoForward),
            "m" => return Some(Minimize),
            "d" => return Some(Duplicate),
            "j" => return Some(AskAi),
            "k" | "p" => return Some(QuickSearch),
            "f" => return Some(FindInPage),
            "q" => return Some(Exit),
            "tab" => return Some(NextTab),
            "," => return Some(OpenSettings),
            "enter" => return Some(OpenInNewTab),
            _ => {}
        }
    }
    // Bare keys. Enter / arrows are only consumed by `ui::app` while a
    // popover is open, so they still work normally inside text editors.
    if !ctrl && !alt && !shift {
        match key {
            "f11" => return Some(ToggleFullscreen),
            "f1" => return Some(OpenShortcuts),
            "escape" => return Some(CloseOverlay),
            "enter" => return Some(Confirm),
            "arrowdown" => return Some(NextChoice),
            "arrowup" => return Some(PreviousChoice),
            _ => {}
        }
    }
    None
}

/// Guard for global shortcut handling (spec-013).
///
/// A key event is only trusted as an application shortcut when the native
/// window owns the keyboard focus, the pointer is actually over the window,
/// and every modifier the event claims is also held for this frame. The last
/// two conditions fix the reported "keyboard input is hijacked when the mouse
/// pointer is outside the program window": winit drops key-up events when the
/// window loses focus, so egui may keep reporting a stale `Ctrl`/`Alt` and
/// ordinary typing would be mistaken for `Ctrl+<letter>` shortcuts. Requiring
/// a present pointer also stops key events that arrive while the pointer is
/// elsewhere from firing global commands.
///
/// `event` and `frame` are `(ctrl, shift, alt)`. The modifier check is
/// lenient: it only rejects an event that *claims* a modifier the frame does
/// not have, which is exactly the stale-state case.
pub fn shortcut_context_ok(
    window_focused: bool,
    pointer_over_window: bool,
    event: (bool, bool, bool),
    frame: (bool, bool, bool),
) -> bool {
    if !window_focused || !pointer_over_window {
        return false;
    }
    let (ec, es, ea) = event;
    let (fc, fs, fa) = frame;
    (!ec || fc) && (!es || fs) && (!ea || fa)
}

/// Documentation of every shortcut shown in the app menu / help page, plus the
/// ones taken from the Notion help site. Used by the in-app shortcut sheet and
/// by the specs, so the list exists in exactly one place (DRY).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortcutDoc {
    pub label: &'static str,
    pub keys: &'static str,
    pub source: &'static str,
}

pub const SHORTCUT_TABLE: &[ShortcutDoc] = &[
    ShortcutDoc { label: "New tab", keys: "Ctrl+T", source: "app menu · File" },
    ShortcutDoc { label: "New window", keys: "Ctrl+Shift+N", source: "app menu · File / help site" },
    ShortcutDoc { label: "Reopen last closed tab", keys: "Ctrl+Shift+T", source: "app menu · File" },
    ShortcutDoc { label: "Close tab", keys: "Ctrl+W", source: "app menu · File" },
    ShortcutDoc { label: "Exit without saving tabs", keys: "Ctrl+Shift+Q", source: "app menu · File" },
    ShortcutDoc { label: "New page", keys: "Ctrl+N", source: "help site · Desktop" },
    ShortcutDoc { label: "Quick search / jump to page", keys: "Ctrl+K or Ctrl+P", source: "help site · Most popular" },
    ShortcutDoc { label: "Search inside a page", keys: "Ctrl+F", source: "help site · Most popular" },
    ShortcutDoc { label: "Command Search", keys: "Shift+Ctrl+K", source: "help site / settings / attachments" },
    ShortcutDoc { label: "Open highlighted result in a new tab", keys: "Ctrl+↵", source: "attachments footer" },
    ShortcutDoc { label: "Copy link to current page", keys: "Ctrl+L", source: "app menu · Edit / help site" },
    ShortcutDoc { label: "Copy name of current page", keys: "Alt+Ctrl+L", source: "app menu · Edit" },
    ShortcutDoc { label: "Undo / Redo", keys: "Ctrl+Z / Ctrl+Shift+Z", source: "app menu · Edit" },
    ShortcutDoc { label: "Cut / Copy / Paste / Select all", keys: "Ctrl+X / C / V / A", source: "app menu · Edit (native editor)" },
    ShortcutDoc { label: "Reload / Force reload", keys: "Ctrl+R / Ctrl+Shift+R", source: "app menu · View" },
    ShortcutDoc { label: "Toggle developer tools", keys: "Ctrl+Shift+I", source: "app menu · View (runtime)" },
    ShortcutDoc { label: "Show / hide sidebar", keys: "Ctrl+\\", source: "app menu · View" },
    ShortcutDoc { label: "Zoom in / out / actual size", keys: "Ctrl++ / Ctrl+- / Ctrl+0", source: "app menu · View / help site" },
    ShortcutDoc { label: "Toggle full screen", keys: "F11", source: "app menu · View" },
    ShortcutDoc { label: "Back / Forward", keys: "Ctrl+[ / Ctrl+]", source: "app menu · History / help site" },
    ShortcutDoc { label: "Minimize", keys: "Ctrl+M", source: "app menu · Window" },
    ShortcutDoc { label: "Previous / next tab", keys: "Ctrl+Shift+Tab / Ctrl+Tab", source: "app menu · Window" },
    ShortcutDoc { label: "Toggle dark / light mode", keys: "Ctrl+Shift+L", source: "help site · Most popular" },
    ShortcutDoc { label: "Open Notion AI", keys: "Shift+Ctrl+J", source: "settings · Desktop app / help site" },
    ShortcutDoc { label: "Duplicate", keys: "Ctrl+D", source: "help site · Edit blocks" },
    ShortcutDoc { label: "Move to", keys: "Ctrl+Shift+P", source: "page actions menu" },
    ShortcutDoc { label: "Go up one level", keys: "Ctrl+Shift+U", source: "help site · Create and style" },
    ShortcutDoc { label: "Preferences", keys: "Ctrl+,", source: "desktop convention" },
    ShortcutDoc { label: "Close / dismiss", keys: "Esc", source: "app / help site" },
    ShortcutDoc { label: "Keyboard shortcut sheet", keys: "F1", source: "in-app help" },
    ShortcutDoc { label: "Markdown: H1 / H2 / H3", keys: "# / ## / ### + space", source: "help site · Markdown style" },
    ShortcutDoc { label: "Markdown: bullet / to-do / quote", keys: "* , [] , > + space", source: "help site · Markdown style" },
];
