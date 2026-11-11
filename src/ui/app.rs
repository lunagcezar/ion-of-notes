//! Application state, command handling, and the eframe::App implementation.
//! This module owns the state machine; drawing lives in `render` and `dialogs`.
//!
//! Keyboard input is translated by the pure `shortcuts` table into `Action`s
//! and then into `Command`s, so every shortcut follows the same client-bound
//! path as a mouse click. The AI assistant runs through the `ollama` adapter
//! on a background thread; the window only polls a shared result slot.

use crate::app;
use crate::client::Client;
use crate::collab::{Access, Member, PageVersion, Role, general_access};
use crate::model::Comment;
use crate::model::{SearchResult, Workspace};
use crate::ollama::{self, OllamaConfig, OllamaStatus};
use crate::screenshot;
use crate::server::realtime::{Awareness, ServerMessage};
use egui::Pos2;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use super::crdt::CrdtSession;
use super::dialogs::{
    render_app_menu, render_grid_menu, render_inbox, render_new_tab, render_onboarding,
    render_page_actions, render_search, render_settings, render_share, render_shortcuts,
    render_sidebar_menu, render_slash, render_templates, render_version_history,
    render_workspace_menu,
};
use super::edit_queue::PendingEdits;
use super::page_icon::render_icon_picker;
use super::palette::{Palette, apply_theme};
use super::render::{TITLE_BAR_HEIGHT, render_content, render_sidebar, render_title_bar};
use super::search_cache::SearchCache;

pub(super) use super::commands::{Command, FontStyle, IconTab, NewTabChoice, SidebarMenu};
use super::input::collect_shortcuts;
pub(super) use super::view::ViewState;

pub(super) type ServerConnectResult = Result<(Client<app::DesktopApi>, Workspace, String), String>;
pub(super) type ServerSlot = Arc<Mutex<Option<ServerConnectResult>>>;

