//! Application state, command handling, and the eframe::App implementation.
//! This module owns the state machine; drawing lives in `render` and `dialogs`.
//!
//! Keyboard input is translated by the pure `shortcuts` table into `Action`s
//! and then into `Command`s, so every shortcut follows the same client-bound
//! path as a mouse click. The AI assistant runs through the `ollama` adapter
//! on a background thread; the window only polls a shared result slot.

use crate::app;
use crate::client::Client;
use crate::collab::PageVersion;
use crate::model::{
    Block, BlockKind, Page, PageDraft, SearchResult, TableRow, Workspace,
};
use crate::ollama::{self, OllamaConfig, OllamaStatus};
use crate::screenshot;
use crate::server::WorkspaceServer;
use crate::shortcuts::{self, Action, Combo};
use egui::Pos2;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use super::dialogs::{
    render_app_menu, render_grid_menu, render_inbox, render_new_tab, render_page_actions,
    render_search, render_settings, render_share, render_shortcuts, render_slash,
    render_version_history, render_workspace_menu,
};
use super::page_icon::render_icon_picker;
use super::palette::{apply_theme, Palette};
use super::render::{render_content, render_sidebar, render_title_bar, TITLE_BAR_HEIGHT};
use super::search_cache::SearchCache;

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

/// A snapshot of non-workspace app state passed to pure render functions.
pub(super) struct ViewState {
    pub(super) current_page: String,
    pub(super) tabs: Vec<String>,
    pub(super) active_tab: usize,
    pub(super) sidebar_open: bool,
    pub(super) expanded: HashSet<String>,
    pub(super) search_open: bool,
    pub(super) search_query: String,
    pub(super) search_results: Arc<Vec<SearchResult>>,
    pub(super) settings_open: bool,
    pub(super) settings_page: String,
    pub(super) settings_query: String,
    pub(super) language: String,
    pub(super) dark: bool,
    pub(super) table_view: bool,
    pub(super) library_open: bool,
    pub(super) library_tab: String,
    pub(super) share_open: bool,
    pub(super) inbox_open: bool,
    pub(super) slash_for: Option<String>,
    pub(super) slash_pos: Pos2,
    pub(super) new_tab_open: bool,
    pub(super) new_tab_query: String,
    pub(super) new_tab_selected: usize,
    pub(super) ai_query: String,
    pub(super) ai_answer: Option<String>,
    pub(super) ai_error: Option<String>,
    pub(super) ai_busy: bool,
    pub(super) ai_status: String,
    pub(super) ai_connected: bool,
    pub(super) ai_endpoint: String,
    pub(super) ai_model: String,
    pub(super) ai_prefer_cli: bool,
    pub(super) shortcuts_open: bool,
    pub(super) search_selected: usize,
    pub(super) highlight_pane: bool,
    pub(super) filters: bool,
    pub(super) icon_picker_open: bool,
    pub(super) icon_picker_tab: IconTab,
    pub(super) icon_picker_query: String,
    pub(super) page_actions_open: bool,
    pub(super) version_history_open: bool,
    pub(super) versions: Arc<Vec<PageVersion>>,
    pub(super) version_selected: usize,
    pub(super) font_style: FontStyle,
    pub(super) full_width: bool,
    pub(super) small_text: bool,
    pub(super) available_offline: bool,
    pub(super) lock_page: bool,
    pub(super) workspace_menu_open: bool,
    pub(super) app_menu_open: bool,
    pub(super) grid_menu_open: bool,
    pub(super) drag_from: Option<String>,
    pub(super) drag_to: Option<String>,
}

pub struct NotionApp {
    client: Client<WorkspaceServer>,
    workspace: Workspace,
    last_committed: Workspace,
    current_page: String,
    tabs: Vec<String>,
    active_tab: usize,
    closed_tabs: Vec<String>,
    sidebar_open: bool,
    expanded: HashSet<String>,
    search_open: bool,
    search_query: String,
    search_results: Arc<Vec<SearchResult>>,
    search_cache: SearchCache,
    settings_open: bool,
    settings_page: String,
    settings_query: String,
    language: String,
    dark: bool,
    theme_dirty: bool,
    table_view: bool,
    library_open: bool,
    library_tab: String,
    share_open: bool,
    inbox_open: bool,
    nav_history: Vec<String>,
    nav_pos: usize,
    slash_for: Option<String>,
    slash_pos: Pos2,
    pending_reload: bool,
    screenshot: Option<PathBuf>,
    frames: u32,
    screenshot_requested: bool,
    screenshot_done: Arc<AtomicBool>,
    // "Open in new tab…" switcher.
    new_tab_open: bool,
    new_tab_query: String,
    new_tab_selected: usize,
    // AI assistant.
    ai_config: OllamaConfig,
    ai_status: OllamaStatus,
    ai_query: String,
    ai_answer: Option<String>,
    ai_error: Option<String>,
    ai_busy: bool,
    ai_slot: Arc<Mutex<Option<Result<String, String>>>>,
    // Help / window.
    shortcuts_open: bool,
    zoom: f32,
    zoom_dirty: bool,
    fullscreen: bool,
    undo_stack: Vec<Workspace>,
    redo_stack: Vec<Workspace>,
    search_selected: usize,
    highlight_pane: bool,
    filters: bool,
    icon_picker_open: bool,
    icon_picker_tab: IconTab,
    icon_picker_query: String,
    page_actions_open: bool,
    version_history_open: bool,
    versions: Arc<Vec<PageVersion>>,
    version_selected: usize,
    font_style: FontStyle,
    full_width: bool,
    small_text: bool,
    workspace_menu_open: bool,
    app_menu_open: bool,
    grid_menu_open: bool,
    drag_from: Option<String>,
    drag_to: Option<String>,
}

