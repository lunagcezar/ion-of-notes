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
use crate::server::WorkspaceServer;
use egui::Pos2;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use super::dialogs::{
    render_app_menu, render_grid_menu, render_inbox, render_new_tab, render_onboarding,
    render_page_actions, render_search, render_settings, render_share, render_shortcuts,
    render_slash, render_templates, render_version_history, render_workspace_menu,
};
use super::edit_queue::PendingEdits;
use super::page_icon::render_icon_picker;
use super::palette::{Palette, apply_theme};
use super::render::{TITLE_BAR_HEIGHT, render_content, render_sidebar, render_title_bar};
use super::search_cache::SearchCache;

pub(super) use super::commands::{Command, FontStyle, IconTab, NewTabChoice};
use super::input::collect_shortcuts;
pub(super) use super::view::ViewState;

pub struct NotionApp {
    pub(super) client: Client<WorkspaceServer>,
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
    pub(super) ai_query: String,
    pub(super) ai_answer: Option<String>,
    pub(super) ai_error: Option<String>,
    pub(super) ai_busy: bool,
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
}

impl NotionApp {
    pub fn new(cc: &eframe::CreationContext<'_>, screenshot: Option<PathBuf>) -> Self {
        let initial_locale =
            std::env::var("ION_OF_NOTES_LOCALE").unwrap_or_else(|_| "en".to_string());
        crate::i18n::set_locale(&initial_locale);
        let language = crate::i18n::current_locale();
        // The composition root (app layer) wires the concrete JSON adapter to
        // the server/client ports; the window only consumes the ready graph.
        let (mut client, mut workspace) = app::build_desktop();
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
        let ai_config = OllamaConfig::from_env();
        let ai_status = ollama::detect(&ai_config);
        let onboarding_open = demo == "onboarding"
            || (workspace.pages.is_empty() && !workspace.settings.onboarding_completed);
        let templates_open = demo == "templates";
        Self {
            client,
            last_committed: workspace.clone(),
            pending_edits: PendingEdits::new(0.25),
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
            settings_open: matches!(
                demo.as_str(),
                "settings"
                    | "ai"
                    | "settings-general"
                    | "settings-sidebar"
                    | "settings-people"
                    | "settings-offline"
            ),
            onboarding_open,
            templates_open,
            settings_page: match demo.as_str() {
                "ai" => "AI assistant".to_string(),
                "settings-general" => "General".to_string(),
                "settings-sidebar" => "Sidebar".to_string(),
                "settings-people" => "People".to_string(),
                "settings-offline" => "Offline".to_string(),
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
            page_status: None,
            comment_input: String::new(),
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
