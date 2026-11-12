//! Settings page: workspace.

use crate::i18n;
use crate::model::Workspace;
use crate::ui::constants as metrics;

use super::super::super::commands::Command;
use super::super::super::palette::Palette;
use super::super::super::settings_search::SettingsRow;
use super::super::super::view::ViewState;
use super::section_heading;
pub(in crate::ui::dialogs) fn render_workspace_settings(
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
            "workspace_name" => {
                ui.label(
                    egui::RichText::new(row.title)
                        .size(metrics::FONT_CAPTION)
                        .color(palette.text),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut ws.name)
                        .desired_width(320.0)
                        .text_color(palette.text),
                );
                ui.add_space(metrics::SPACE_8);
            }
            "icon" => {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(row.title)
                            .size(metrics::FONT_CAPTION)
                            .color(palette.text),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut ws.settings.icon)
                            .desired_width(48.0)
                            .text_color(palette.text),
                    );
                });
                ui.add_space(metrics::SPACE_8);
            }
            "landing_page" => {
                ui.label(
                    egui::RichText::new(row.title)
                        .size(metrics::FONT_CAPTION)
                        .color(palette.text),
                );
                let pages: Vec<(String, String)> = ws
                    .pages
                    .iter()
                    .map(|page| (page.id.clone(), page.title.clone()))
                    .collect();
                let first_page = i18n::t("first_page");
                let selected = ws
                    .settings
                    .landing_page
                    .as_deref()
                    .and_then(|id| {
                        pages
                            .iter()
                            .find(|(page_id, _)| page_id == id)
                            .map(|(_, title)| title.as_str())
                    })
                    .unwrap_or(&first_page);
                egui::ComboBox::from_id_salt("landing_page")
                    .selected_text(selected)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut ws.settings.landing_page, None, &first_page);
                        for (id, title) in &pages {
                            ui.selectable_value(
                                &mut ws.settings.landing_page,
                                Some(id.clone()),
                                title,
                            );
                        }
                    });
                ui.add_space(metrics::SPACE_8);
            }
            "allowed_domains" => {
                let input_id = egui::Id::unique("allowed_domain_input");
                let mut domain: String = ui
                    .ctx()
                    .data(|data| data.get_temp(input_id).unwrap_or_default());
                ui.horizontal(|ui| {
                    let edit = ui.add(
                        egui::TextEdit::singleline(&mut domain)
                            .desired_width(260.0)
                            .hint_text("Type an email domain...")
                            .text_color(palette.text),
                    );
                    if edit.changed() {
                        ui.ctx()
                            .data_mut(|data| data.insert_temp(input_id, domain.clone()));
                    }
                    if ui.button(i18n::t("add_domain")).clicked() {
                        let value = domain.trim();
                        if !value.is_empty()
                            && !ws.settings.allowed_domains.iter().any(|item| item == value)
                        {
                            ws.settings.allowed_domains.push(value.to_string());
                        }
                        ui.ctx()
                            .data_mut(|data| data.remove_temp::<String>(input_id));
                    }
                });
                let mut remove = None;
                for (index, value) in ws.settings.allowed_domains.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(value)
                                .size(metrics::FONT_CAPTION)
                                .color(palette.text),
                        );
                        if ui.button(i18n::t("remove")).clicked() {
                            remove = Some(index);
                        }
                    });
                }
                if let Some(index) = remove {
                    ws.settings.allowed_domains.remove(index);
                }
                ui.add_space(metrics::SPACE_12);
            }
            "get_started" => {
                if ui.button(i18n::t("get_started")).clicked() {
                    commands.push(Command::OpenOnboarding);
                }
                ui.add_space(metrics::SPACE_12);
            }
            "members_access" => {
                if ui
                    .add(egui::Button::new(i18n::t("invite_members")))
                    .clicked()
                {
                    commands.push(Command::OpenShare);
                }
                ui.add_space(metrics::SPACE_12);
            }
            "data_export" => {
                ui.label(
                    egui::RichText::new(i18n::t("export_includes"))
                        .size(metrics::FONT_CAPTION)
                        .color(palette.muted),
                );
                ui.horizontal(|ui| {
                    if ui.button(i18n::t("export_workspace")).clicked() {
                        commands.push(Command::ExportWorkspace);
                    }
                    if ui.button(i18n::t("export_members")).clicked() {
                        commands.push(Command::ExportMembers);
                    }
                });
                if let Some(status) = &view.settings.status {
                    ui.label(
                        egui::RichText::new(status)
                            .size(metrics::FONT_MICRO)
                            .color(palette.accent),
                    );
                }
                ui.add_space(metrics::SPACE_10);
            }
            _ => {}
        }
    }
}