impl NotionApp {
    pub fn new(cc: &eframe::CreationContext<'_>, screenshot: Option<PathBuf>) -> Self {
        crate::i18n::set_locale("en");
        // The composition root (app layer) wires the concrete JSON adapter to
        // the server/client ports; the window only consumes the ready graph.
        let (client, workspace) = app::build_desktop();
        let current_page = workspace
            .pages
            .first()
            .map(|p| p.id.clone())
            .unwrap_or_default();
        apply_theme(&cc.egui_ctx, true);
        // A harmless demo hook: with NOT_NOTION_SHOW set, one of the opened
        // states from the reference pictures is shown on startup. Normal runs
        // (and the rebuild check) never set it.
        let demo = std::env::var("NOT_NOTION_SHOW").unwrap_or_default();
        let demo_current_page = if demo == "tasks" {
            workspace
                .page("p-tasks")
                .map(|p| p.id.clone())
                .unwrap_or_else(|| current_page.clone())
        } else {
            current_page.clone()
        };
        let nav_history = vec![demo_current_page.clone()];
        let ai_config = OllamaConfig::from_env();
        let ai_status = ollama::detect(&ai_config);
        Self {
            client,
            last_committed: workspace.clone(),
            workspace,
            current_page: demo_current_page.clone(),
            tabs: vec![demo_current_page],
            active_tab: 0,
            closed_tabs: Vec::new(),
            sidebar_open: true,
            expanded: HashSet::new(),
            search_open: demo == "search",
            search_query: String::new(),
            search_results: Arc::new(Vec::new()),
            search_cache: SearchCache::default(),
            settings_open: matches!(demo.as_str(), "settings" | "ai"),
            settings_page: if demo == "ai" {
                "AI assistant".to_string()
            } else {
                "Preferences".to_string()
            },
            settings_query: String::new(),
            language: "en".to_string(),
            dark: true,
            theme_dirty: false,
            table_view: true,
            library_open: demo == "library",
            library_tab: "Recents".to_string(),
            share_open: demo == "share",
            inbox_open: demo == "inbox",
            nav_history,
            nav_pos: 0,
            slash_for: None,
            slash_pos: Pos2::new(360.0, 240.0),
            pending_reload: false,
            screenshot,
            frames: 0,
            screenshot_requested: false,
            screenshot_done: Arc::new(AtomicBool::new(false)),
            new_tab_open: demo == "newtab",
            new_tab_query: String::new(),
            new_tab_selected: 0,
            ai_config,
            ai_status,
            ai_query: String::new(),
            ai_answer: None,
            ai_error: None,
            ai_busy: false,
            ai_slot: Arc::new(Mutex::new(None)),
            shortcuts_open: demo == "shortcuts",
            zoom: 1.0,
            zoom_dirty: false,
            fullscreen: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            search_selected: 0,
            highlight_pane: true,
            filters: false,
            icon_picker_open: demo == "iconpicker",
            icon_picker_tab: IconTab::Emoji,
            icon_picker_query: String::new(),
            page_actions_open: demo == "pageactions",
            version_history_open: false,
            versions: Arc::new(Vec::new()),
            version_selected: 0,
            font_style: FontStyle::Default,
            full_width: false,
            small_text: false,
            workspace_menu_open: false,
            app_menu_open: false,
            grid_menu_open: false,
            drag_from: None,
            drag_to: None,
        }
    }

    fn view_state(&self) -> ViewState {
        let (available_offline, lock_page) = self
            .workspace
            .page(&self.current_page)
            .map(|p| (p.available_offline, p.locked))
            .unwrap_or((false, false));
        ViewState {
            current_page: self.current_page.clone(),
            tabs: self.tabs.clone(),
            active_tab: self.active_tab,
            sidebar_open: self.sidebar_open,
            expanded: self.expanded.clone(),
            search_open: self.search_open,
            search_query: self.search_query.clone(),
            search_results: Arc::clone(&self.search_results),
            settings_open: self.settings_open,
            settings_page: self.settings_page.clone(),
            settings_query: self.settings_query.clone(),
            language: self.language.clone(),
            dark: self.dark,
            table_view: self.table_view,
            library_open: self.library_open,
            library_tab: self.library_tab.clone(),
            share_open: self.share_open,
            inbox_open: self.inbox_open,
            slash_for: self.slash_for.clone(),
            slash_pos: self.slash_pos,
            new_tab_open: self.new_tab_open,
            new_tab_query: self.new_tab_query.clone(),
            new_tab_selected: self.new_tab_selected,
            ai_query: self.ai_query.clone(),
            ai_answer: self.ai_answer.clone(),
            ai_error: self.ai_error.clone(),
            ai_busy: self.ai_busy,
            ai_status: self.ai_status.label(),
            ai_connected: self.ai_status.connected(),
            ai_endpoint: self.ai_config.endpoint.clone(),
            ai_model: self.ai_config.model.clone(),
            ai_prefer_cli: self.ai_config.prefer_cli,
            shortcuts_open: self.shortcuts_open,
            search_selected: self.search_selected,
            highlight_pane: self.highlight_pane,
            filters: self.filters,
            icon_picker_open: self.icon_picker_open,
            icon_picker_tab: self.icon_picker_tab,
            icon_picker_query: self.icon_picker_query.clone(),
            page_actions_open: self.page_actions_open,
            version_history_open: self.version_history_open,
            versions: Arc::clone(&self.versions),
            version_selected: self.version_selected,
            font_style: self.font_style,
            full_width: self.full_width,
            small_text: self.small_text,
            available_offline,
            lock_page,
            workspace_menu_open: self.workspace_menu_open,
            app_menu_open: self.app_menu_open,
            grid_menu_open: self.grid_menu_open,
            drag_from: self.drag_from.clone(),
            drag_to: self.drag_to.clone(),
        }
    }

