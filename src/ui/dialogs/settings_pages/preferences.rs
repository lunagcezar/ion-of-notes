//! Settings page: preferences.

use crate::i18n;
use crate::ui::constants as metrics;
use egui::{Align2, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, Vec2};

use super::super::super::commands::Command;
use super::super::super::palette::Palette;
use super::super::super::settings_search::SettingsRow;
use super::super::super::view::{ServerView, SettingsView};
use super::{display_row, info_row, section_heading};
fn format_label(format: crate::model::NumberFormat) -> &'static str {
    match format {
        crate::model::NumberFormat::Automatic => "Automatic",
        crate::model::NumberFormat::DotThousands => "1.234,5",
        crate::model::NumberFormat::CommaThousands => "1,234.5",
    }
}

fn time_label(format: crate::model::TimeFormat) -> &'static str {
    match format {
        crate::model::TimeFormat::Hour24 => "24-hour",
        crate::model::TimeFormat::Hour12 => "12-hour",
    }
}

/// A preference row whose value chip cycles on click. Shared by the number
/// and time format rows so their layout matches the language row.
fn choice_row(
    ui: &mut egui::Ui,
    title: &str,
    description: &str,
    value: &str,
    palette: &Palette,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), metrics::SPACE_46),
        Sense::click(),
    );
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), palette.hover);
    }
    ui.painter().text(
        rect.min + Vec2::new(0.0, metrics::SPACE_14),
        Align2::LEFT_CENTER,
        title,
        FontId::proportional(12.5),
        palette.text,
    );
    ui.painter().text(
        rect.min + Vec2::new(0.0, metrics::SPACE_32),
        Align2::LEFT_CENTER,
        description,
        FontId::proportional(11.0),
        palette.muted,
    );
    let chip = Rect::from_min_size(
        Pos2::new(
            rect.max.x - metrics::CHIP_WIDTH,
            rect.min.y + metrics::SPACE_8,
        ),
        Vec2::new(metrics::CHIP_WIDTH, metrics::CHIP_HEIGHT),
    );
    ui.painter().rect_stroke(
        chip,
        CornerRadius::same(6),
        Stroke::new(metrics::STROKE_HAIRLINE, palette.border),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        chip.center(),
        Align2::CENTER_CENTER,
        format!("{value} ⌄"),
        FontId::proportional(12.0),
        palette.text,
    );
    response.clicked()
}

pub(in crate::ui::dialogs) fn render_preferences(
    ui: &mut egui::Ui,
    settings: &SettingsView,
    server: &ServerView,
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
                        let label = if settings.dark { "Dark" } else { "Light" };
                        if ui.add(egui::Button::new(label)).clicked() {
                            commands.push(Command::ToggleTheme);
                        }
                    });
                });
                ui.add_space(metrics::SPACE_6);
                ui.label(
                    egui::RichText::new(row.description)
                        .size(metrics::FONT_CAPTION)
                        .color(palette.muted),
                );
                ui.add_space(metrics::SPACE_10);
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
                let (lang_rect, lang_resp) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), metrics::SPACE_46),
                    Sense::click(),
                );
                if lang_resp.hovered() {
                    ui.painter()
                        .rect_filled(lang_rect, CornerRadius::same(6), palette.hover);
                }
                ui.painter().text(
                    lang_rect.min + Vec2::new(0.0, metrics::SPACE_14),
                    Align2::LEFT_CENTER,
                    i18n::t("language"),
                    FontId::proportional(12.5),
                    palette.text,
                );
                ui.painter().text(
                    lang_rect.min + Vec2::new(0.0, metrics::SPACE_32),
                    Align2::LEFT_CENTER,
                    i18n::t("language_description"),
                    FontId::proportional(11.0),
                    palette.muted,
                );
                let lang_label = crate::i18n::label_for(&settings.language);
                let chip = Rect::from_min_size(
                    Pos2::new(
                        lang_rect.max.x - metrics::LANGUAGE_CHIP_WIDTH,
                        lang_rect.min.y + metrics::SPACE_8,
                    ),
                    Vec2::new(metrics::LANGUAGE_CHIP_WIDTH, metrics::CHIP_HEIGHT),
                );
                ui.painter().rect_stroke(
                    chip,
                    CornerRadius::same(6),
                    Stroke::new(metrics::STROKE_HAIRLINE, palette.border),
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
                    let idx = codes
                        .iter()
                        .position(|c| *c == settings.language)
                        .unwrap_or(0);
                    let next = codes[(idx + 1) % codes.len()];
                    commands.push(Command::SetLanguage(next.to_string()));
                }
            }
            "number_format" => {
                if choice_row(
                    ui,
                    row.title,
                    row.description,
                    format_label(settings.number_format),
                    palette,
                ) {
                    let next = match settings.number_format {
                        crate::model::NumberFormat::Automatic => {
                            crate::model::NumberFormat::DotThousands
                        }
                        crate::model::NumberFormat::DotThousands => {
                            crate::model::NumberFormat::CommaThousands
                        }
                        crate::model::NumberFormat::CommaThousands => {
                            crate::model::NumberFormat::Automatic
                        }
                    };
                    commands.push(Command::SetNumberFormat(next));
                }
            }
            "time_format" => {
                if choice_row(
                    ui,
                    row.title,
                    row.description,
                    time_label(settings.time_format),
                    palette,
                ) {
                    let next = match settings.time_format {
                        crate::model::TimeFormat::Hour24 => crate::model::TimeFormat::Hour12,
                        crate::model::TimeFormat::Hour12 => crate::model::TimeFormat::Hour24,
                    };
                    commands.push(Command::SetTimeFormat(next));
                }
            }
            "server" => {
                let state = if server.connected {
                    "connected"
                } else {
                    "local mode"
                };
                ui.label(
                    egui::RichText::new(format!("Self-hosted server  ·  {state}"))
                        .size(metrics::FONT_CAPTION)
                        .color(palette.accent),
                );
                ui.add_space(metrics::SPACE_6);
                ui.label(
                    egui::RichText::new(row.description)
                        .size(metrics::FONT_CAPTION)
                        .color(palette.muted),
                );
                ui.add_space(metrics::SPACE_10);
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
                    .size(metrics::FONT_CAPTION)
                    .color(palette.muted),
                );
                ui.add_space(metrics::SPACE_12);
            }
            "restore_demo" => {
                if ui
                    .add(egui::Button::new("Restore demo workspace"))
                    .clicked()
                {
                    commands.push(Command::ResetWorkspace);
                }
                ui.add_space(metrics::SPACE_10);
            }
            _ => {}
        }
    }
}
