//! Settings page: sidebar.

use crate::i18n;
use crate::model::Workspace;

use super::super::super::commands::Command;
use super::super::super::palette::Palette;
use super::super::super::settings_search::SettingsRow;
use super::super::super::view::ViewState;
use super::super::super::widgets::toggle_button;
use super::section_heading;
pub(in crate::ui::dialogs) fn render_sidebar_settings(
    ui: &mut egui::Ui,
    ws: &mut Workspace,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
    rows: &[&SettingsRow],
) {
    let mut section = "";
    for row in rows {
        if row.section != section {
            section = row.section;
            section_heading(ui, section, palette);
        }
        match row.id {
            "library" => {
                if toggle_button(
                    ui,
                    &i18n::t("library"),
                    ws.settings.sidebar_library,
                    palette,
                )
                .clicked()
                {
                    ws.settings.sidebar_library = !ws.settings.sidebar_library;
                }
            }
            "settings" => {
                if toggle_button(
                    ui,
                    &i18n::t("settings"),
                    ws.settings.sidebar_settings,
                    palette,
                )
                .clicked()
                {
                    ws.settings.sidebar_settings = !ws.settings.sidebar_settings;
                }
            }
            "inbox" => {
                if toggle_button(ui, &i18n::t("inbox"), ws.settings.sidebar_inbox, palette)
                    .clicked()
                {
                    ws.settings.sidebar_inbox = !ws.settings.sidebar_inbox;
                }
            }
            "favorites" => {
                if toggle_button(
                    ui,
                    &i18n::t("favorites"),
                    ws.settings.sidebar_favorites,
                    palette,
                )
                .clicked()
                {
                    ws.settings.sidebar_favorites = !ws.settings.sidebar_favorites;
                }
            }
            "private" => {
                if toggle_button(
                    ui,
                    &i18n::t("private"),
                    ws.settings.sidebar_private,
                    palette,
                )
                .clicked()
                {
                    ws.settings.sidebar_private = !ws.settings.sidebar_private;
                }
            }
            "invite_card" => {
                if toggle_button(ui, row.title, ws.settings.sidebar_invite, palette).clicked() {
                    ws.settings.sidebar_invite = !ws.settings.sidebar_invite;
                }
            }
            "customize" => {
                if toggle_button(ui, row.title, ws.settings.sidebar_customize, palette).clicked() {
                    ws.settings.sidebar_customize = !ws.settings.sidebar_customize;
                }
            }
            "other_apps" => {
                if toggle_button(ui, row.title, ws.settings.show_other_apps, palette).clicked() {
                    ws.settings.show_other_apps = !ws.settings.show_other_apps;
                }
            }
            "export" => {
                ui.horizontal(|ui| {
                    if ui.button(i18n::t("export_workspace")).clicked() {
                        commands.push(Command::ExportWorkspace);
                    }
                    if ui.button(i18n::t("export_members")).clicked() {
                        commands.push(Command::ExportMembers);
                    }
                });
                if let Some(status) = &view.settings_status {
                    ui.label(egui::RichText::new(status).size(11.0).color(palette.accent));
                }
                ui.add_space(10.0);
            }
            "analytics" => {
                if toggle_button(ui, row.title, ws.settings.analytics, palette).clicked() {
                    ws.settings.analytics = !ws.settings.analytics;
                }
                ui.add_space(10.0);
            }
            _ => {}
        }
    }
}