    fn find_block_mut(&mut self, block_id: &str) -> Option<&mut Block> {
        self.workspace
            .pages
            .iter_mut()
            .flat_map(|p| p.blocks.iter_mut())
            .find(|b| b.id == block_id)
    }

    fn current_page_mut(&mut self) -> Option<&mut Page> {
        let id = self.current_page.clone();
        self.workspace.page_mut(&id)
    }

    fn close_overlays(&mut self) {
        self.search_open = false;
        self.settings_open = false;
        self.share_open = false;
        self.inbox_open = false;
        self.slash_for = None;
        self.new_tab_open = false;
        self.shortcuts_open = false;
        self.icon_picker_open = false;
        self.page_actions_open = false;
        self.version_history_open = false;
        self.drag_from = None;
        self.drag_to = None;
    }

    /// Add `id` as a new active tab (the "+" / Ctrl+T / Ctrl+Enter behaviour).
    fn open_in_tab(&mut self, id: String) {
        self.tabs.push(id.clone());
        self.active_tab = self.tabs.len() - 1;
        self.current_page = id.clone();
        self.library_open = false;
        self.push_history(id);
    }

    fn poll_ai(&mut self) {
        if !self.ai_busy {
            return;
        }
        let done = self.ai_slot.lock().ok().and_then(|mut guard| guard.take());
        if let Some(result) = done {
            match result {
                Ok(answer) => {
                    self.ai_answer = Some(answer);
                    self.ai_error = None;
                }
                Err(error) => {
                    self.ai_error = Some(error);
                    self.ai_answer = None;
                }
            }
            self.ai_busy = false;
        }
    }

    fn command_for_action(&self, action: Action) -> Command {
        use Action::*;
        match action {
            NewTab | NewWindow => Command::OpenNewTab,
            ReopenClosedTab => Command::ReopenClosedTab,
            CloseTab => Command::CloseTab(self.active_tab),
            Exit => Command::WindowClose,
            NewPage => Command::NewPage(None),
            Undo => Command::Undo,
            Redo => Command::Redo,
            CopyLink => Command::CopyLink,
            CopyPageName => Command::CopyPageName,
            Reload | ForceReload => Command::Reload,
            ToggleSidebar => Command::ToggleSidebar,
            ZoomIn => Command::ZoomIn,
            ZoomOut => Command::ZoomOut,
            ZoomReset => Command::ZoomReset,
            ToggleFullscreen => Command::ToggleFullscreen,
            GoBack => Command::GoBack,
            GoForward => Command::GoForward,
            Minimize => Command::WindowMinimize,
            PreviousTab => Command::PreviousTab,
            NextTab => Command::NextTab,
            OpenShortcuts => Command::OpenShortcuts,
            QuickSearch => Command::ToggleSearch,
            CommandSearch | FindInPage | MoveTo => Command::OpenSearch,
            ToggleTheme => Command::ToggleTheme,
            AskAi => Command::OpenAiChat,
            Duplicate => Command::DuplicatePage,
            GoUpLevel => Command::GoUpLevel,
            OpenSettings => Command::OpenSettings,
            OpenInNewTab | Confirm => Command::ActivateSelectedNewTab,
            CloseOverlay => Command::CloseAll,
            NextChoice => Command::MoveNewTabSelection(1),
            PreviousChoice => Command::MoveNewTabSelection(-1),
        }
    }

    fn page_locked(&self) -> bool {
        self.workspace
            .page(&self.current_page)
            .map(|p| p.locked)
            .unwrap_or(false)
    }

