//! Settings page: offline.

use crate::i18n;
use crate::model::Workspace;

use super::super::super::format::format_timestamp;
use super::super::super::palette::Palette;
use super::super::super::settings_search::SettingsRow;
use super::super::super::widgets::toggle_button;
use super::section_heading;
pub(in crate::ui::dialogs) fn render_offline_settings(
    ui: &mut egui::Ui,
    ws: &mut Workspace,
    palette: &Palette,
    rows: &[&SettingsRow],
) {
    let mut section = "";
    for row in rows {
        if row.section != section {
            section = row.section;
            section_heading(ui, section, palette);
        }
        match row.id {
            "auto_download" => {
                if toggle_button(ui, row.title, ws.settings.offline_auto_download, palette)
                    .clicked()
                {
                    ws.settings.offline_auto_download = !ws.settings.offline_auto_download;
                }
                ui.add_space(14.0);
            }
            "offline_pages" => {
                let search_id = egui::Id::unique("offline_search");
                let mut query: String = ui
                    .ctx()
                    .data(|data| data.get_temp(search_id).unwrap_or_default());
                let edit = ui.add(
                    egui::TextEdit::singleline(&mut query)
                        .desired_width(280.0)
                        .hint_text(i18n::t("search_offline"))
                        .text_color(palette.text),
                );
                if edit.changed() {
                    ui.ctx()
                        .data_mut(|data| data.insert_temp(search_id, query.clone()));
                }
                let query = query.trim().to_lowercase();
                let mut found = false;
                for page in ws.pages.iter().filter(|page| {
                    page.available_offline && page.title.to_lowercase().contains(&query)
                }) {
                    found = true;
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&page.title).color(palette.text));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format_timestamp(
                                    &page.updated_at,
                                    ws.settings.time_format,
                                ))
                                .size(11.0)
                                .color(palette.muted),
                            );
                        });
                    });
                }
                if !found {
                    ui.label(
                        egui::RichText::new(i18n::t("no_offline"))
                            .size(12.0)
                            .color(palette.muted),
                    );
                }
            }
            _ => {}
        }
    }
}
