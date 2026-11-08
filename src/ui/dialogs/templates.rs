//! Starter template library (spec-027).

use egui::{Align2, CornerRadius, FontId, Margin, Sense, Vec2};

use crate::i18n;
use crate::templates;

use super::super::commands::Command;
use super::super::palette::{Palette, strong_font};
use super::super::view::ViewState;
use super::super::widgets::popover_frame;

pub(crate) fn render_templates(
    ctx: &egui::Context,
    _view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    egui::Window::new("template_library")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_size(Vec2::new(720.0, 600.0))
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .frame(popover_frame(palette))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(i18n::t("template_library"))
                        .font(strong_font(22.0))
                        .color(palette.text),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("×").size(18.0).color(palette.muted),
                            )
                            .frame(false),
                        )
                        .clicked()
                    {
                        commands.push(Command::CloseTemplates);
                    }
                });
            });
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for template in templates::catalog() {
                        egui::Frame::NONE
                            .fill(palette.callout)
                            .corner_radius(CornerRadius::same(8))
                            .inner_margin(Margin::same(12))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let (icon_rect, _) =
                                        ui.allocate_exact_size(Vec2::splat(28.0), Sense::hover());
                                    ui.painter().text(
                                        icon_rect.center(),
                                        Align2::CENTER_CENTER,
                                        template.icon,
                                        FontId::proportional(20.0),
                                        palette.muted,
                                    );
                                    ui.vertical(|ui| {
                                        ui.label(
                                            egui::RichText::new(template.name)
                                                .font(strong_font(15.0))
                                                .color(palette.text),
                                        );
                                        ui.label(
                                            egui::RichText::new(template.description)
                                                .size(12.0)
                                                .color(palette.muted),
                                        );
                                    });
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui
                                                .add(
                                                    egui::Button::new(
                                                        egui::RichText::new(i18n::t(
                                                            "use_template",
                                                        ))
                                                        .size(12.0)
                                                        .color(egui::Color32::WHITE),
                                                    )
                                                    .fill(palette.accent)
                                                    .corner_radius(CornerRadius::same(5)),
                                                )
                                                .clicked()
                                            {
                                                commands.push(Command::ApplyTemplate(
                                                    template.id.to_string(),
                                                ));
                                            }
                                        },
                                    );
                                });
                            });
                        ui.add_space(8.0);
                    }
                });
        });
}
