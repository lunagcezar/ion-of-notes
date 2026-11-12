//! Settings page: ai.

use crate::ui::constants as metrics;
use egui::{Color32, CornerRadius, Margin, Sense, Vec2};

use super::super::super::commands::Command;
use super::super::super::palette::Palette;
use super::super::super::settings_search::SettingsRow;
use super::super::super::view::ViewState;
use super::section_heading;
pub(in crate::ui::dialogs) fn render_ai_settings(
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
            "connection" => {
                let checking = view.ai_checking;
                let label = if checking {
                    "Checking…".to_string()
                } else {
                    view.ai_status.clone()
                };
                ui.horizontal(|ui| {
                    let dot_color = if view.ai_connected && !checking {
                        palette.accent
                    } else {
                        palette.muted
                    };
                    let (dot, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                    ui.painter().circle_filled(dot.center(), 4.0, dot_color);
                    ui.label(
                        egui::RichText::new(label)
                            .size(metrics::FONT_CAPTION)
                            .color(if view.ai_connected && !checking {
                                palette.text
                            } else {
                                palette.muted
                            }),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new("Refresh")).clicked() {
                            commands.push(Command::RefreshAiStatus);
                        }
                    });
                });
                ui.add_space(metrics::SECTION_GAP);
            }
            "endpoint" => {
                ui.label(
                    egui::RichText::new("Service endpoint (host:port on your network)")
                        .size(metrics::FONT_CAPTION)
                        .strong()
                        .color(palette.text),
                );
                ui.add_space(metrics::SPACE_6);
                let mut endpoint = view.ai_endpoint.clone();
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut endpoint)
                            .desired_width(f32::INFINITY)
                            .text_color(palette.text),
                    )
                    .changed()
                {
                    commands.push(Command::SetAiEndpoint(endpoint));
                }
                ui.add_space(metrics::SPACE_4);
                ui.label(
                    egui::RichText::new(
                        "Start Ollama (`ollama serve`) and point this at its port, or leave the default loopback. OLLAMA_HOST also works.",
                    )
                    .size(metrics::FONT_MICRO)
                    .color(palette.muted),
                );
                ui.add_space(metrics::SPACE_16);
            }
            "model" => {
                ui.label(
                    egui::RichText::new("Model")
                        .size(metrics::FONT_CAPTION)
                        .strong()
                        .color(palette.text),
                );
                ui.add_space(metrics::SPACE_6);
                let mut model = view.ai_model.clone();
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut model)
                            .desired_width(f32::INFINITY)
                            .text_color(palette.text),
                    )
                    .changed()
                {
                    commands.push(Command::SetAiModel(model));
                }
                ui.add_space(metrics::SPACE_16);
            }
            "prefer_cli" => {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(row.title)
                            .size(metrics::FONT_CAPTION)
                            .color(palette.text),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let label = if view.ai_prefer_cli { "On" } else { "Off" };
                        if ui.add(egui::Button::new(label)).clicked() {
                            commands.push(Command::ToggleAiCli);
                        }
                    });
                });
                ui.add_space(metrics::SPACE_22);
            }
            "try_it" => {
                if ui.add(egui::Button::new("Ask a test question")).clicked() {
                    commands.push(Command::AskAi(
                        "Say hello in one short sentence.".to_string(),
                    ));
                }
                ui.add_space(metrics::SPACE_10);
                if view.ai_busy {
                    ui.label(
                        egui::RichText::new("Waiting for Ollama…")
                            .size(metrics::FONT_CAPTION)
                            .color(palette.accent),
                    );
                } else if let Some(answer) = &view.ai_answer {
                    egui::Frame::NONE
                        .fill(palette.callout)
                        .corner_radius(CornerRadius::same(6))
                        .inner_margin(Margin::same(10))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(answer)
                                    .size(metrics::FONT_CAPTION)
                                    .color(palette.text),
                            );
                        });
                } else if let Some(error) = &view.ai_error {
                    ui.label(
                        egui::RichText::new(error)
                            .size(metrics::FONT_CAPTION)
                            .color(Color32::from_rgb(224, 108, 108)),
                    );
                }
                ui.add_space(metrics::SECTION_GAP);
            }
            "shortcut" => {
                ui.label(
                    egui::RichText::new(
                        "Shortcut: Shift+Ctrl+J opens the assistant. Ask anything from Search or ask.",
                    )
                    .size(metrics::FONT_MICRO)
                    .color(palette.muted),
                );
                ui.add_space(metrics::SPACE_10);
            }
            _ => {}
        }
    }
}
