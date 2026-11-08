// ---- Share, inbox and slash menu -------------------------------------------

//! Share popover and Inbox panel.

use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, Vec2};

use super::super::commands::Command;
use super::super::icons::{draw_inbox, draw_person};
use super::super::palette::{Palette, strong_font};
use super::super::view::ViewState;
use super::super::widgets::{popover_frame, section_label};

pub(crate) fn render_share(
    ctx: &egui::Context,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let screen = ctx.viewport_rect();
    let resp = egui::Window::new("share")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_size(Vec2::new(460.0, 340.0))
        .fixed_pos(Pos2::new(screen.right() - 480.0, screen.top() + 82.0))
        .frame(popover_frame(palette))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add(
                    egui::Button::new(
                        egui::RichText::new("Share")
                            .size(13.0)
                            .strong()
                            .color(palette.text),
                    )
                    .frame(false),
                );
                ui.add(
                    egui::Button::new(
                        egui::RichText::new("Publish")
                            .size(13.0)
                            .color(palette.muted),
                    )
                    .frame(false),
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
                        commands.push(Command::CloseShare);
                    }
                    ui.label(
                        egui::RichText::new("Learn about sharing")
                            .size(11.0)
                            .color(palette.accent),
                    );
                });
            });
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                let mut invite = String::new();
                ui.add(
                    egui::TextEdit::singleline(&mut invite)
                        .desired_width(280.0)
                        .hint_text("Email or group, separated by commas")
                        .text_color(palette.text),
                );
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Invite")
                                .size(13.0)
                                .color(Color32::WHITE),
                        )
                        .fill(palette.accent)
                        .corner_radius(CornerRadius::same(6)),
                    )
                    .clicked()
                {
                    commands.push(Command::CloseShare);
                }
            });
            ui.add_space(14.0);

            let (row, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 44.0), Sense::hover());
            draw_person(
                ui.painter(),
                Rect::from_min_size(row.min + Vec2::new(2.0, 12.0), Vec2::splat(20.0)),
                palette.muted,
            );
            ui.painter().text(
                row.min + Vec2::new(32.0, 16.0),
                Align2::LEFT_CENTER,
                "Local workspace (You)",
                FontId::proportional(13.0),
                palette.text,
            );
            ui.painter().text(
                row.min + Vec2::new(32.0, 32.0),
                Align2::LEFT_CENTER,
                "Owner",
                FontId::proportional(11.0),
                palette.muted,
            );
            ui.painter().text(
                row.max + Vec2::new(-4.0, -22.0),
                Align2::RIGHT_CENTER,
                "Full access",
                FontId::proportional(12.0),
                palette.muted,
            );

            ui.add_space(8.0);
            section_label(ui, "General access", palette);
            let (general, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 32.0), Sense::hover());
            ui.painter().text(
                general.min + Vec2::new(4.0, 16.0),
                Align2::LEFT_CENTER,
                "Everyone with the link",
                FontId::proportional(13.0),
                palette.text,
            );
            ui.painter().text(
                general.max + Vec2::new(-4.0, -16.0),
                Align2::RIGHT_CENTER,
                "Full access",
                FontId::proportional(12.0),
                palette.muted,
            );
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Advanced")
                        .size(13.0)
                        .color(palette.text),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Copy link")
                                    .size(13.0)
                                    .color(palette.text),
                            )
                            .frame(false),
                        )
                        .clicked()
                    {
                        commands.push(Command::CloseShare);
                    }
                });
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
        commands.push(Command::CloseShare);
    }
}

pub(crate) fn render_inbox(
    ctx: &egui::Context,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let screen = ctx.viewport_rect();
    let resp = egui::Window::new("inbox")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_size(Vec2::new(420.0, 600.0))
        .fixed_pos(Pos2::new(screen.left() + 266.0, screen.top() + 78.0))
        .frame(popover_frame(palette))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Inbox")
                        .font(strong_font(18.0))
                        .color(palette.text),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("«").size(16.0).color(palette.muted),
                            )
                            .frame(false),
                        )
                        .clicked()
                    {
                        commands.push(Command::CloseInbox);
                    }
                });
            });
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(16.0);
            ui.label(
                egui::RichText::new("Last week")
                    .size(12.0)
                    .color(palette.muted),
            );
            ui.add_space(12.0);

            for (title, day) in [
                ("Your workspace export is ready", "Oct 30"),
                ("Your workspace export is ready", "Oct 30"),
            ] {
                let (row, _) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), 96.0), Sense::hover());
                draw_inbox(
                    ui.painter(),
                    Rect::from_min_size(row.min + Vec2::new(2.0, 6.0), Vec2::splat(18.0)),
                    palette.accent,
                );
                ui.painter().text(
                    row.min + Vec2::new(30.0, 14.0),
                    Align2::LEFT_CENTER,
                    title,
                    FontId::proportional(13.0),
                    palette.text,
                );
                ui.painter().text(
                    row.min + Vec2::new(30.0, 34.0),
                    Align2::LEFT_CENTER,
                    "The download link will expire in 7 days.",
                    FontId::proportional(11.5),
                    palette.muted,
                );
                ui.painter().text(
                    row.max + Vec2::new(-6.0, -82.0),
                    Align2::RIGHT_CENTER,
                    day,
                    FontId::proportional(11.0),
                    palette.muted,
                );
                ui.painter()
                    .circle_filled(row.max + Vec2::new(-6.0, -66.0), 3.5, palette.accent);
                let button_rect =
                    Rect::from_min_size(row.min + Vec2::new(30.0, 52.0), Vec2::new(84.0, 26.0));
                ui.painter().rect_stroke(
                    button_rect,
                    CornerRadius::same(5),
                    Stroke::new(1.0, palette.border),
                    egui::StrokeKind::Inside,
                );
                ui.painter().text(
                    button_rect.center(),
                    Align2::CENTER_CENTER,
                    "Download",
                    FontId::proportional(12.0),
                    palette.text,
                );
                ui.painter()
                    .hline(row.x_range(), row.max.y, Stroke::new(1.0, palette.border));
            }
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
        commands.push(Command::CloseInbox);
    }
}
