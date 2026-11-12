//! Command shapes and the small pure builders shared by the UI.
//!
//! Every user-visible state change is collected as a `Command` and applied
//! through the `Client`; the render modules never mutate server state directly.
//! Keeping the shapes here separates "what can happen" from the state machine
//! in `ui::app` (SRP, spec-017).

use crate::collab::{Access, Role};
use crate::model::{BlockKind, NumberFormat, TimeFormat, Workspace};
use egui::Pos2;

/// Tabs of the page-icon picker (spec-013).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

/// The popover opened from the customize-sidebar rows (spec-062).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum SidebarMenu {
    Shortcut,
    Section,
    RowSettings {
        section: String,
        shortcut: Option<String>,
    },
}

/// Create or rename dialog for the workspace catalog (spec-058).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WorkspaceDialogMode {
    Create,
    Rename,
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

/// The command family that owns the transition. `apply_commands` uses this
/// classification to route one command to one `ui::apply/*` module, so the
/// state machine is not one giant switch (spec-049).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CommandFamily {
    Navigation,
    Library,
    Sharing,
    Blocks,
    Workspace,
    Tabs,
    Ai,
    Settings,
    PageActions,
    Crdt,
    Server,
    NoOp,
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
    SetInviteInput(String),
    SetInviteRole(Role),
    SetInviteMessage(String),
    SendInvites,
    SetGeneralAccess(Access),
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
        query: String,
    },
    CloseSlash,
    ClearSlashDismissed,
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
    HoverBlockDrag {
        block: String,
        before: bool,
    },
    DropBlock,
    CancelBlockDrag,
    SplitBlock(String),
    ClearBlockFocus,
    /// Page-level text selection across blocks (Ctrl+A, Ctrl+C, drag).
    SelectAllBlocks,
    CopyBlockSelection,
    ExtendBlockSelection(String),
    ClearBlockSelection,
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
    ToggleSearchTitleOnly,
    SetSearchCreatedBy(Option<String>),
    SetSearchIn(Option<String>),
    ToggleSearchHasComments,
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
    // Onboarding and templates (spec-027).
    OpenOnboarding,
    CloseOnboarding,
    CompleteOnboarding(String),
    OpenTemplates,
    CloseTemplates,
    ApplyTemplate(String),
    // Settings, help and window zoom.
    SetSettingsPage(String),
    SetSettingsQuery(String),
    SetSettingsStatus(String),
    ExportWorkspace,
    ExportMembers,
    SetLanguage(String),
    SetNumberFormat(NumberFormat),
    SetTimeFormat(TimeFormat),
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
    CopyPageContents,
    TogglePageToc,
    TogglePageDiscussions,
    TogglePageInlineComments,
    TogglePageArchive,
    SetCommentInput(String),
    AddPageComment,
    DeletePageComment(String),
    SetPageStatus(String),
    // Version history (spec-015).
    OpenVersionHistory,
    CloseVersionHistory,
    SetVersionSelected(usize),
    RestoreVersion(usize),
    ToggleWorkspaceMenu,
    CloseWorkspaceMenu,
    SetSidebarCustomize(bool),
    OpenSidebarMenu(SidebarMenu),
    CloseSidebarMenu,
    AddSidebarShortcut(String, String),
    AddSidebarSection(String, crate::model::SidebarSectionSource),
    ToggleSidebarSectionHidden(String),
    ToggleSidebarShortcutHidden(String, String),
    RemoveSidebarSection(String),
    RemoveSidebarShortcut(String, String),
    RenameSidebarSection(String, String),
    RenameSidebarShortcut(String, String, String),
    MoveSidebarSection(String, usize),
    MoveSidebarShortcut(String, String, String),
    SetSidebarShowExcluded(bool),
    ToggleSidebarNav(String),
    OpenWorkspaceCreate,
    OpenWorkspaceRename,
    CloseWorkspaceDialog,
    SetWorkspaceName(String),
    SetWorkspaceIcon(String),
    ConfirmWorkspaceDialog,
    SwitchWorkspace(String),
    DeleteWorkspace(String),
    OpenAppMenu,
    CloseAppMenu,
    ToggleComposeMenu,
    CloseComposeMenu,
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
    // Real-time CRDT and awareness.
    CrdtTextChanged {
        page_id: String,
        block_id: String,
        text: String,
    },
    CrdtCursor {
        page_id: String,
        block_id: String,
        index: usize,
        selection_end: Option<usize>,
    },
    // Server login/settings.
    SetServerUrl(String),
    SetServerEmail(String),
    SetServerPassword(String),
    ConnectServer,
    DisconnectServer,
    NoOp,
}

