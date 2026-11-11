//! Command family: settings.

use crate::i18n;

use super::super::app::NotionApp;
use super::super::commands::Command;
impl NotionApp {
    pub(super) fn apply_settings(&mut self, command: Command) {
        match command {
            // ---- Settings / help / window ------------------------------
            Command::SetSettingsPage(page) => {
                self.settings_page = page.clone();
                self.settings_status = None;
                if page == "People" {
                    self.refresh_members();
                }
            }
            Command::SetSettingsQuery(query) => self.settings_query = query,
            Command::SetSettingsStatus(message) => self.settings_status = Some(message),
            Command::ExportWorkspace => match self.client.export_tree() {
                Ok(files) => {
                    self.settings_status = Some(i18n::t_count("exported_files", files.len()))
                }
                Err(error) => self.settings_status = Some(error.to_string()),
            },
            Command::ExportMembers => match self.client.export_tree() {
                Ok(files) => {
                    let count = files
                        .iter()
                        .filter(|file| file.path.starts_with("People"))
                        .count();
                    self.settings_status = Some(i18n::t_count("exported_people_files", count));
                }
                Err(error) => self.settings_status = Some(error.to_string()),
            },
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
            other => debug_assert!(false, "unexpected settings command: {other:?}"),
        }
    }
}
