// ---- Share, inbox and slash menu -------------------------------------------

//! Share popover and Inbox panel.

use crate::collab::{Access, Role};
use crate::i18n;
use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, Vec2};

use super::super::commands::Command;
use super::super::icons::{draw_inbox, draw_person};
use super::super::palette::{Palette, strong_font};
use super::super::view::ViewState;
use super::super::widgets::{popover_frame, section_label};

fn role_label(role: Role) -> String {
    match role {
        Role::Owner => i18n::t("workspace_owner"),
        Role::Member => i18n::t("member"),
        Role::Guest => i18n::t("guest"),
    }
}

fn access_label(access: Access) -> String {
    match access {
        Access::Full => i18n::t("access_full"),
        Access::Edit => i18n::t("access_edit"),
        Access::View => i18n::t("access_view"),
        Access::None => i18n::t("access_none"),
    }
}

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
        .fixed_size(Vec2::new(480.0, 560.0))
        .fixed_pos(Pos2::new(screen.right() - 500.0, screen.top() + 82.0))
        .frame(popover_frame(palette))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add(
                    egui::Button::new(
                        egui::RichText::new(i18n::t("share"))
                            .size(13.0)
                            .strong()
                            .color(palette.text),
                    )
                    .frame(false),
                );
                ui.add(
                    egui::Button::new(
                        egui::RichText::new(i18n::t("publish"))
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
                });
            });
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(12.0);

            let mut invite = view.invite_input.clone();
            ui.horizontal(|ui| {
                let edit = ui.add(
                    egui::TextEdit::singleline(&mut invite)
                        .desired_width(300.0)
                        .hint_text(i18n::t("email_commas"))
                        .text_color(palette.text),
                );
                if edit.changed() {
                    commands.push(Command::SetInviteInput(invite));
                }
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new(i18n::t("send_invite"))
                                .size(12.5)
                                .color(Color32::WHITE),
                        )
                        .fill(palette.accent)
                        .corner_radius(CornerRadius::same(6)),
                    )
                    .clicked()
                {
                    commands.push(Command::SendInvites);
                }
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(i18n::t("role"))
                        .size(11.0)
                        .color(palette.muted),
                );
                for role in [Role::Guest, Role::Member] {
                    let active = view.invite_role == role;
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new(role_label(role))
                                    .size(11.0)
                                    .color(if active { palette.text } else { palette.muted }),
                            )
                            .fill(if active {
                                palette.selected
                            } else {
                                Color32::TRANSPARENT
                            })
                            .corner_radius(CornerRadius::same(5)),
                        )
                        .clicked()
                    {
                        commands.push(Command::SetInviteRole(role));
                    }
                }
            });
            ui.add_space(6.0);
            let mut message = view.invite_message.clone();
            let message_edit = ui.add(
                egui::TextEdit::multiline(&mut message)
                    .desired_rows(2)
                    .desired_width(f32::INFINITY)
                    .hint_text(i18n::t("invite_note"))
                    .text_color(palette.text),
            );
            if message_edit.changed() {
                commands.push(Command::SetInviteMessage(message));
            }
            if let Some(error) = &view.invite_error {
                ui.label(
                    egui::RichText::new(error)
                        .size(11.0)
                        .color(Color32::from_rgb(224, 108, 108)),
                );
            }
            if let Some(status) = &view.invite_status {
                ui.label(egui::RichText::new(status).size(11.0).color(palette.accent));
            }
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(8.0);

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(180.0)
                .show(ui, |ui| {
                    for member in view.share_members.iter() {
                        let (row, _) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), 44.0),
                            Sense::hover(),
                        );
                        draw_person(
                            ui.painter(),
                            Rect::from_min_size(row.min + Vec2::new(2.0, 12.0), Vec2::splat(20.0)),
                            palette.muted,
                        );
                        ui.painter().text(
                            row.min + Vec2::new(32.0, 16.0),
                            Align2::LEFT_CENTER,
                            &member.name,
                            FontId::proportional(13.0),
                            palette.text,
                        );
                        ui.painter().text(
                            row.min + Vec2::new(32.0, 32.0),
                            Align2::LEFT_CENTER,
                            &member.email,
                            FontId::proportional(11.0),
                            palette.muted,
                        );
                        ui.painter().text(
                            row.max + Vec2::new(-4.0, -22.0),
                            Align2::RIGHT_CENTER,
                            member.role.label(),
                            FontId::proportional(12.0),
                            palette.muted,
                        );
                    }
                });

            ui.add_space(8.0);
            section_label(ui, &i18n::t("general_access"), palette);
            let (general, response) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 32.0), Sense::click());
            if response.hovered() {
                ui.painter()
                    .rect_filled(general, CornerRadius::same(5), palette.hover);
            }
            ui.painter().text(
                general.min + Vec2::new(4.0, 16.0),
                Align2::LEFT_CENTER,
                i18n::t("everyone_workspace"),
                FontId::proportional(13.0),
                palette.text,
            );
            ui.painter().text(
                general.max + Vec2::new(-4.0, -16.0),
                Align2::RIGHT_CENTER,
                access_label(view.share_general_access),
                FontId::proportional(12.0),
                palette.muted,
            );
            if response.clicked() {
                let next = match view.share_general_access {
                    Access::Full => Access::Edit,
                    Access::Edit => Access::View,
                    Access::View => Access::None,
                    Access::None => Access::Full,
                };
                commands.push(Command::SetGeneralAccess(next));
            }
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(i18n::t("advanced"))
                        .size(13.0)
                        .color(palette.text),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new(i18n::t("copy_link"))
                                    .size(13.0)
                                    .color(palette.text),
                            )
                            .frame(false),
                        )
                        .clicked()
                    {
                        commands.push(Command::CopyLink);
                        commands.push(Command::CloseShare);
                    }
                });
            });
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
