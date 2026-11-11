//! Command family: navigation.

use crate::model::PageDraft;
use crate::templates;

use super::super::app::NotionApp;
use super::super::commands::Command;
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
                self.settings_status = None;
                if self.settings_page == "People" {
                    self.refresh_members();
                }
            }
            Command::CloseSettings => self.settings_open = false,
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
                self.dark = !self.dark;
                self.theme_dirty = true;
                apply_theme(ctx, self.dark);
            }
            Command::ToggleSidebar => self.sidebar_open = !self.sidebar_open,
            Command::SetTableView(table) => self.table_view = table,
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
            Command::SetSearchSelected(index) => self.search_selected = index,
            Command::ToggleHighlightPane => self.highlight_pane = !self.highlight_pane,
            Command::ToggleFilters => self.filters = !self.filters,
            other => debug_assert!(false, "unexpected navigation command: {other:?}"),
        }
    }
}
