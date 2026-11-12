//! Command family: tabs.

use crate::model::PageDraft;

use super::super::app::NotionApp;
use super::super::commands::{Command, NewTabChoice, new_tab_choices};
impl NotionApp {
    pub(super) fn apply_tabs(
        &mut self,
        command: Command,
        ctx: &egui::Context,
        commands: &mut Vec<Command>,
    ) {
        match command {
            // ---- Tabs / "+" switcher ----------------------------------
            Command::OpenNewTab => {
                self.close_overlays();
                self.refresh_members();
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
                } else if self.search.open
                    && let Some(result) = self.search.results.get(self.search.selected)
                {
                    let id = result.page_id.clone();
                    self.open_in_tab(id);
                    self.close_overlays();
                }
            }
            Command::ActivateNewTab(choice) => match choice {
                NewTabChoice::NewChat => {
                    self.close_overlays();
                    self.search.open = true;
                    self.ai_query.clear();
                    self.ai_answer = None;
                    self.ai_error = None;
                }
                NewTabChoice::NewPage => {
                    let draft = PageDraft {
                        title: "Untitled".to_string(),
                        icon: String::new(),
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
            other => debug_assert!(false, "unexpected tabs command: {other:?}"),
        }
    }
}
