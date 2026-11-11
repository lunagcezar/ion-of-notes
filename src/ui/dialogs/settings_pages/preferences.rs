//! Settings page: preferences.

use crate::i18n;
use egui::{Align2, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, Vec2};

use super::super::super::commands::Command;
use super::super::super::palette::Palette;
use super::super::super::settings_search::SettingsRow;
use super::super::super::view::ViewState;
use super::{display_row, info_row, section_heading};
pub(in crate::ui::dialogs) fn render_preferences(
    ui: &mut egui::Ui,
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
            "theme" => {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(row.title).color(palette.text));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let label = if view.dark { "Dark" } else { "Light" };
                        if ui.add(egui::Button::new(label)).clicked() {
                            commands.push(Command::ToggleTheme);
                        }
                    });
                });
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(row.description)
                        .size(12.0)
                        .color(palette.muted),
                );
                ui.add_space(10.0);
            }
            "high_contrast" => display_row(
                ui,
                row.title,
                row.description,
                "Use system setting",
                palette,
            ),
            "enter_new_line" => display_row(ui, row.title, row.description, "Off", palette),
            "language" => {
                let (lang_rect, lang_resp) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), 46.0), Sense::click());
                if lang_resp.hovered() {
                    ui.painter()
                        .rect_filled(lang_rect, CornerRadius::same(6), palette.hover);
                }
                ui.painter().text(
                    lang_rect.min + Vec2::new(0.0, 14.0),
                    Align2::LEFT_CENTER,
                    i18n::t("language"),
                    FontId::proportional(12.5),
                    palette.text,
                );
                ui.painter().text(
                    lang_rect.min + Vec2::new(0.0, 32.0),
                    Align2::LEFT_CENTER,
                    i18n::t("language_description"),
                    FontId::proportional(11.0),
                    palette.muted,
                );
                let lang_label = crate::i18n::label_for(&view.language);
                let chip = Rect::from_min_size(
                    Pos2::new(lang_rect.max.x - 170.0, lang_rect.min.y + 9.0),
                    Vec2::new(170.0, 26.0),
                );
                ui.painter().rect_stroke(
                    chip,
                    CornerRadius::same(6),
                    Stroke::new(1.0, palette.border),
                    egui::StrokeKind::Inside,
                );
                ui.painter().text(
                    chip.center(),
                    Align2::CENTER_CENTER,
                    format!("{lang_label} ⌄"),
                    FontId::proportional(12.0),
                    palette.text,
                );
                if lang_resp.clicked() {
                    let codes: Vec<&str> = crate::i18n::SUPPORTED.iter().map(|(c, _)| *c).collect();
                    let idx = codes.iter().position(|c| *c == view.language).unwrap_or(0);
                    let next = codes[(idx + 1) % codes.len()];
                    commands.push(Command::SetLanguage(next.to_string()));
                }
            }
            "number_format" => display_row(ui, row.title, row.description, "Automatic", palette),
            "time_format" => display_row(ui, row.title, row.description, "24-hour", palette),
            "server" => {
                let state = if view.server_connected {
                    "connected"
                } else {
                    "local mode"
                };
                ui.label(
                    egui::RichText::new(format!("Self-hosted server  ·  {state}"))
                        .size(12.0)
                        .color(palette.accent),
                );
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(row.description)
                        .size(12.0)
                        .color(palette.muted),
                );
                ui.add_space(10.0);
            }
            "accounts" => info_row(ui, row.title, "1 owner · 2 guests", palette),
            "sharing" => info_row(
                ui,
                row.title,
                "Invite by email · full / edit / view",
                palette,
            ),
            "sync" => info_row(ui, row.title, "Versioned changes across devices", palette),
            "realtime" => info_row(ui, row.title, "Live co-editing over WebSocket", palette),
            "export" => info_row(ui, row.title, "Markdown tree · People CSV", palette),
            "license" => {
                ui.label(
                    egui::RichText::new(
                        "Open source under the MIT license. Run your own server; your data stays yours.",
                    )
                    .size(12.0)
                    .color(palette.muted),
                );
                ui.add_space(12.0);
            }
            "restore_demo" => {
                if ui
                    .add(egui::Button::new("Restore demo workspace"))
                    .clicked()
                {
                    commands.push(Command::ResetWorkspace);
                }
                ui.add_space(10.0);
            }
            _ => {}
        }
    }
}
