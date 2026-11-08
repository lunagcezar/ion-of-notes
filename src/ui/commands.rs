//! Command shapes and the small pure builders shared by the UI.
//!
//! Every user-visible state change is collected as a `Command` and applied
//! through the `Client`; the render modules never mutate server state directly.
//! Keeping the shapes here separates "what can happen" from the state machine
//! in `ui::app` (SRP, spec-017).

use crate::model::{BlockKind, Workspace};
use egui::Pos2;

/// Tabs of the page-icon picker (spec-013).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum IconTab {
    Emoji,
    Icons,
    Upload,
}

/// Page body font style chosen from Appearance & layout (spec-013).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FontStyle {
    Default,
    Serif,
    Mono,
}

/// One entry in the "Open in new tab…" switcher opened by the tab-strip `+`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum NewTabChoice {
    NewChat,
    NewPage,
    Page(String),
}

/// Pure builder for the switcher rows (DRY: used by the dialog and by the
/// keyboard activation path). `New chat` and `New page` always lead the list.
pub(super) fn new_tab_choices(ws: &Workspace, query: &str) -> Vec<NewTabChoice> {
    let q = query.trim().to_lowercase();
    let mut choices = vec![NewTabChoice::NewChat, NewTabChoice::NewPage];
    for page in &ws.pages {
        if q.is_empty()
            || page.title.to_lowercase().contains(&q)
            || page.plain_text().to_lowercase().contains(&q)
        {
            choices.push(NewTabChoice::Page(page.id.clone()));
        }
    }
    choices
}

/// Every user-visible state change is collected as a `Command` and applied
/// through the `Client`, so the UI never mutates server state directly.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(super) enum Command {
    SelectPage(String),
    NewPage(Option<String>),
    DeletePage(String),
    ToggleExpand(String),
    ToggleFavorite(String),
    OpenSearch,
    ToggleSearch,
    CloseSearch,
    SetSearchQuery(String),
    OpenSettings,
    CloseSettings,
    ToggleTheme,
    ToggleSidebar,
    SetTableView(bool),
    OpenLibrary,
    CloseLibrary,
    SetLibraryTab(String),
    OpenShare,
    CloseShare,
    OpenInbox,
    CloseInbox,
    GoBack,
    GoForward,
    GoUpLevel,
    WindowMinimize,
    WindowMaximize,
    WindowClose,
    ToggleFullscreen,
    OpenSlash {
        block: String,
        pos: Pos2,
    },
    CloseSlash,
    SetBlockKind(String, BlockKind),
    ApplyMarkdown {
        block: String,
        kind: BlockKind,
        prefix: String,
    },
    InsertBlockAfter(String),
    AppendBlock,
    /// Insert a block above (`above`) or below the given block, then open the
    /// block menu for the new block. Alt+click adds above (spec-013).
    AddBlock {
        block: String,
        above: bool,
        pos: Pos2,
    },
    /// Drag-to-reorder state for the six-dot block handle.
    StartBlockDrag(String),
    HoverBlockDrag(String),
    DropBlock,
    DeleteBlock(String),
    ToggleTodo(String),
    CycleStatus(String),
    AddRow,
    ResetWorkspace,
    // Tabs and the "+" switcher.
    OpenNewTab,
    CloseNewTab,
    SetNewTabQuery(String),
    SetNewTabSelected(usize),
    MoveNewTabSelection(i32),
    ActivateNewTab(NewTabChoice),
    ActivateSelectedNewTab,
    SetSearchSelected(usize),
    SelectTab(usize),
    CloseTab(usize),
    PreviousTab,
    NextTab,
    ReopenClosedTab,
    // AI assistant (Ollama).
    OpenAiChat,
    SetAiQuery(String),
    AskAi(String),
    ClearAi,
    SetAiEndpoint(String),
    SetAiModel(String),
    ToggleAiCli,
    RefreshAiStatus,
    // Settings, help and window zoom.
    SetSettingsPage(String),
    SetSettingsQuery(String),
    SetLanguage(String),
    OpenShortcuts,
    CloseShortcuts,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    // Editing history and misc.
    Undo,
    Redo,
    DuplicatePage,
    Reload,
    CopyLink,
    CopyPageName,
    CloseAll,
    // Page icon customization (spec-013).
    OpenIconPicker,
    CloseIconPicker,
    SetIconTab(IconTab),
    SetIconQuery(String),
    SetPageIcon(String),
    // Page actions / settings from the "..." menu (spec-013).
    OpenPageActions,
    ClosePageActions,
    // Version history (spec-015).
    OpenVersionHistory,
    CloseVersionHistory,
    SetVersionSelected(usize),
    RestoreVersion(usize),
    ToggleWorkspaceMenu,
    CloseWorkspaceMenu,
    OpenAppMenu,
    CloseAppMenu,
    ToggleGridMenu,
    CloseGridMenu,
    ToggleFullWidth,
    ToggleSmallText,
    SetFontStyle(FontStyle),
    ToggleAvailableOffline,
    ToggleLockPage,
    // Search / new-tab window panes (spec-013).
    ToggleHighlightPane,
    ToggleFilters,
    NoOp,
}
