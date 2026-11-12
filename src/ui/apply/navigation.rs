//! Command family: navigation.

use crate::model::PageDraft;
use crate::templates;

use super::super::app::NotionApp;
use super::super::commands::{Command, SidebarCmd};
use super::super::palette::apply_theme;
impl NotionApp {
    pub(super) fn apply_navigation(
        &mut self,
        command: Command,
        ctx: &egui::Context,
        commands: &mut Vec<Command>,
    ) {
        match command {
            Command::SelectPage(id) => {
                self.current_page = id.clone();
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    *tab = id.clone();
                }
                self.library_open = false;
                self.close_overlays();
                self.comment_input.clear();
                self.page_status = None;
                self.push_history(id);
            }
            Command::NewPage(parent) => {
                let draft = PageDraft {
                    title: "Untitled".to_string(),
                    icon: String::new(),
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
            Command::Sidebar(SidebarCmd::ToggleExpand(id)) => {
                if !self.expanded.insert(id.clone()) {
                    self.expanded.remove(&id);
                }
            }
            Command::Sidebar(SidebarCmd::ToggleFavorite(id)) => {
                if let Some(page) = self.workspace.page_mut(&id) {
                    page.favorite = !page.favorite;
                }
            }
            Command::OpenSearch => {
                self.close_overlays();
                self.refresh_members();
                self.search.open = true;
            }
            Command::ToggleSearch => {
                if self.search.open {
                    self.search.open = false;
                    self.search.query.clear();
                } else {
                    self.close_overlays();
                    self.refresh_members();
                    self.search.open = true;
                }
            }
            Command::CloseSearch => {
                self.search.open = false;
                self.search.query.clear();
            }
            Command::SetSearchQuery(query) => self.search.query = query,
            Command::ToggleSearchTitleOnly => {
                self.search.title_only = !self.search.title_only;
            }
            Command::SetSearchCreatedBy(name) => self.search.created_by = name,
            Command::SetSearchIn(page) => self.search.in_page = page,
            Command::ToggleSearchHasComments => {
                self.search.has_comments = !self.search.has_comments;
            }
            Command::OpenSettings => {
                self.close_overlays();
                self.settings.open = true;
                self.settings.status = None;
                if self.settings.page == "People" {
                    self.refresh_members();
                }
            }
            Command::CloseSettings => self.settings.open = false,
            Command::OpenOnboarding => {
                self.close_overlays();
                self.onboarding_open = true;
            }
            Command::CloseOnboarding => {
                self.onboarding_open = false;
                self.workspace.settings.onboarding_completed = true;
            }
            Command::CompleteOnboarding(purpose) => {
                self.workspace.settings.onboarding_completed = true;
                self.workspace.settings.workspace_purpose = Some(purpose.clone());
                self.onboarding_open = false;
                if let Some(template) = templates::template_for_purpose(&purpose) {
                    self.apply_template(template.id);
                }
            }
            Command::OpenTemplates => {
                self.close_overlays();
                self.templates_open = true;
            }
            Command::CloseTemplates => self.templates_open = false,
            Command::ApplyTemplate(id) => self.apply_template(&id),
            Command::ToggleTheme => {
                self.settings.dark = !self.settings.dark;
                self.settings.theme_dirty = true;
                apply_theme(ctx, self.settings.dark);
            }
            Command::Sidebar(SidebarCmd::Toggle) => self.sidebar_open = !self.sidebar_open,
            Command::SetTableView(table) => self.settings.table_view = table,
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
            Command::SetSearchSelected(index) => self.search.selected = index,
            Command::ToggleHighlightPane => self.highlight_pane = !self.highlight_pane,
            Command::ToggleFilters => self.filters = !self.filters,
            other => debug_assert!(false, "unexpected navigation command: {other:?}"),
        }
    }
}
