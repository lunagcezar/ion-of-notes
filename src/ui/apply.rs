//! Command application and app-state transitions.
//!
//! `NotionApp` owns the state; this module applies the collected `Command`s
//! through the client and keeps navigation, tabs, history and panels coherent.
//! Splitting it from the struct/eframe shell keeps `ui::app` small (SRP,
//! spec-017).

use crate::collab::{MemberDraft, Share, general_access};
use crate::model::{Block, BlockKind, Page, PageDraft, TableRow, Workspace};
use crate::ollama;
use crate::shortcuts::Action;
use std::sync::Arc;
use uuid::Uuid;

use super::app::NotionApp;
use super::commands::{Command, IconTab, NewTabChoice, new_tab_choices};
use super::edit_queue::{execute_plan, plan_for};
use super::invite::{is_valid_email, member_name_from_email, parse_invite_emails};
use super::palette::apply_theme;
use super::view::ViewState;

impl NotionApp {
    pub(super) fn view_state(&self) -> ViewState {
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
            share_members: Arc::clone(&self.share_members),
            invite_input: self.invite_input.clone(),
            invite_role: self.invite_role,
            invite_message: self.invite_message.clone(),
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
    pub(super) fn apply_commands(
        &mut self,
        mut commands: Vec<Command>,
        ctx: &egui::Context,
        frame_before: Workspace,
    ) {
        let now = ctx.input(|i| i.time);
        let force_flush = commands.iter().any(|command| {
            matches!(
                command,
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
        });
        if force_flush {
            self.commit_edits(now, true);
        }

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
                    self.invite_error = None;
                    self.invite_status = None;
                    self.refresh_share();
                }
                Command::CloseShare => self.share_open = false,
                Command::SetInviteInput(input) => self.invite_input = input,
                Command::SetInviteRole(role) => self.invite_role = role,
                Command::SetInviteMessage(message) => self.invite_message = message,
                Command::SendInvites => {
                    let emails: Vec<String> = parse_invite_emails(&self.invite_input)
                        .into_iter()
                        .filter(|email| is_valid_email(email))
                        .collect();
                    if emails.is_empty() {
                        self.invite_error =
                            Some("Enter at least one valid email address.".to_string());
                        self.invite_status = None;
                    } else {
                        let mut invited = 0;
                        let mut errors = Vec::new();
                        for email in emails {
                            let draft = MemberDraft {
                                name: member_name_from_email(&email),
                                email,
                                role: self.invite_role,
                            };
                            match self.client.invite_member(&draft) {
                                Ok(_) => invited += 1,
                                Err(error) => errors.push(error.to_string()),
                            }
                        }
                        if invited > 0 {
                            self.invite_input.clear();
                            self.invite_status = Some(format!("Invited {invited} member(s)."));
                        }
                        self.invite_error = if errors.is_empty() {
                            None
                        } else {
                            Some(errors.join("; "))
                        };
                        self.refresh_share();
                    }
                }
                Command::SetGeneralAccess(access) => {
                    let share = Share {
                        page_id: self.current_page.clone(),
                        subject: "Everyone at workspace".to_string(),
                        access,
                    };
                    match self.client.set_share(&share) {
                        Ok(_) => {
                            self.share_general_access = access;
                            self.invite_error = None;
                            self.invite_status = Some("General access updated.".to_string());
                        }
                        Err(error) => self.invite_error = Some(error.to_string()),
                    }
                }
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
                Command::ApplyMarkdown {
                    block,
                    kind,
                    prefix,
                } => {
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
                        page.blocks
                            .insert(at, Block::new(new_id.clone(), BlockKind::Paragraph, ""));
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
                    if self.drag_from.is_some() && self.drag_from.as_deref() != Some(id.as_str()) {
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
                    let _ = self
                        .client
                        .replace_workspace(&Workspace::seed("not-notion"));
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
                Command::ActivateNewTab(choice) => match choice {
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
                },
                Command::SelectTab(index) => {
                    if let Some(id) = self.tabs.get(index).cloned() {
                        self.active_tab = index;
                        self.current_page = id;
                        self.library_open = false;
                        self.close_overlays();
                    }
                }
                Command::CloseTab(index) => {
                    let index = if index == usize::MAX {
                        self.active_tab
                    } else {
                        index
                    };
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
                        self.active_tab = (self.active_tab + self.tabs.len() - 1) % self.tabs.len();
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
                }
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
                    self.versions = Arc::new(
                        self.client
                            .page_versions(&self.current_page)
                            .unwrap_or_default(),
                    );
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