pub struct NotionApp {
    pub(super) client: Client<app::DesktopApi>,
    pub(super) workspace: Workspace,
    pub(super) last_committed: Workspace,
    pub(super) pending_edits: PendingEdits,
    pub(super) current_page: String,
    pub(super) tabs: Vec<String>,
    pub(super) active_tab: usize,
    pub(super) closed_tabs: Vec<String>,
    pub(super) sidebar_open: bool,
    pub(super) expanded: HashSet<String>,
    pub(super) search_open: bool,
    pub(super) search_query: String,
    pub(super) search_results: Arc<Vec<SearchResult>>,
    pub(super) search_cache: SearchCache,
    pub(super) settings_open: bool,
    pub(super) onboarding_open: bool,
    pub(super) templates_open: bool,
    pub(super) settings_page: String,
    pub(super) settings_query: String,
    pub(super) settings_status: Option<String>,
    pub(super) language: String,
    pub(super) dark: bool,
    pub(super) theme_dirty: bool,
    pub(super) table_view: bool,
    pub(super) library_open: bool,
    pub(super) library_tab: String,
    pub(super) library_shared: Arc<HashSet<String>>,
    pub(super) library_owner: String,
    pub(super) share_open: bool,
    pub(super) share_members: Arc<Vec<Member>>,
    pub(super) invite_input: String,
    pub(super) invite_role: Role,
    pub(super) invite_message: String,
    pub(super) invite_error: Option<String>,
    pub(super) invite_status: Option<String>,
    pub(super) share_general_access: Access,
    pub(super) inbox_open: bool,
    pub(super) nav_history: Vec<String>,
    pub(super) nav_pos: usize,
    pub(super) slash_for: Option<String>,
    pub(super) slash_pos: Pos2,
    pub(super) slash_query: String,
    pub(super) slash_dismissed: Option<String>,
    pub(super) sidebar_customize: bool,
    pub(super) sidebar_menu: Option<SidebarMenu>,
    pub(super) pending_reload: bool,
    pub(super) screenshot: Option<PathBuf>,
    pub(super) frames: u32,
    pub(super) screenshot_requested: bool,
    pub(super) screenshot_done: Arc<AtomicBool>,
    // "Open in new tab…" switcher.
    pub(super) new_tab_open: bool,
    pub(super) new_tab_query: String,
    pub(super) new_tab_selected: usize,
    // AI assistant.
    pub(super) ai_config: OllamaConfig,
    pub(super) ai_status: OllamaStatus,
    pub(super) ai_checking: bool,
    pub(super) ai_status_slot: Arc<Mutex<Option<OllamaStatus>>>,
    pub(super) ai_query: String,
    pub(super) ai_answer: Option<String>,
    pub(super) ai_error: Option<String>,
    pub(super) ai_busy: bool,
    pub(super) ai_started: Option<std::time::Instant>,
    pub(super) ai_slot: Arc<Mutex<Option<Result<String, String>>>>,
    // Help / window.
    pub(super) shortcuts_open: bool,
    pub(super) zoom: f32,
    pub(super) zoom_dirty: bool,
    pub(super) fullscreen: bool,
    pub(super) undo_stack: Vec<Workspace>,
    pub(super) redo_stack: Vec<Workspace>,
    pub(super) search_selected: usize,
    pub(super) highlight_pane: bool,
    pub(super) filters: bool,
    pub(super) icon_picker_open: bool,
    pub(super) icon_picker_tab: IconTab,
    pub(super) icon_picker_query: String,
    pub(super) page_actions_open: bool,
    pub(super) page_status: Option<String>,
    pub(super) comment_input: String,
    pub(super) version_history_open: bool,
    pub(super) versions: Arc<Vec<PageVersion>>,
    pub(super) version_selected: usize,
    pub(super) font_style: FontStyle,
    pub(super) full_width: bool,
    pub(super) small_text: bool,
    pub(super) workspace_menu_open: bool,
    pub(super) app_menu_open: bool,
    pub(super) grid_menu_open: bool,
    pub(super) drag_from: Option<String>,
    pub(super) drag_to: Option<String>,
    pub(super) page_selection: Vec<String>,
    pub(super) block_selecting: bool,
    // Real-time CRDT and awareness.
    pub(super) crdt: CrdtSession,
    pub(super) crdt_page: String,
    pub(super) crdt_rx: Option<tokio::sync::broadcast::Receiver<ServerMessage>>,
    pub(super) remote_awareness: Vec<Awareness>,
    pub(super) awareness_session: String,
    pub(super) awareness_color: String,
    pub(super) last_awareness: Option<(String, usize, Option<usize>)>,
    // Server login/settings.
    pub(super) server_url: String,
    pub(super) server_email: String,
    pub(super) server_password: String,
    pub(super) server_status: Option<String>,
    pub(super) server_connected: bool,
    pub(super) server_busy: bool,
    pub(super) server_slot: ServerSlot,
}

impl NotionApp {
    pub fn new(cc: &eframe::CreationContext<'_>, screenshot: Option<PathBuf>) -> Self {
        let (client, workspace) = app::build_desktop();
        Self::with_client(cc, screenshot, client, workspace)
    }

    /// Open the Server settings page with a prefilled URL (startup prompt).
    pub fn open_server_login(&mut self, server: String) {
        self.server_url = server;
        self.settings_open = true;
        self.settings_page = "Server".to_string();
        self.server_busy = false;
        self.server_status = Some("Enter your account and password.".to_string());
    }

    /// Mark the current transport as a connected remote server.
    pub fn mark_server_connected(&mut self, server: String) {
        self.server_url = server.clone();
        self.server_connected = true;
        self.server_busy = false;
        self.server_status = Some(format!("Connected to {server}"));
    }

