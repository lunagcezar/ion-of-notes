//! Command application and app-state transitions.
//!
//! `NotionApp` owns the state; this module applies the collected `Command`s
//! through the client and keeps navigation, tabs, history and panels coherent.
//! Splitting it from the struct/eframe shell keeps `ui::app` small (SRP,
//! spec-017).

use crate::client::Client;
use crate::collab::{Role, general_access};
use crate::model::{Block, Page, Workspace};
use crate::server::realtime::ServerMessage;
use crate::shortcuts::Action;
use crate::templates;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use std::collections::HashSet;
use std::sync::Arc;
use uuid::Uuid;

use super::app::NotionApp;
use super::commands::{Command, CommandFamily};

mod ai;
mod blocks;
mod crdt;
mod library;
mod navigation;
mod page_actions;
mod server;
mod settings;
mod sharing;
mod tabs;
mod workspace;
use super::crdt::CrdtSession;
use super::edit_queue::{PendingEdits, execute_plan, plan_for};
use super::view::ViewState;

pub(super) fn set_block_text(blocks: &mut [Block], block_id: &str, text: &str) -> bool {
    for block in blocks {
        if block.id == block_id {
            block.text = text.to_string();
            block.normalize();
            return true;
        }
        if set_block_text(&mut block.children, block_id, text) {
            return true;
        }
    }
    false
}