impl Command {
    /// A pending edit must be written before this command changes the page or
    /// the workspace structure. One list, consumed by `apply_commands`.
    pub(super) fn forces_flush(&self) -> bool {
        matches!(
            self,
            Command::SelectPage(_)
                | Command::SelectTab(_)
                | Command::CloseTab(_)
                | Command::GoBack
                | Command::GoForward
                | Command::OpenLibrary
                | Command::OpenSettings
                | Command::Reload
                | Command::NewPage(_)
                | Command::DeletePage(_)
                | Command::ResetWorkspace
                | Command::ActivateNewTab(_)
                | Command::ActivateSelectedNewTab
                | Command::OpenVersionHistory
                | Command::RestoreVersion(_)
                | Command::CloseAll
                | Command::WindowClose
        )
    }

    /// Commands refused while the current page is locked. One list, consumed
    /// by `apply_commands`.
    pub(super) fn blocked_when_locked(&self) -> bool {
        matches!(
            self,
            Command::DeletePage(_)
                | Command::ToggleFavorite(_)
                | Command::SetTableView(_)
                | Command::OpenSlash { .. }
                | Command::SetBlockKind(_, _)
                | Command::ApplyMarkdown { .. }
                | Command::InsertBlockAfter(_)
                | Command::AppendBlock
                | Command::AddBlock { .. }
                | Command::SplitBlock(_)
                | Command::StartBlockDrag(_)
                | Command::HoverBlockDrag { .. }
                | Command::DropBlock
                | Command::DeleteBlock(_)
                | Command::ToggleTodo(_)
                | Command::CycleStatus(_)
                | Command::AddRow
                | Command::SetPageIcon(_)
                | Command::DuplicatePage
                | Command::RestoreVersion(_)
        )
    }

