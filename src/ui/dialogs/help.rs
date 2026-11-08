//! Keyboard shortcut sheet.

use crate::shortcuts::SHORTCUT_TABLE;
use egui::{Align2, FontId, Sense, Vec2};

use super::super::commands::Command;
use super::super::palette::{Palette, strong_font};
use super::super::view::ViewState;
use super::super::widgets::popover_frame;

pub(crate) fn render_shortcuts(
    ctx: &egui::Context,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let resp = egui::Window::new("shortcut_sheet")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_size(Vec2::new(560.0, 620.0))
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .frame(popover_frame(palette))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Keyboard shortcuts")
                        .font(strong_font(18.0))
                        .color(palette.text),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("×").size(16.0).color(palette.muted),
                            )
                            .frame(false),
                        )
                        .clicked()
                    {
                        commands.push(Command::CloseShortcuts);
                    }
                });
            });
            ui.add_space(6.0);
            ui.separator();
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for doc in SHORTCUT_TABLE {
                        let (rect, _) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), 38.0),
                            Sense::hover(),
                        );
                        ui.painter().text(
                            rect.min + Vec2::new(4.0, 11.0),
                            Align2::LEFT_CENTER,
                            doc.label,
                            FontId::proportional(12.5),
                            palette.text,
                        );
                        ui.painter().text(
                            rect.max + Vec2::new(-4.0, -11.0),
                            Align2::RIGHT_CENTER,
                            doc.keys,
                            FontId::proportional(12.0),
                            palette.accent,
                        );
                        ui.painter().text(
                            rect.min + Vec2::new(4.0, 28.0),
                            Align2::LEFT_CENTER,
                            doc.source,
                            FontId::proportional(10.0),
                            palette.muted,
                        );
                    }
                });
            let _ = view;
        });
    if let Some(inner) = resp
        && ctx.input(|i| i.pointer.any_click())
        && ctx.input(|i| {
            i.pointer
                .interact_pos()
                .is_some_and(|p| !inner.response.rect.contains(p))
        })
    {
        commands.push(Command::CloseShortcuts);
    }
}