impl NotionApp {
    pub(super) fn view_state(&self) -> ViewState {
        let (available_offline, lock_page) = self
            .workspace
            .page(&self.current_page)
            .map(|p| (p.available_offline, p.locked))
            .unwrap_or((false, false));
        ViewState {
            current_page: self.current_page.clone(),
            remote_cursors: self.remote_awareness.clone(),
            server_url: self.server_url.clone(),
            server_email: self.server_email.clone(),
            server_password: self.server_password.clone(),
            server_status: self.server_status.clone(),
            server_connected: self.server_connected,
            server_busy: self.server_busy,
            tabs: self.tabs.clone(),
            active_tab: self.active_tab,
            sidebar_open: self.sidebar_open,
            expanded: self.expanded.clone(),
            search_open: self.search_open,
            search_query: self.search_query.clone(),
            search_results: Arc::clone(&self.search_results),
            settings_open: self.settings_open,
            onboarding_open: self.onboarding_open,
            templates_open: self.templates_open,
            settings_page: self.settings_page.clone(),
            settings_query: self.settings_query.clone(),
            settings_status: self.settings_status.clone(),
            language: self.language.clone(),
            dark: self.dark,
            table_view: self.table_view,
            library_open: self.library_open,
            library_tab: self.library_tab.clone(),
            library_shared: Arc::clone(&self.library_shared),
            library_owner: self.library_owner.clone(),
            share_open: self.share_open,
            share_members: Arc::clone(&self.share_members),
            invite_input: self.invite_input.clone(),
            invite_error: self.invite_error.clone(),
            invite_status: self.invite_status.clone(),
            share_general_access: self.share_general_access,
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
            page_status: self.page_status.clone(),
            comment_input: self.comment_input.clone(),
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
        self.onboarding_open = false;
        self.templates_open = false;
        self.drag_from = None;
        self.drag_to = None;
    }

    /// Add `id` as a new active tab (the "+" / Ctrl+T / Ctrl+Enter behaviour).
    fn open_in_tab(&mut self, id: String) {
        self.tabs.push(id.clone());
        self.active_tab = self.tabs.len() - 1;
        self.current_page = id.clone();
        self.library_open = false;
        self.comment_input.clear();
        self.page_status = None;
        self.push_history(id);
    }

    pub(super) fn poll_ai(&mut self) {
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

    pub(super) fn command_for_action(&self, action: Action) -> Command {
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

    /// Refresh the member directory and the general-access level for the
    /// current page (spec-023).
    fn refresh_share(&mut self) {
        self.share_members = Arc::new(self.client.list_members().unwrap_or_default());
        let shares = self
            .client
            .list_shares(Some(&self.current_page))
            .unwrap_or_default();
        self.share_general_access = general_access(&shares, &self.current_page);
    }

    /// Create a page from a starter template and open it in the active tab.
    fn apply_template(&mut self, template_id: &str) {
        let Some(template) = templates::find(template_id) else {
            return;
        };
        let id = Uuid::new_v4().to_string();
        let page = template.to_page(id.clone());
        if self.workspace.add_page(page).is_ok() {
            self.current_page = id.clone();
            if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                *tab = id.clone();
            }
            self.library_open = false;
            self.onboarding_open = false;
            self.templates_open = false;
            self.push_history(id);
        }
    }

    /// Refresh the member directory used by Share and Settings → People.
    fn refresh_members(&mut self) {
        self.share_members = Arc::new(self.client.list_members().unwrap_or_default());
    }

    /// Refresh the Library filters and owner label (spec-024).
    fn refresh_library(&mut self) {
        let shares = self.client.list_shares(None).unwrap_or_default();
        let shared: HashSet<String> = shares
            .iter()
            .filter(|share| share.page_id != "*")
            .map(|share| share.page_id.clone())
            .collect();
        self.library_shared = Arc::new(shared);
        let members = self.client.list_members().unwrap_or_default();
        self.library_owner = members
            .iter()
            .find(|member| member.role == Role::Owner)
            .map(|member| member.name.clone())
            .unwrap_or_else(|| "Local workspace".to_string());
    }

    /// Persist the difference between the last committed state and the
    /// current workspace. Single-page edits use `PATCH`; structural changes
    /// use the full snapshot fallback. The debounce window keeps rapid typing
    /// from sending one request per frame (spec-020).
    pub(super) fn commit_edits(&mut self, now: f64, force: bool) {
        let diff = self.last_committed.diff(&self.workspace);
        if diff.is_empty() {
            self.pending_edits.flushed();
            return;
        }

        self.pending_edits.observe(&diff, now);
        if force {
            self.pending_edits.force();
        }
        if !self.pending_edits.should_flush(now) {
            return;
        }

        let plan = plan_for(&diff);
        let sent = execute_plan(&mut self.client, &self.workspace, &plan);

        if sent {
            self.last_committed = self.workspace.clone();
            self.pending_edits.flushed();
        } else {
            // Keep the diff but wait another debounce window before retrying.
            self.pending_edits.mark(now);
        }
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn sync_crdt(&mut self) {
        if self.crdt_page != self.current_page
            && let Some(page) = self.workspace.page(&self.current_page).cloned()
        {
            self.crdt_page = self.current_page.clone();
            self.last_awareness = None;
            self.crdt.open_empty(&self.current_page);
            if let Ok((update, _)) = self.client.crdt_sync(&self.current_page, None)
                && self.crdt.apply_update(&update).is_ok()
            {
                self.crdt.baseline_page(&page);
                self.project_crdt_page();
            } else {
                self.crdt.open_page(&page);
            }
        }

        let mut messages = Vec::new();
        if let Some(receiver) = &mut self.crdt_rx {
            while let Ok(message) = receiver.try_recv() {
                messages.push(message);
            }
        }
        for message in messages {
            match message {
                ServerMessage::CrdtUpdate { page_id, update }
                | ServerMessage::CrdtSync {
                    page_id, update, ..
                } => {
                    if page_id == self.crdt_page
                        && let Ok(bytes) = BASE64.decode(update)
                        && self.crdt.apply_update(&bytes).is_ok()
                    {
                        self.project_crdt_page();
                    }
                }
                ServerMessage::Awareness { members } => {
                    self.remote_awareness = members
                        .into_iter()
                        .filter(|member| member.session_id != self.awareness_session)
                        .collect();
                }
                _ => {}
            }
        }
    }

    fn replace_client(&mut self, client: Client<crate::app::DesktopApi>, workspace: Workspace) {
        self.client = client;
        self.workspace = workspace;
        self.last_committed = self.workspace.clone();
        self.current_page = self
            .workspace
            .pages
            .first()
            .map(|page| page.id.clone())
            .unwrap_or_default();
        self.tabs = if self.current_page.is_empty() {
            Vec::new()
        } else {
            vec![self.current_page.clone()]
        };
        self.active_tab = 0;
        self.closed_tabs.clear();
        self.nav_history = self.tabs.clone();
        self.nav_pos = 0;
        self.pending_edits = PendingEdits::new(0.25);
        self.crdt = CrdtSession::new();
        self.crdt_page.clear();
        self.crdt_rx = self.client.subscribe_realtime();
        self.remote_awareness.clear();
        self.last_awareness = None;
        self.server_password.clear();
    }

    fn project_crdt_page(&mut self) {
        let page_id = self.crdt_page.clone();
        if let Some(mut page) = self.workspace.page(&page_id).cloned() {
            self.crdt.project_page(&mut page);
            if let Some(slot) = self.workspace.page_mut(&page_id) {
                *slot = page.clone();
            }
            if let Some(committed) = self.last_committed.page_mut(&page_id) {
                *committed = page;
            }
        }
    }

    pub(super) fn apply_commands(
        &mut self,
        mut commands: Vec<Command>,
        ctx: &egui::Context,
        frame_before: Workspace,
    ) {
        let now = ctx.input(|i| i.time);
        let force_flush = commands.iter().any(Command::forces_flush);
        if force_flush {
            self.commit_edits(now, true);
        }

        let mut had_undo_redo = false;
        let mut index = 0;
        while index < commands.len() {
            let command = commands[index].clone();
            let locked = self.page_locked();
            let blocked = locked && command.blocked_when_locked();
            if blocked {
                index += 1;
                continue;
            }
            match command.family() {
                CommandFamily::Navigation => self.apply_navigation(command, ctx, &mut commands),
                CommandFamily::Library => self.apply_library(command),
                CommandFamily::Sharing => self.apply_sharing(command),
                CommandFamily::Blocks => self.apply_blocks(command),
                CommandFamily::Workspace => had_undo_redo |= self.apply_workspace(command, ctx),
                CommandFamily::Tabs => self.apply_tabs(command, ctx, &mut commands),
                CommandFamily::Ai => self.apply_ai(command, &mut commands),
                CommandFamily::Settings => self.apply_settings(command),
                CommandFamily::PageActions => self.apply_page_actions(command, ctx),
                CommandFamily::Crdt => self.apply_crdt(command),
                CommandFamily::Server => self.apply_server(command),
                CommandFamily::NoOp => {}
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
            self.commit_edits(now, true);
            if let Ok(workspace) = self.client.workspace() {
                self.workspace = workspace;
                self.last_committed = self.workspace.clone();
                self.pending_edits.flushed();
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

        self.commit_edits(now, false);
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