    /// Route a command to the module that owns its transition.
    pub(super) fn family(&self) -> CommandFamily {
        use CommandFamily::*;
        match self {
            Command::SelectPage(_)
            | Command::NewPage(_)
            | Command::DeletePage(_)
            | Command::ToggleExpand(_)
            | Command::ToggleFavorite(_)
            | Command::OpenSearch
            | Command::ToggleSearch
            | Command::CloseSearch
            | Command::SetSearchQuery(_)
            | Command::OpenSettings
            | Command::CloseSettings
            | Command::OpenOnboarding
            | Command::CloseOnboarding
            | Command::CompleteOnboarding(_)
            | Command::OpenTemplates
            | Command::CloseTemplates
            | Command::ApplyTemplate(_)
            | Command::ToggleTheme
            | Command::ToggleSidebar
            | Command::SetTableView(_)
            | Command::GoBack
            | Command::GoForward
            | Command::GoUpLevel
            | Command::WindowMinimize
            | Command::WindowMaximize
            | Command::WindowClose
            | Command::ToggleFullscreen
            | Command::SetSearchSelected(_)
            | Command::ToggleSearchTitleOnly
            | Command::SetSearchCreatedBy(_)
            | Command::SetSearchIn(_)
            | Command::ToggleSearchHasComments
            | Command::ToggleHighlightPane
            | Command::ToggleFilters => Navigation,
            Command::OpenLibrary | Command::CloseLibrary | Command::SetLibraryTab(_) => Library,
            Command::OpenShare
            | Command::CloseShare
            | Command::SetInviteInput(_)
            | Command::SetInviteRole(_)
            | Command::SetInviteMessage(_)
            | Command::SendInvites
            | Command::SetGeneralAccess(_)
            | Command::OpenInbox
            | Command::CloseInbox => Sharing,
            Command::OpenSlash { .. }
            | Command::CloseSlash
            | Command::ClearSlashDismissed
            | Command::SetBlockKind(_, _)
            | Command::ApplyMarkdown { .. }
            | Command::InsertBlockAfter(_)
            | Command::AppendBlock
            | Command::AddBlock { .. }
            | Command::StartBlockDrag(_)
            | Command::HoverBlockDrag { .. }
            | Command::DropBlock
            | Command::CancelBlockDrag
            | Command::SplitBlock(_)
            | Command::ClearBlockFocus
            | Command::SelectAllBlocks
            | Command::CopyBlockSelection
            | Command::ExtendBlockSelection(_)
            | Command::ClearBlockSelection
            | Command::DeleteBlock(_)
            | Command::ToggleTodo(_)
            | Command::CycleStatus(_)
            | Command::AddRow => Blocks,
            Command::ResetWorkspace
            | Command::Undo
            | Command::Redo
            | Command::DuplicatePage
            | Command::Reload
            | Command::CopyLink
            | Command::CopyPageName
            | Command::CloseAll => Workspace,
            Command::OpenNewTab
            | Command::CloseNewTab
            | Command::SetNewTabQuery(_)
            | Command::SetNewTabSelected(_)
            | Command::MoveNewTabSelection(_)
            | Command::ActivateNewTab(_)
            | Command::ActivateSelectedNewTab
            | Command::SelectTab(_)
            | Command::CloseTab(_)
            | Command::PreviousTab
            | Command::NextTab
            | Command::ReopenClosedTab => Tabs,
            Command::OpenAiChat
            | Command::SetAiQuery(_)
            | Command::AskAi(_)
            | Command::ClearAi
            | Command::SetAiEndpoint(_)
            | Command::SetAiModel(_)
            | Command::ToggleAiCli
            | Command::RefreshAiStatus => Ai,
            Command::SetSettingsPage(_)
            | Command::SetSettingsQuery(_)
            | Command::SetSettingsStatus(_)
            | Command::ExportWorkspace
            | Command::ExportMembers
            | Command::SetLanguage(_)
            | Command::SetNumberFormat(_)
            | Command::SetTimeFormat(_)
            | Command::OpenShortcuts
            | Command::CloseShortcuts
            | Command::ZoomIn
            | Command::ZoomOut
            | Command::ZoomReset
            | Command::ToggleWorkspaceMenu
            | Command::CloseWorkspaceMenu
            | Command::SetSidebarCustomize(_)
            | Command::OpenSidebarMenu(_)
            | Command::CloseSidebarMenu
            | Command::AddSidebarShortcut(_, _)
            | Command::AddSidebarSection(_, _)
            | Command::ToggleSidebarSectionHidden(_)
            | Command::ToggleSidebarShortcutHidden(_, _)
            | Command::RemoveSidebarSection(_)
            | Command::RemoveSidebarShortcut(_, _)
            | Command::RenameSidebarSection(_, _)
            | Command::RenameSidebarShortcut(_, _, _)
            | Command::MoveSidebarSection(_, _)
            | Command::MoveSidebarShortcut(_, _, _)
            | Command::SetSidebarShowExcluded(_)
            | Command::ToggleSidebarNav(_)
            | Command::OpenWorkspaceCreate
            | Command::OpenWorkspaceRename
            | Command::CloseWorkspaceDialog
            | Command::SetWorkspaceName(_)
            | Command::SetWorkspaceIcon(_)
            | Command::ConfirmWorkspaceDialog
            | Command::SwitchWorkspace(_)
            | Command::DeleteWorkspace(_)
            | Command::OpenAppMenu
            | Command::CloseAppMenu
            | Command::ToggleComposeMenu
            | Command::CloseComposeMenu
            | Command::ToggleGridMenu
            | Command::CloseGridMenu
            | Command::ToggleFullWidth
            | Command::ToggleSmallText
            | Command::SetFontStyle(_) => Settings,
            Command::OpenIconPicker
            | Command::CloseIconPicker
            | Command::SetIconTab(_)
            | Command::SetIconQuery(_)
            | Command::SetPageIcon(_)
            | Command::OpenPageActions
            | Command::ClosePageActions
            | Command::CopyPageContents
            | Command::TogglePageToc
            | Command::TogglePageDiscussions
            | Command::TogglePageInlineComments
            | Command::TogglePageArchive
            | Command::SetCommentInput(_)
            | Command::AddPageComment
            | Command::DeletePageComment(_)
            | Command::SetPageStatus(_)
            | Command::OpenVersionHistory
            | Command::CloseVersionHistory
            | Command::SetVersionSelected(_)
            | Command::RestoreVersion(_)
            | Command::ToggleAvailableOffline
            | Command::ToggleLockPage => PageActions,
            Command::CrdtTextChanged { .. } | Command::CrdtCursor { .. } => Crdt,
            Command::SetServerUrl(_)
            | Command::SetServerEmail(_)
            | Command::SetServerPassword(_)
            | Command::ConnectServer
            | Command::DisconnectServer => Server,
            Command::NoOp => NoOp,
        }
    }
}
