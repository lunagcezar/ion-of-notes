//! Settings page: people.

use crate::i18n;
use crate::model::Workspace;
use egui::Color32;

use super::super::super::commands::Command;
use super::super::super::palette::Palette;
use super::super::super::settings_search::SettingsRow;
use super::super::super::view::ViewState;
use super::super::super::widgets::toggle_button;
use super::section_heading;
pub(in crate::ui::dialogs) fn render_people_settings(
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
            "people_directory" => {
                if toggle_button(ui, row.title, ws.settings.people_directory, palette).clicked() {
                    ws.settings.people_directory = !ws.settings.people_directory;
                }
            }
            "recent_activity" => {
                if toggle_button(ui, row.title, ws.settings.recent_activity, palette).clicked() {
                    ws.settings.recent_activity = !ws.settings.recent_activity;
                }
            }
            "hover_cards" => {
                if toggle_button(
                    ui,
                    &i18n::t("hover_cards"),
                    ws.settings.hover_cards,
                    palette,
                )
                .clicked()
                {
                    ws.settings.hover_cards = !ws.settings.hover_cards;
                }
            }
            "members" => {
                if view.share_members.is_empty() {
                    ui.label(
                        egui::RichText::new(i18n::t("no_members"))
                            .size(12.0)
                            .color(palette.muted),
                    );
                }
                for member in view.share_members.iter() {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&member.name).color(palette.text));
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new(&member.email)
                                .size(11.0)
                                .color(palette.muted),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(member.role.label())
                                    .size(11.0)
                                    .color(palette.muted),
                            );
                        });
                    });
                    ui.add_space(4.0);
                }
                if ui
                    .add(egui::Button::new(i18n::t("invite_members")))
                    .clicked()
                {
                    commands.push(Command::OpenShare);
                }
                ui.add_space(10.0);
            }
            "danger_zone" => {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(
                            "Permanently delete this workspace, including all pages and files.",
                        )
                        .size(12.0)
                        .color(palette.muted),
                    );
                    if ui
                        .add(egui::Button::new(
                            egui::RichText::new(i18n::t("delete_workspace"))
                                .color(Color32::from_rgb(224, 108, 108)),
                        ))
                        .clicked()
                    {
                        commands.push(Command::SetSettingsStatus(i18n::t("delete_disabled")));
                    }
                });
                if let Some(status) = &view.settings_status {
                    ui.label(egui::RichText::new(status).size(11.0).color(palette.accent));
                }
                ui.add_space(12.0);
            }
            "workspace_id" => {
                ui.label(
                    egui::RichText::new(i18n::t("workspace_id"))
                        .size(12.0)
                        .color(palette.text),
                );
                ui.label(
                    egui::RichText::new(&ws.settings.id)
                        .size(11.0)
                        .color(palette.muted),
                );
                ui.add_space(10.0);
            }
            _ => {}
        }
    }
}
