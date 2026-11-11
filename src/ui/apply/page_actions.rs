//! Command family: page_actions.

use crate::collab::page_markdown;
use crate::i18n;
use crate::model::{Comment, Page, now_label};
use std::sync::Arc;
use uuid::Uuid;

use super::super::app::NotionApp;
use super::super::commands::{Command, IconTab};
impl NotionApp {
    pub(super) fn apply_page_actions(&mut self, command: Command, ctx: &egui::Context) {
        match command {
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
                self.page_status = None;
                self.page_actions_open = true;
            }
            Command::ClosePageActions => self.page_actions_open = false,
            Command::CopyPageContents => {
                if let Some(page) = self.workspace.page(&self.current_page) {
                    ctx.copy_text(page_markdown(page));
                    self.page_status = Some(i18n::t("copied_contents"));
                }
            }
            Command::TogglePageToc => {
                if let Some(page) = self.current_page_mut() {
                    page.show_toc = !page.show_toc;
                }
            }
            Command::TogglePageDiscussions => {
                if let Some(page) = self.current_page_mut() {
                    page.discussions = !page.discussions;
                }
            }
            Command::TogglePageInlineComments => {
                if let Some(page) = self.current_page_mut() {
                    page.inline_comments = !page.inline_comments;
                }
            }
            Command::TogglePageArchive => {
                if let Some(page) = self.current_page_mut() {
                    page.archived = !page.archived;
                    self.page_status = Some(if page.archived {
                        i18n::t("page_archived")
                    } else {
                        i18n::t("page_restored")
                    });
                }
            }
            Command::SetCommentInput(input) => self.comment_input = input,
            Command::AddPageComment => {
                let text = self.comment_input.trim().to_string();
                if text.is_empty() {
                    self.page_status = Some(i18n::t("write_comment_first"));
                } else if let Some(page) = self.current_page_mut() {
                    page.comments.push(Comment::new(
                        Uuid::new_v4().to_string(),
                        "Local user",
                        text,
                        now_label(),
                    ));
                    self.comment_input.clear();
                    self.page_status = Some(i18n::t("comment_added"));
                }
            }
            Command::DeletePageComment(id) => {
                if let Some(page) = self.current_page_mut() {
                    page.comments.retain(|comment| comment.id != id);
                }
            }
            Command::SetPageStatus(message) => self.page_status = Some(message),
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
            other => debug_assert!(false, "unexpected page_actions command: {other:?}"),
        }
    }
}