    #[allow(clippy::too_many_lines)]
    fn apply_commands(
        &mut self,
        mut commands: Vec<Command>,
        ctx: &egui::Context,
        frame_before: Workspace,
    ) {
        let mut had_undo_redo = false;
        let mut index = 0;
        while index < commands.len() {
            let command = commands[index].clone();
            let locked = self.page_locked();
            let blocked = locked
                && matches!(
                    command,
                    Command::DeletePage(_)
                        | Command::ToggleFavorite(_)
                        | Command::SetTableView(_)
                        | Command::OpenSlash { .. }
                        | Command::SetBlockKind(_, _)
                        | Command::ApplyMarkdown { .. }
                        | Command::InsertBlockAfter(_)
                        | Command::AppendBlock
                        | Command::AddBlock { .. }
                        | Command::StartBlockDrag(_)
                        | Command::HoverBlockDrag(_)
                        | Command::DropBlock
                        | Command::DeleteBlock(_)
                        | Command::ToggleTodo(_)
                        | Command::CycleStatus(_)
                        | Command::AddRow
                        | Command::SetPageIcon(_)
                        | Command::DuplicatePage
                        | Command::RestoreVersion(_)
                );
            if blocked {
                index += 1;
                continue;
            }
            match command {
                Command::SelectPage(id) => {
                    self.current_page = id.clone();
                    if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                        *tab = id.clone();
                    }
                    self.library_open = false;
                    self.close_overlays();
                    self.push_history(id);
                }
                Command::NewPage(parent) => {
                    let draft = PageDraft {
                        title: "Untitled".to_string(),
                        icon: "P".to_string(),
                        parent,
                    };
                    if let Ok(page) = self.client.create_page(&draft) {
                        self.current_page = page.id.clone();
                        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                            *tab = page.id.clone();
                        }
                        self.library_open = false;
                        self.close_overlays();
                        self.push_history(page.id);
                        self.pending_reload = true;
                    }
                }
                Command::DeletePage(id) => {
                    let _ = self.client.delete_page(&id);
                    self.pending_reload = true;
                }
                Command::ToggleExpand(id) => {
                    if !self.expanded.insert(id.clone()) {
                        self.expanded.remove(&id);
                    }
                }
                Command::ToggleFavorite(id) => {
                    if let Some(page) = self.workspace.page_mut(&id) {
                        page.favorite = !page.favorite;
                    }
                }
                Command::OpenSearch => {
                    self.close_overlays();
                    self.search_open = true;
                }
                Command::ToggleSearch => {
                    if self.search_open {
                        self.search_open = false;
                        self.search_query.clear();
                    } else {
                        self.close_overlays();
                        self.search_open = true;
                    }
                }
                Command::CloseSearch => {
                    self.search_open = false;
                    self.search_query.clear();
                }
                Command::SetSearchQuery(query) => self.search_query = query,
                Command::OpenSettings => {
                    self.close_overlays();
                    self.settings_open = true;
                }
                Command::CloseSettings => self.settings_open = false,
                Command::ToggleTheme => {
                    self.dark = !self.dark;
                    self.theme_dirty = true;
                    apply_theme(ctx, self.dark);
                }
                Command::ToggleSidebar => self.sidebar_open = !self.sidebar_open,
                Command::SetTableView(table) => self.table_view = table,
                Command::OpenLibrary => {
                    self.close_overlays();
                    self.library_open = true;
                }
                Command::CloseLibrary => self.library_open = false,
                Command::SetLibraryTab(tab) => self.library_tab = tab,
                Command::OpenShare => {
                    self.close_overlays();
                    self.share_open = true;
                }
                Command::CloseShare => self.share_open = false,
                Command::OpenInbox => {
                    self.close_overlays();
                    self.inbox_open = true;
                }
                Command::CloseInbox => self.inbox_open = false,
                Command::GoBack => {
                    if self.nav_pos > 0 {
                        self.nav_pos -= 1;
                        self.current_page = self.nav_history[self.nav_pos].clone();
                        self.library_open = false;
                        self.close_overlays();
                    }
                }
                Command::GoForward => {
                    if self.nav_pos + 1 < self.nav_history.len() {
                        self.nav_pos += 1;
                        self.current_page = self.nav_history[self.nav_pos].clone();
                        self.library_open = false;
                        self.close_overlays();
                    }
                }
                Command::GoUpLevel => {
                    let parent = self
                        .workspace
                        .page(&self.current_page)
                        .and_then(|p| p.parent.clone());
                    if let Some(parent) = parent {
                        commands.push(Command::SelectPage(parent));
                    }
                }
                Command::WindowMinimize => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                }
                Command::WindowMaximize => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
                }
                Command::WindowClose => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                Command::ToggleFullscreen => {
                    self.fullscreen = !self.fullscreen;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.fullscreen));
                }
                Command::OpenSlash { block, pos } => {
                    if let Some(b) = self.find_block_mut(&block) {
                        b.text = b.text.trim_start_matches('/').trim_start().to_string();
                    }
                    self.slash_for = Some(block);
                    self.slash_pos = pos;
                }
                Command::CloseSlash => self.slash_for = None,
                Command::SetBlockKind(block, kind) => {
                    if let Some(b) = self.find_block_mut(&block) {
                        b.set_kind(kind);
                        if kind.is_divider() {
                            b.text.clear();
                        } else {
                            b.text = b.text.trim_start_matches('/').trim_start().to_string();
                        }
                    }
                    self.slash_for = None;
                }
                Command::ApplyMarkdown { block, kind, prefix } => {
                    if let Some(b) = self.find_block_mut(&block) {
                        if let Some(rest) = b.text.strip_prefix(&prefix) {
                            b.text = rest.to_string();
                        }
                        b.set_kind(kind);
                        if kind.is_divider() {
                            b.text.clear();
                        }
                    }
                }
                Command::InsertBlockAfter(block_id) => {
                    if let Some(page) = self.current_page_mut()
                        && let Some(index) = page.blocks.iter().position(|b| b.id == block_id)
                    {
                        page.blocks.insert(
                            index + 1,
                            Block::new(Uuid::new_v4().to_string(), BlockKind::Paragraph, ""),
                        );
                    }
                }
                Command::AppendBlock => {
                    if let Some(page) = self.current_page_mut() {
                        page.blocks.push(Block::new(
                            Uuid::new_v4().to_string(),
                            BlockKind::Paragraph,
                            "",
                        ));
                    }
                }
                Command::AddBlock { block, above, pos } => {
                    // The "+" beside a block adds a new block below (or above
                    // with Alt) and immediately opens the block menu for it.
                    let new_id = Uuid::new_v4().to_string();
                    if let Some(page) = self.current_page_mut()
                        && let Some(index) = page.blocks.iter().position(|b| b.id == block)
                    {
                        let at = if above { index } else { index + 1 };
                        page.blocks.insert(
                            at,
                            Block::new(new_id.clone(), BlockKind::Paragraph, ""),
                        );
                        self.slash_for = Some(new_id);
                        self.slash_pos = pos;
                    }
                }
                Command::StartBlockDrag(id) => {
                    self.drag_from = Some(id);
                    self.drag_to = None;
                }
                Command::HoverBlockDrag(id) => {
                    // Only track a target while a drag is actually in progress;
                    // this also prevents a late hover command (pushed later in
                    // the same frame as the drop) from leaving stale state.
                    if self.drag_from.is_some()
                        && self.drag_from.as_deref() != Some(id.as_str())
                    {
                        self.drag_to = Some(id);
                    }
                }
                Command::DropBlock => {
                    let from = self.drag_from.take();
                    let to = self.drag_to.take();
                    if let (Some(from), Some(to)) = (from, to)
                        && from != to
                        && let Some(page) = self.current_page_mut()
                        && let Some(from_idx) = page.blocks.iter().position(|b| b.id == from)
                    {
                        let block = page.blocks.remove(from_idx);
                        let target = page
                            .blocks
                            .iter()
                            .position(|b| b.id == to)
                            .map_or(page.blocks.len(), |i| i + 1);
                        page.blocks.insert(target.min(page.blocks.len()), block);
                    }
                }
                Command::DeleteBlock(block_id) => {
                    if let Some(page) = self.current_page_mut() {
                        page.blocks.retain(|b| b.id != block_id);
                    }
                }
                Command::ToggleTodo(block_id) => {
                    if let Some(b) = self.find_block_mut(&block_id) {
                        b.toggle_todo();
                    }
                }
                Command::CycleStatus(row_id) => {
                    for page in &mut self.workspace.pages {
                        for row in &mut page.rows {
                            if row.id == row_id {
                                row.status = match row.status.as_str() {
                                    "Not started" => "In progress",
                                    "In progress" => "Done",
                                    _ => "Not started",
                                }
                                .to_string();
                            }
                        }
                    }
                }
                Command::AddRow => {
                    if let Some(page) = self.current_page_mut() {
                        page.rows.push(TableRow::new(
                            Uuid::new_v4().to_string(),
                            "New task",
                            "Not started",
                            "",
                            "",
                        ));
                    }
                }
                Command::ResetWorkspace => {
                    let _ = self.client.replace_workspace(&Workspace::seed("not-notion"));
                    self.pending_reload = true;
                }
                // ---- Tabs / "+" switcher ----------------------------------
                Command::OpenNewTab => {
                    self.close_overlays();
                    self.new_tab_open = true;
                    self.new_tab_query.clear();
                    self.new_tab_selected = 0;
                }
                Command::CloseNewTab => self.new_tab_open = false,
                Command::SetNewTabQuery(query) => {
                    self.new_tab_query = query;
                    self.new_tab_selected = 0;
                }
                Command::SetNewTabSelected(index) => self.new_tab_selected = index,
                Command::MoveNewTabSelection(delta) => {
                    let count = new_tab_choices(&self.workspace, &self.new_tab_query).len();
                    if count > 0 {
                        let current = self.new_tab_selected as i32 + delta;
                        self.new_tab_selected = current.rem_euclid(count as i32) as usize;
                    }
                }
                Command::ActivateSelectedNewTab => {
                    if self.new_tab_open {
                        let choices = new_tab_choices(&self.workspace, &self.new_tab_query);
                        if let Some(choice) = choices.get(self.new_tab_selected).cloned() {
                            commands.push(Command::ActivateNewTab(choice));
                        }
                    } else if self.search_open
                        && let Some(result) = self.search_results.get(self.search_selected)
                    {
                        let id = result.page_id.clone();
                        self.open_in_tab(id);
                        self.close_overlays();
                    }
                }
                Command::ActivateNewTab(choice) => {
                    match choice {
                        NewTabChoice::NewChat => {
                            self.close_overlays();
                            self.search_open = true;
                            self.ai_query.clear();
                            self.ai_answer = None;
                            self.ai_error = None;
                        }
                        NewTabChoice::NewPage => {
                            let draft = PageDraft {
                                title: "Untitled".to_string(),
                                icon: "P".to_string(),
                                parent: None,
                            };
                            if let Ok(page) = self.client.create_page(&draft) {
                                self.open_in_tab(page.id);
                                self.pending_reload = true;
                            }
                            self.close_overlays();
                        }
                        NewTabChoice::Page(id) => {
                            self.open_in_tab(id);
                            self.close_overlays();
                        }
                    }
                }
                Command::SelectTab(index) => {
                    if let Some(id) = self.tabs.get(index).cloned() {
                        self.active_tab = index;
                        self.current_page = id;
                        self.library_open = false;
                        self.close_overlays();
                    }
                }
                Command::CloseTab(index) => {
                    let index = if index == usize::MAX { self.active_tab } else { index };
                    if self.tabs.len() > 1 && index < self.tabs.len() {
                        let id = self.tabs.remove(index);
                        self.closed_tabs.push(id);
                        if self.active_tab >= self.tabs.len() {
                            self.active_tab = self.tabs.len() - 1;
                        }
                        self.current_page = self.tabs[self.active_tab].clone();
                    } else if self.tabs.len() <= 1 {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
                Command::PreviousTab => {
                    if !self.tabs.is_empty() {
                        self.active_tab =
                            (self.active_tab + self.tabs.len() - 1) % self.tabs.len();
                        self.current_page = self.tabs[self.active_tab].clone();
                    }
                }
                Command::NextTab => {
                    if !self.tabs.is_empty() {
                        self.active_tab = (self.active_tab + 1) % self.tabs.len();
                        self.current_page = self.tabs[self.active_tab].clone();
                    }
                }
                Command::ReopenClosedTab => {
                    if let Some(id) = self.closed_tabs.pop()
                        && self.workspace.page(&id).is_some()
                    {
                        self.open_in_tab(id);
                        self.close_overlays();
                    }
                }
                // ---- AI assistant ------------------------------------------
                Command::OpenAiChat => {
                    self.close_overlays();
                    self.search_open = true;
                    self.ai_answer = None;
                    self.ai_error = None;
                    if !self.ai_query.trim().is_empty() {
                        commands.push(Command::AskAi(self.ai_query.clone()));
                    }
                }
                Command::SetAiQuery(query) => self.ai_query = query,
                Command::AskAi(prompt) => {
                    if !self.ai_busy {
                        let config = self.ai_config.clone();
                        let slot = Arc::clone(&self.ai_slot);
                        self.ai_busy = true;
                        self.ai_answer = None;
                        self.ai_error = None;
                        std::thread::spawn(move || {
                            let result = ollama::generate(&config, &prompt);
                            if let Ok(mut guard) = slot.lock() {
                                *guard = Some(result);
                            }
                        });
                    }
                }
                Command::ClearAi => {
                    self.ai_query.clear();
                    self.ai_answer = None;
                    self.ai_error = None;
                }
                Command::SetAiEndpoint(endpoint) => self.ai_config.endpoint = endpoint,
                Command::SetAiModel(model) => self.ai_config.model = model,
                Command::ToggleAiCli => self.ai_config.prefer_cli = !self.ai_config.prefer_cli,
                Command::RefreshAiStatus => {
                    self.ai_status = ollama::detect(&self.ai_config);
                }
                // ---- Settings / help / window ------------------------------
                Command::SetSettingsPage(page) => self.settings_page = page,
                Command::SetSettingsQuery(query) => self.settings_query = query,
                Command::SetLanguage(locale) => {
                    crate::i18n::set_locale(&locale);
                    self.language = locale;
                },
                Command::OpenShortcuts => {
                    self.close_overlays();
                    self.shortcuts_open = true;
                }
                Command::CloseShortcuts => self.shortcuts_open = false,
                Command::ZoomIn => {
                    self.zoom = (self.zoom + 0.1).min(1.6);
                    self.zoom_dirty = true;
                }
                Command::ZoomOut => {
                    self.zoom = (self.zoom - 0.1).max(0.7);
                    self.zoom_dirty = true;
                }
                Command::ZoomReset => {
                    self.zoom = 1.0;
                    self.zoom_dirty = true;
                }
                // ---- Editing history / misc --------------------------------
                Command::Undo => {
                    if let Some(previous) = self.undo_stack.pop() {
                        self.redo_stack.push(self.workspace.clone());
                        self.workspace = previous;
                    }
                    had_undo_redo = true;
                }
                Command::Redo => {
                    if let Some(next) = self.redo_stack.pop() {
                        self.undo_stack.push(self.workspace.clone());
                        self.workspace = next;
                    }
                    had_undo_redo = true;
                }
                Command::DuplicatePage => {
                    if let Some(page) = self.workspace.page(&self.current_page).cloned() {
                        let mut copy = page;
                        copy.id = Uuid::new_v4().to_string();
                        copy.title = format!("{} (copy)", copy.title);
                        copy.favorite = false;
                        for (i, block) in copy.blocks.iter_mut().enumerate() {
                            block.id = format!("{}-dup-{i}-{}", copy.id, Uuid::new_v4());
                        }
                        for row in &mut copy.rows {
                            row.id = Uuid::new_v4().to_string();
                        }
                        let new_id = copy.id.clone();
                        self.workspace.pages.push(copy);
                        self.open_in_tab(new_id);
                    }
                }
                Command::Reload => self.pending_reload = true,
                Command::CopyLink => {
                    ctx.copy_text(format!("not-notion://page/{}", self.current_page));
                }
                Command::CopyPageName => {
                    let name = self
                        .workspace
                        .page(&self.current_page)
                        .map(|p| p.title.clone())
                        .unwrap_or_else(|| "Untitled".to_string());
                    ctx.copy_text(name);
                }
                Command::CloseAll => {
                    self.close_overlays();
                    self.library_open = false;
                }
                // ---- Page icon customization (spec-013) ---------------------
                Command::OpenIconPicker => {
                    self.close_overlays();
                    self.icon_picker_open = true;
                    self.icon_picker_query.clear();
                    self.icon_picker_tab = IconTab::Emoji;
                }
                Command::CloseIconPicker => self.icon_picker_open = false,
                Command::SetIconTab(tab) => self.icon_picker_tab = tab,
                Command::SetIconQuery(query) => self.icon_picker_query = query,
                Command::SetPageIcon(icon) => {
                    if let Some(page) = self.current_page_mut() {
                        page.icon = icon;
                    }
                    self.icon_picker_open = false;
                    self.icon_picker_query.clear();
                }
                // ---- Page actions / appearance (spec-013) -------------------
                Command::OpenPageActions => {
                    self.close_overlays();
                    self.page_actions_open = true;
                }
                Command::ClosePageActions => self.page_actions_open = false,
                Command::OpenVersionHistory => {
                    self.close_overlays();
                    self.versions =
                        Arc::new(self.client.page_versions(&self.current_page).unwrap_or_default());
                    self.version_selected = 0;
                    self.version_history_open = true;
                }
                Command::CloseVersionHistory => self.version_history_open = false,
                Command::SetVersionSelected(index) => self.version_selected = index,
                Command::RestoreVersion(index) => {
                    if let Some(version) = self.versions.get(index).cloned()
                        && let Ok(mut page) = serde_json::from_str::<Page>(&version.snapshot)
                    {
                        page.id = self.current_page.clone();
                        if self.client.update_page(&page).is_ok() {
                            self.pending_reload = true;
                        }
                    }
                    self.version_history_open = false;
                }
                Command::ToggleWorkspaceMenu => {
                    let open = !self.workspace_menu_open;
                    self.close_overlays();
                    self.workspace_menu_open = open;
                }
                Command::CloseWorkspaceMenu => self.workspace_menu_open = false,
                Command::OpenAppMenu => {
                    let open = !self.app_menu_open;
                    self.close_overlays();
                    self.app_menu_open = open;
                }
                Command::CloseAppMenu => self.app_menu_open = false,
                Command::ToggleGridMenu => {
                    let open = !self.grid_menu_open;
                    self.close_overlays();
                    self.grid_menu_open = open;
                }
                Command::CloseGridMenu => self.grid_menu_open = false,
                Command::ToggleFullWidth => self.full_width = !self.full_width,
                Command::ToggleSmallText => self.small_text = !self.small_text,
                Command::SetFontStyle(style) => self.font_style = style,
                Command::ToggleAvailableOffline => {
                    if let Some(page) = self.current_page_mut() {
                        page.available_offline = !page.available_offline;
                    }
                }
                Command::ToggleLockPage => {
                    if let Some(page) = self.current_page_mut() {
                        page.locked = !page.locked;
                    }
                }
                // ---- Search / new-tab panes (spec-013) ----------------------
                Command::SetSearchSelected(index) => self.search_selected = index,
                Command::ToggleHighlightPane => self.highlight_pane = !self.highlight_pane,
                Command::ToggleFilters => self.filters = !self.filters,
                Command::NoOp => {}
            }
            index += 1;
        }

        if !had_undo_redo && self.workspace != frame_before {
            self.undo_stack.push(frame_before);
            if self.undo_stack.len() > 60 {
                self.undo_stack.remove(0);
            }
            self.redo_stack.clear();
        }

        if self.pending_reload {
            if let Ok(workspace) = self.client.workspace() {
                self.workspace = workspace;
                self.last_committed = self.workspace.clone();
                if self.workspace.page(&self.current_page).is_none() {
                    self.current_page = self
                        .workspace
                        .pages
                        .first()
                        .map(|p| p.id.clone())
                        .unwrap_or_default();
                }
            }
            self.pending_reload = false;
        }

        if self.workspace != self.last_committed {
            let _ = self.client.replace_workspace(&self.workspace);
            self.last_committed = self.workspace.clone();
        }
    }

    fn push_history(&mut self, id: String) {
        if self.nav_history.get(self.nav_pos) == Some(&id) {
            return;
        }
        self.nav_history.truncate(self.nav_pos + 1);
        self.nav_history.push(id);
        self.nav_pos = self.nav_history.len() - 1;
    }
}

