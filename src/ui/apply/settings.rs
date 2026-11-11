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
            Command::ToggleSidebarSectionHidden(id) => {
                if let Some(section) = self.workspace.settings.sidebar_layout.section_mut(&id) {
                    section.hidden = !section.hidden;
                }
            }
            Command::ToggleSidebarShortcutHidden(section_id, shortcut_id) => {
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
            Command::MoveSidebarSection(id, index) => {
                let layout = &mut self.workspace.settings.sidebar_layout;
                if let Some(position) = layout.sections.iter().position(|section| section.id == id)
                {
                    let section = layout.sections.remove(position);
                    let target = index.min(layout.sections.len());
                    layout.sections.insert(target, section);
                }
            }
            Command::MoveSidebarShortcut(from, shortcut_id, to) => {
                let layout = &mut self.workspace.settings.sidebar_layout;
                let moved = layout.section_mut(&from).and_then(|section| {
                    let index = section
                        .shortcuts
                        .iter()
                        .position(|shortcut| shortcut.id == shortcut_id)?;
                    Some(section.shortcuts.remove(index))
                });
                if let Some(shortcut) = moved
                    && let Some(section) = layout.section_mut(&to)
                {
                    section.shortcuts.push(shortcut);
                }
            }
            Command::SetSidebarShowExcluded(_) => {}
            Command::ToggleSidebarNav(id) => match id.as_str() {
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
            Command::ToggleFullWidth => self.full_width = !self.full_width,
            Command::ToggleSmallText => self.small_text = !self.small_text,
            Command::SetFontStyle(style) => self.font_style = style,
            other => debug_assert!(false, "unexpected settings command: {other:?}"),
        }
    }
}
