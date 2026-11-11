// ---- Share, inbox and slash menu -------------------------------------------

//! Share popover and Inbox panel.

use crate::collab::{Access, Role};
use crate::i18n;
use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, Vec2};

use super::super::commands::Command;
use super::super::icons::{draw_inbox, draw_lock, draw_settings};
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
        .fixed_size(Vec2::new(460.0, 372.0))
        .fixed_pos(Pos2::new(screen.right() - 480.0, screen.top() + 82.0))
        .frame(popover_frame(palette))
        .show(ctx, |ui| {
            // Header: the two tabs on the left, the help hint and the close
            // control on the right (reference/share.png).
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(i18n::t("share"))
                        .font(strong_font(14.0))
                        .color(palette.text),
                );
                ui.add_space(14.0);
                ui.label(
                    egui::RichText::new(i18n::t("publish"))
                        .size(13.0)
                        .color(palette.muted),
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
                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new(i18n::t("learn_about_sharing"))
                            .size(11.0)
                            .color(palette.muted),
                    );
                });
            });
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(14.0);

            // Invite row: the field grows, the accent button keeps its width.
            let mut invite = view.invite_input.clone();
            ui.horizontal(|ui| {
                let width = (ui.available_width() - 78.0).max(220.0);
                let edit = ui.add(
                    egui::TextEdit::singleline(&mut invite)
                        .desired_width(width)
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

            // People with access: avatar, name, address and role.
            for member in view.share_members.iter() {
                let (row, _) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), 46.0), Sense::hover());
                let avatar = Rect::from_center_size(
                    Pos2::new(row.left() + 18.0, row.center().y),
                    Vec2::splat(28.0),
                );
                ui.painter()
                    .circle_filled(avatar.center(), 14.0, palette.hover);
                let initial = member
                    .name
                    .chars()
                    .next()
                    .map(|c| c.to_uppercase().to_string())
                    .unwrap_or_default();
                ui.painter().text(
                    avatar.center(),
                    Align2::CENTER_CENTER,
                    initial,
                    FontId::proportional(12.0),
                    palette.text,
                );
                ui.painter().text(
                    Pos2::new(row.left() + 44.0, row.center().y - 8.0),
                    Align2::LEFT_CENTER,
                    &member.name,
                    FontId::proportional(13.0),
                    palette.text,
                );
                ui.painter().text(
                    Pos2::new(row.left() + 44.0, row.center().y + 9.0),
                    Align2::LEFT_CENTER,
                    &member.email,
                    FontId::proportional(11.0),
                    palette.muted,
                );
                ui.painter().text(
                    Pos2::new(row.right() - 4.0, row.center().y),
                    Align2::RIGHT_CENTER,
                    format!("{} ⌄", role_label(member.role)),
                    FontId::proportional(12.0),
                    palette.muted,
                );
            }

            ui.add_space(10.0);
            section_label(ui, &i18n::t("general_access"), palette);
            ui.add_space(4.0);
            let (general, response) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 40.0), Sense::click());
            ui.painter().rect(
                general,
                CornerRadius::same(6),
                if response.hovered() {
                    palette.hover
                } else {
                    Color32::TRANSPARENT
                },
                Stroke::new(1.0, palette.border),
                egui::StrokeKind::Inside,
            );
            draw_lock(
                ui.painter(),
                Rect::from_center_size(
                    Pos2::new(general.left() + 20.0, general.center().y),
                    Vec2::splat(16.0),
                ),
                palette.muted,
            );
            ui.painter().text(
                Pos2::new(general.left() + 40.0, general.center().y),
                Align2::LEFT_CENTER,
                i18n::t("everyone_workspace"),
                FontId::proportional(13.0),
                palette.text,
            );
            ui.painter().text(
                Pos2::new(general.right() - 14.0, general.center().y),
                Align2::RIGHT_CENTER,
                format!("{} ⌄", access_label(view.share_general_access)),
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

            ui.add_space(14.0);
            ui.separator();
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                let (glyph, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                draw_settings(ui.painter(), glyph, palette.muted);
                ui.add_space(6.0);
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
                                    .size(12.5)
                                    .color(palette.text),
                            )
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::new(1.0, palette.border))
                            .corner_radius(CornerRadius::same(6)),
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
