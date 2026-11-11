//! Command family: workspace.

use crate::model::Workspace;
use uuid::Uuid;

use super::super::app::NotionApp;
use super::super::commands::Command;
impl NotionApp {
    pub(super) fn apply_workspace(&mut self, command: Command, ctx: &egui::Context) -> bool {
        let mut had_undo_redo = false;
        match command {
            Command::ResetWorkspace => {
                let _ = self
                    .client
                    .replace_workspace(&Workspace::seed("ion-of-notes"));
                self.pending_reload = true;
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
                ctx.copy_text(format!("ion-of-notes://page/{}", self.current_page));
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
            other => debug_assert!(false, "unexpected workspace command: {other:?}"),
        }
        had_undo_redo
    }
}