/// Translate an `egui::Key` into the small names used by `shortcuts::lookup`.
fn key_name(key: egui::Key) -> &'static str {
    use egui::Key::*;
    match key {
        A => "a", B => "b", C => "c", D => "d", E => "e", F => "f", G => "g",
        H => "h", I => "i", J => "j", K => "k", L => "l", M => "m", N => "n",
        O => "o", P => "p", Q => "q", R => "r", S => "s", T => "t", U => "u",
        V => "v", W => "w", X => "x", Y => "y", Z => "z",
        Num0 => "0", Num1 => "1", Num2 => "2", Num3 => "3", Num4 => "4",
        Num5 => "5", Num6 => "6", Num7 => "7", Num8 => "8", Num9 => "9",
        Plus => "+", Minus => "-", Equals => "=", Backslash => "\\", Slash => "/",
        Comma => ",", Period => ".", Colon => ":", Semicolon => ";", Quote => "'",
        Backtick => "`", Pipe => "|", Questionmark => "?", Exclamationmark => "!",
        OpenBracket => "[", CloseBracket => "]", OpenCurlyBracket => "{",
        CloseCurlyBracket => "}",
        Tab => "tab", Escape => "escape", Enter => "enter", Space => "space",
        ArrowUp => "arrowup", ArrowDown => "arrowdown", ArrowLeft => "arrowleft",
        ArrowRight => "arrowright",
        F1 => "f1", F11 => "f11",
        _ => "",
    }
}