    pub fn with_client(
        cc: &eframe::CreationContext<'_>,
        screenshot: Option<PathBuf>,
        mut client: Client<app::DesktopApi>,
        mut workspace: Workspace,
    ) -> Self {
        let initial_locale =
            std::env::var("ION_OF_NOTES_LOCALE").unwrap_or_else(|_| "en".to_string());
        crate::i18n::set_locale(&initial_locale);
        let language = crate::i18n::current_locale();
        let current_page = workspace
            .pages
            .first()
            .map(|p| p.id.clone())
            .unwrap_or_default();
        apply_theme(&cc.egui_ctx, true);
        // A harmless demo hook: with ION_OF_NOTES_SHOW set, one of the opened
        // states from the reference pictures is shown on startup. Normal runs
        // (and the rebuild check) never set it.
        let demo = std::env::var("ION_OF_NOTES_SHOW").unwrap_or_default();
        let demo_current_page = if demo == "tasks" {
            workspace
                .page("p-tasks")
                .map(|p| p.id.clone())
                .unwrap_or_else(|| current_page.clone())
        } else {
            current_page.clone()
        };
        let (share_members, share_general_access, library_shared, library_owner) =
            if demo == "share" || demo == "library" || demo == "settings-people" {
                let members = client.list_members().unwrap_or_default();
                let shares = client
                    .list_shares(Some(&demo_current_page))
                    .unwrap_or_default();
                let all_shares = if demo == "library" {
                    client.list_shares(None).unwrap_or_default()
                } else {
                    shares.clone()
                };
                let shared: HashSet<String> = all_shares
                    .iter()
                    .filter(|share| share.page_id != "*")
                    .map(|share| share.page_id.clone())
                    .collect();
                let owner = members
                    .iter()
                    .find(|member| member.role == Role::Owner)
                    .map(|member| member.name.clone())
                    .unwrap_or_else(|| "Local workspace".to_string());
                (
                    Arc::new(members),
                    general_access(&shares, &demo_current_page),
                    Arc::new(shared),
                    owner,
                )
            } else {
                (
                    Arc::new(Vec::new()),
                    Access::None,
                    Arc::new(HashSet::new()),
                    "Local workspace".to_string(),
                )
            };
        if demo == "discussions"
            && let Some(page) = workspace.page_mut("p-home")
        {
            page.show_toc = true;
            page.discussions = true;
            page.inline_comments = true;
            page.comments.push(Comment::new(
                "demo-comment",
                "Local user",
                "This page looks good.",
                "Nov 08, 2026 10:00",
            ));
        }
        let nav_history = vec![demo_current_page.clone()];
        let slash_block = if demo == "slash" {
            workspace
                .page(&demo_current_page)
                .and_then(|page| page.blocks.first())
                .map(|block| block.id.clone())
        } else {
            None
        };
        let page_selection = if demo == "selection" {
            workspace
                .page(&demo_current_page)
                .map(|page| page.blocks.iter().map(|b| b.id.clone()).collect())
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let ai_config = OllamaConfig::from_env();
        let ai_status = OllamaStatus::Unavailable;
        let ai_status_slot = Arc::new(Mutex::new(None));
        {
            let config = ai_config.clone();
            let slot = Arc::clone(&ai_status_slot);
            std::thread::spawn(move || {
                let status = ollama::detect(&config);
                if let Ok(mut guard) = slot.lock() {
                    *guard = Some(status);
                }
            });
        }
        let onboarding_open = demo == "onboarding"
            || demo == "new-workspace"
            || (workspace.pages.is_empty() && !workspace.settings.onboarding_completed);
        let templates_open = demo == "templates";
        let mut crdt = CrdtSession::new();
        if let Some(page) = workspace.page(&demo_current_page) {
            crdt.open_page(page);
        }
        let crdt_rx = client.subscribe_realtime();
        let awareness_session = format!("ui-{}", uuid::Uuid::new_v4());
        Self {
            client,
            last_committed: workspace.clone(),
            pending_edits: PendingEdits::new(0.25),
            workspace,
            current_page: demo_current_page.clone(),
            crdt,
            crdt_page: demo_current_page.clone(),
            crdt_rx,
            remote_awareness: Vec::new(),
            awareness_session,
            awareness_color: "#4C8BF5".to_string(),
            last_awareness: None,
            server_url: "http://127.0.0.1:8787".to_string(),
            server_email: "owner@workspace.local".to_string(),
            server_password: String::new(),
            server_status: None,
            server_connected: false,
            server_busy: false,
            server_slot: Arc::new(Mutex::new(None)),
            tabs: vec![demo_current_page],
            active_tab: 0,
            closed_tabs: Vec::new(),
            sidebar_open: true,
            expanded: HashSet::new(),
            search_open: demo == "search",
            search_query: String::new(),
            search_results: Arc::new(Vec::new()),
            search_cache: SearchCache::default(),
            settings_open: matches!(
                demo.as_str(),
                "settings"
                    | "ai"
                    | "ai-error"
                    | "settings-general"
                    | "settings-sidebar"
                    | "settings-people"
                    | "settings-offline"
                    | "settings-server"
            ),
            onboarding_open,
            templates_open,
            settings_page: match demo.as_str() {
                "ai" | "ai-error" => "AI assistant".to_string(),
                "settings-general" => "General".to_string(),
                "settings-sidebar" => "Sidebar".to_string(),
                "settings-people" => "People".to_string(),
                "settings-offline" => "Offline".to_string(),
                "settings-server" => "Server".to_string(),
                _ => "Preferences".to_string(),
            },
            settings_query: String::new(),
            settings_status: None,
            language,
            dark: true,
            theme_dirty: false,
            table_view: true,
            library_open: demo == "library",
            library_tab: "Recents".to_string(),
            library_shared,
            library_owner,
            share_open: demo == "share",
            share_members,
            invite_input: String::new(),
            invite_role: Role::Guest,
            invite_message: String::new(),
            invite_error: None,
            invite_status: None,
            share_general_access,
            inbox_open: demo == "inbox",
            nav_history,
            nav_pos: 0,
            slash_for: slash_block,
            slash_pos: Pos2::new(560.0, 400.0),
            slash_query: if demo == "slash" {
                "head".to_string()
            } else {
                String::new()
            },
            slash_dismissed: None,
            sidebar_customize: matches!(demo.as_str(), "customize-sidebar" | "customize-shortcut"),
            sidebar_menu: if demo == "customize-shortcut" {
                Some(SidebarMenu::Shortcut)
            } else {
                None
            },
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
            ai_checking: true,
            ai_status_slot,
            ai_query: String::new(),
            ai_answer: None,
            ai_error: if demo == "ai-error" {
                Some(
                    "Ollama is not available. Install the `ollama` CLI in PATH, or start the \
                     service and set OLLAMA_HOST to a reachable host:port."
                        .to_string(),
                )
            } else {
                None
            },
            ai_busy: false,
            ai_started: None,
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
            icon_picker_open: matches!(
                demo.as_str(),
                "iconpicker" | "iconpicker-icons" | "iconpicker-search"
            ),
            icon_picker_tab: if demo == "iconpicker-icons" {
                IconTab::Icons
            } else {
                IconTab::Emoji
            },
            icon_picker_query: if demo == "iconpicker-search" {
                "rocket".to_string()
            } else {
                String::new()
            },
            page_actions_open: demo == "pageactions",
            page_status: None,
            comment_input: String::new(),
            version_history_open: false,
            versions: Arc::new(Vec::new()),
            version_selected: 0,
            font_style: FontStyle::Default,
            full_width: false,
            small_text: false,
            workspace_menu_open: demo == "workspace-switcher",
            app_menu_open: false,
            grid_menu_open: false,
            drag_from: None,
            drag_to: None,
            page_selection,
            block_selecting: false,
        }
    }
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
        self.poll_server();
        self.sync_crdt();
        // Reset the per-frame "pointer is over a block" flag used by the
        // release-based drag drop (spec-060).
        ctx.data_mut(|data| {
            data.insert_temp(egui::Id::unique("block_drag_over"), false);
        });

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
                    } => !consumed.iter().any(|(k, m)| k == key && m == modifiers),
                    _ => true,
                });
            });
        }
        let mut commands: Vec<Command> = actions
            .iter()
            .map(|a| self.command_for_action(*a))
            .collect();

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

        // A block drag ends on pointer release anywhere, not only over the
        // six-dot handle. Releasing outside a block cancels the move.
        if self.drag_from.is_some() && !ctx.input(|i| i.pointer.primary_down()) {
            let over = ctx
                .data(|data| data.get_temp::<bool>(egui::Id::unique("block_drag_over")))
                .unwrap_or(false);
            commands.push(if over {
                Command::DropBlock
            } else {
                Command::CancelBlockDrag
            });
        }

        if view.search_open {
            render_search(&ctx, &self.workspace, &view, &palette, &mut commands);
        }
        if view.settings_open {
            render_settings(&ctx, &mut self.workspace, &view, &palette, &mut commands);
        }
        if view.onboarding_open {
            render_onboarding(&ctx, &view, &palette, &mut commands);
        }
        if view.templates_open {
            render_templates(&ctx, &view, &palette, &mut commands);
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
        if let Some(menu) = view.sidebar_menu {
            render_sidebar_menu(&ctx, menu, &self.workspace, &view, &palette, &mut commands);
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

    fn on_exit(&mut self) {
        // Flush pending edits even when the window is closed by the OS.
        self.commit_edits(f64::MAX, true);
    }
}
