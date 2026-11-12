//! Command family: settings.

use crate::i18n;

use super::super::app::NotionApp;
use super::super::commands::{Command, SidebarCmd, WorkspaceDialogMode};
impl NotionApp {
    pub(super) fn apply_settings(&mut self, command: Command) {
        match command {
            // ---- Settings / help / window ------------------------------
            Command::SetSettingsPage(page) => {
                self.settings.page = page.clone();
                self.settings.status = None;
                if page == "People" {
                    self.refresh_members();
                }
            }
            Command::SetSettingsQuery(query) => self.settings.query = query,
            Command::SetSettingsStatus(message) => self.settings.status = Some(message),
            Command::ExportWorkspace => match self.client.export_tree() {
                Ok(files) => {
                    self.settings.status = Some(i18n::t_count("exported_files", files.len()))
                }
                Err(error) => self.settings.status = Some(error.to_string()),
            },
            Command::ExportMembers => match self.client.export_tree() {
                Ok(files) => {
                    let count = files
                        .iter()
                        .filter(|file| file.path.starts_with("People"))
                        .count();
                    self.settings.status = Some(i18n::t_count("exported_people_files", count));
                }
                Err(error) => self.settings.status = Some(error.to_string()),
            },
            Command::SetLanguage(locale) => {
                crate::i18n::set_locale(&locale);
                self.settings.language = locale;
            }
            Command::SetNumberFormat(format) => {
                self.workspace.settings.number_format = format;
            }
            Command::SetTimeFormat(format) => {
                self.workspace.settings.time_format = format;
            }
            Command::OpenShortcuts => {
                self.close_overlays();
                self.shortcuts_open = true;
            }
            Command::CloseShortcuts => self.shortcuts_open = false,
            Command::ZoomIn => {
                self.settings.zoom = (self.settings.zoom + 0.1).min(1.6);
                self.settings.zoom_dirty = true;
            }
            Command::ZoomOut => {
                self.settings.zoom = (self.settings.zoom - 0.1).max(0.7);
                self.settings.zoom_dirty = true;
            }
            Command::ZoomReset => {
                self.settings.zoom = 1.0;
                self.settings.zoom_dirty = true;
            }
            Command::ToggleWorkspaceMenu => {
                let open = !self.workspace_menu_open;
                self.close_overlays();
                self.workspace_menu_open = open;
            }
            Command::CloseWorkspaceMenu => self.workspace_menu_open = false,
            Command::SetSidebarCustomize(on) => {
                self.sidebar_customize = on;
                self.sidebar_menu = None;
            }
            Command::OpenSidebarMenu(menu) => self.sidebar_menu = Some(menu),
            Command::CloseSidebarMenu => self.sidebar_menu = None,
            Command::AddSidebarShortcut(page_id, label) => {
                self.workspace
                    .settings
                    .sidebar_layout
                    .add_shortcut(&page_id, &label);
            }
            Command::AddSidebarSection(title, source) => {
                self.workspace
                    .settings
                    .sidebar_layout
                    .add_section(&title, source);
            }
            Command::Sidebar(SidebarCmd::ToggleSectionHidden(id)) => {
                if let Some(section) = self.workspace.settings.sidebar_layout.section_mut(&id) {
                    section.hidden = !section.hidden;
                }
            }
            Command::Sidebar(SidebarCmd::ToggleShortcutHidden(section_id, shortcut_id)) => {
                if let Some(section) = self
                    .workspace
                    .settings
                    .sidebar_layout
                    .section_mut(&section_id)
                    && let Some(shortcut) = section
                        .shortcuts
                        .iter_mut()
                        .find(|shortcut| shortcut.id == shortcut_id)
                {
                    shortcut.hidden = !shortcut.hidden;
                }
            }
            Command::RemoveSidebarSection(id) => {
                self.workspace
                    .settings
                    .sidebar_layout
                    .sections
                    .retain(|section| section.id != id);
            }
            Command::RemoveSidebarShortcut(section_id, shortcut_id) => {
                if let Some(section) = self
                    .workspace
                    .settings
                    .sidebar_layout
                    .section_mut(&section_id)
                {
                    section
                        .shortcuts
                        .retain(|shortcut| shortcut.id != shortcut_id);
                }
            }
            Command::RenameSidebarSection(id, title) => {
                if let Some(section) = self.workspace.settings.sidebar_layout.section_mut(&id) {
                    section.title = title;
                }
            }
            Command::RenameSidebarShortcut(section_id, shortcut_id, title) => {
                if let Some(section) = self
                    .workspace
                    .settings
                    .sidebar_layout
                    .section_mut(&section_id)
                    && let Some(shortcut) = section
                        .shortcuts
                        .iter_mut()
                        .find(|shortcut| shortcut.id == shortcut_id)
                {
                    shortcut.label = title;
                }
            }
            Command::Sidebar(SidebarCmd::MoveSection(id, index)) => {
                self.workspace
                    .settings
                    .sidebar_layout
                    .move_section(&id, index);
            }
            Command::Sidebar(SidebarCmd::MoveShortcut(from, shortcut_id, to, index)) => {
                self.workspace.settings.sidebar_layout.move_shortcut(
                    &from,
                    &shortcut_id,
                    &to,
                    index,
                );
            }
            Command::Sidebar(SidebarCmd::SetShowExcluded(_)) => {}
            Command::Sidebar(SidebarCmd::AddTab) => {
                self.workspace.settings.sidebar_layout.add_tab()
            }
            Command::Sidebar(SidebarCmd::SwitchTab(id)) => {
                self.workspace.settings.sidebar_layout.switch_tab(&id);
            }
            Command::Sidebar(SidebarCmd::RemoveTab(id)) => {
                self.workspace.settings.sidebar_layout.remove_tab(&id);
            }
            Command::Sidebar(SidebarCmd::ToggleNav(id)) => match id.as_str() {
                "library" => {
                    self.workspace.settings.sidebar_library =
                        !self.workspace.settings.sidebar_library;
                }
                "settings" => {
                    self.workspace.settings.sidebar_settings =
                        !self.workspace.settings.sidebar_settings;
                }
                "inbox" => {
                    self.workspace.settings.sidebar_inbox = !self.workspace.settings.sidebar_inbox;
                }
                _ => {}
            },
            Command::OpenWorkspaceCreate => {
                self.workspace_dialog = Some(WorkspaceDialogMode::Create);
                self.workspace_name.clear();
                self.workspace_icon = "N".to_string();
                self.workspace_menu_open = false;
            }
            Command::OpenWorkspaceRename => {
                self.workspace_dialog = Some(WorkspaceDialogMode::Rename);
                self.workspace_name = self.workspace.name.clone();
                self.workspace_icon = self.workspace.settings.icon.clone();
                self.workspace_menu_open = false;
            }
            Command::CloseWorkspaceDialog => self.workspace_dialog = None,
            Command::SetWorkspaceName(name) => self.workspace_name = name,
            Command::SetWorkspaceIcon(icon) => self.workspace_icon = icon,
            Command::ConfirmWorkspaceDialog => {
                let name = self.workspace_name.trim().to_string();
                let icon = self.workspace_icon.clone();
                if !name.is_empty() {
                    match self.workspace_dialog {
                        Some(WorkspaceDialogMode::Create) => {
                            self.create_workspace(&name, &icon);
                        }
                        Some(WorkspaceDialogMode::Rename) => {
                            self.rename_workspace(&name, &icon);
                        }
                        None => {}
                    }
                }
                self.workspace_dialog = None;
            }
            Command::SwitchWorkspace(id) => {
                self.switch_workspace(&id);
                self.workspace_menu_open = false;
            }
            Command::DeleteWorkspace(id) => {
                self.delete_workspace(&id);
                self.workspace_menu_open = false;
            }
            Command::OpenAppMenu => {
                let open = !self.app_menu_open;
                self.close_overlays();
                self.app_menu_open = open;
            }
            Command::CloseAppMenu => self.app_menu_open = false,
            Command::ToggleComposeMenu => {
                let open = !self.compose_menu_open;
                self.close_overlays();
                self.compose_menu_open = open;
            }
            Command::CloseComposeMenu => self.compose_menu_open = false,
            Command::ToggleGridMenu => {
                let open = !self.grid_menu_open;
                self.close_overlays();
                self.grid_menu_open = open;
            }
            Command::CloseGridMenu => self.grid_menu_open = false,
            Command::ToggleFullWidth => self.settings.full_width = !self.settings.full_width,
            Command::ToggleSmallText => self.settings.small_text = !self.settings.small_text,
            Command::SetFontStyle(style) => self.settings.font_style = style,
            other => debug_assert!(false, "unexpected settings command: {other:?}"),
        }
    }
}