/// Read the pressed keys, resolve exact shortcuts, and return the events to
/// consume so they do not also reach a focused `TextEdit`.
fn collect_shortcuts(
    ctx: &egui::Context,
    popup_open: bool,
) -> (Vec<Action>, Vec<(egui::Key, egui::Modifiers)>) {
    // Read the frame's trusted input context together with the events. A key
    // is only allowed to become a shortcut when the pointer is over the
    // window, the window owns the keyboard focus, and the event's modifiers
    // agree with the held modifiers (spec-013). This stops key events that
    // arrive after the pointer leaves the window — or with a stale Ctrl/Alt
    // left over from a focus change — from hijacking ordinary typing.
    let (events, context) = ctx.input(|i| {
        let events: Vec<(egui::Key, egui::Modifiers)> = i
            .events
            .iter()
            .filter_map(|event| match event {
                egui::Event::Key {
                    key,
                    pressed: true,
                    repeat: false,
                    modifiers,
                    ..
                } => Some((*key, *modifiers)),
                _ => None,
            })
            .collect();
        (
            events,
            (
                i.focused,
                i.pointer.has_pointer(),
                (i.modifiers.ctrl, i.modifiers.shift, i.modifiers.alt),
            ),
        )
    });
    let (window_focused, pointer_over_window, frame_mods) = context;
    let mut actions = Vec::new();
    let mut consumed = Vec::new();
    for (key, modifiers) in events {
        let event_mods = (modifiers.ctrl, modifiers.shift, modifiers.alt);
        if !shortcuts::shortcut_context_ok(
            window_focused,
            pointer_over_window,
            event_mods,
            frame_mods,
        ) {
            continue;
        }
        let name = key_name(key);
        // Enter / arrow keys are navigation only while a popover is open;
        // otherwise they must remain available to text editors.
        let popup_only = matches!(name, "enter" | "arrowup" | "arrowdown")
            && !modifiers.ctrl
            && !modifiers.shift
            && !modifiers.alt;
        if popup_only && !popup_open {
            continue;
        }
        let combo = Combo::new(modifiers.ctrl, modifiers.shift, modifiers.alt, name);
        if let Some(action) = shortcuts::lookup(combo) {
            actions.push(action);
            consumed.push((key, modifiers));
        }
    }
    (actions, consumed)
}

impl eframe::App for NotionApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if self.theme_dirty {
            apply_theme(&ctx, self.dark);
            self.theme_dirty = false;
        }
        if self.zoom_dirty {
            ctx.set_zoom_factor(self.zoom);
            self.zoom_dirty = false;
        }
        self.poll_ai();

        // Snapshot before rendering: text edits mutate `workspace` directly, so
        // this is the state an Undo would restore.
        let frame_before = self.workspace.clone();

        let popup_open = self.new_tab_open
            || self.search_open
            || self.icon_picker_open
            || self.page_actions_open
            || self.version_history_open
            || self.shortcuts_open
            || self.slash_for.is_some();
        let (actions, consumed) = collect_shortcuts(&ctx, popup_open);
        if !consumed.is_empty() {
            ctx.input_mut(|i| {
                i.events.retain(|event| match event {
                    egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => !consumed
                        .iter()
                        .any(|(k, m)| k == key && m == modifiers),
                    _ => true,
                });
            });
        }
        let mut commands: Vec<Command> =
            actions.iter().map(|a| self.command_for_action(*a)).collect();

        // Search results are fetched through the client before rendering, but
        // only when the query or the workspace version changed (spec-015).
        // An unchanged frame is a cache hit and never calls the server.
        if self
            .search_cache
            .needs_refresh(&self.search_query, self.workspace.version)
        {
            let results = if self.search_query.trim().is_empty() {
                self.workspace
                    .pages
                    .iter()
                    .take(6)
                    .map(|p| SearchResult {
                        page_id: p.id.clone(),
                        page_title: p.title.clone(),
                        snippet: p.plain_text(),
                        score: 0,
                    })
                    .collect()
            } else {
                self.client.search(&self.search_query).unwrap_or_default()
            };
            self.search_results = Arc::new(results);
        }

        let palette = Palette::for_theme(self.dark);
        let view = self.view_state();

        // Full-width custom window chrome (tabs + window controls). The OS
        // decorations are off, so the whole strip is a drag handle.
        egui::Panel::top("title_bar")
            .exact_size(TITLE_BAR_HEIGHT)
            .resizable(false)
            .show_separator_line(false)
            .frame(egui::Frame::NONE.fill(palette.chrome))
            .show(ui, |ui| {
                let drag_rect = ui.available_rect_before_wrap();
                let drag = ui.interact(
                    drag_rect,
                    egui::Id::unique("custom_title_bar_drag"),
                    egui::Sense::drag(),
                );
                if drag.drag_started() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                render_title_bar(ui, &self.workspace, &view, &palette, &mut commands);
            });

        if view.sidebar_open {
            egui::Panel::left("sidebar")
                .exact_size(266.0)
                .resizable(false)
                .show_separator_line(false)
                .frame(
                    egui::Frame::NONE
                        .fill(palette.sidebar)
                        .inner_margin(egui::Margin::symmetric(10, 6)),
                )
                .show(ui, |ui| {
                    render_sidebar(ui, &self.workspace, &view, &palette, &mut commands);
                });
        }

        {
            let workspace_mut = &mut self.workspace;
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.fill(palette.content))
                .show(ui, |ui| {
                    render_content(ui, workspace_mut, &view, &palette, &mut commands);
                });
        }

        if view.search_open {
            render_search(&ctx, &self.workspace, &view, &palette, &mut commands);
        }
        if view.settings_open {
            render_settings(&ctx, &self.workspace, &view, &palette, &mut commands);
        }
        if view.share_open {
            render_share(&ctx, &view, &palette, &mut commands);
        }
        if view.inbox_open {
            render_inbox(&ctx, &view, &palette, &mut commands);
        }
        if view.new_tab_open {
            render_new_tab(&ctx, &self.workspace, &view, &palette, &mut commands);
        }
        if view.shortcuts_open {
            render_shortcuts(&ctx, &view, &palette, &mut commands);
        }
        if let Some(block) = view.slash_for.as_ref() {
            render_slash(&ctx, block, &view, &palette, &mut commands);
        }
        if view.icon_picker_open {
            render_icon_picker(&ctx, &view, &palette, &mut commands);
        }
        if view.page_actions_open {
            render_page_actions(&ctx, &self.workspace, &view, &palette, &mut commands);
        }
        if view.version_history_open {
            render_version_history(&ctx, &view, &palette, &mut commands);
        }
        if view.app_menu_open {
            render_app_menu(&ctx, &palette, &mut commands);
        }
        if view.workspace_menu_open {
            render_workspace_menu(&ctx, &palette, &mut commands);
        }
        if view.grid_menu_open {
            render_grid_menu(&ctx, &palette, &mut commands);
        }

        self.apply_commands(commands, &ctx, frame_before);

        // --screenshot: draw ~120 frames, capture the window, save a PNG, exit.
        self.frames += 1;
        if let Some(path) = self.screenshot.clone() {
            ctx.request_repaint();
            if !self.screenshot_requested && self.frames >= 110 {
                self.screenshot_requested = true;
                let done = Arc::clone(&self.screenshot_done);
                let target = path.clone();
                ctx.request_screenshot(move |image| {
                    let _ = screenshot::write_png(&target, &image);
                    done.store(true, Ordering::SeqCst);
                });
            }
            if (self.screenshot_done.load(Ordering::SeqCst) && self.frames >= 120)
                || self.frames >= 360
            {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
}
