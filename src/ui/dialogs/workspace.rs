//! Create/rename workspace dialog for the local workspace catalog (spec-058).

use egui::{Align2, CornerRadius, Vec2};

use super::super::commands::{Command, WorkspaceDialogMode};
use super::super::palette::{Palette, strong_font};
use super::super::view::ViewState;
use super::super::widgets::popover_frame;

pub(crate) fn render_workspace_dialog(
    ctx: &egui::Context,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let Some(mode) = view.workspace_dialog else {
        return;
    };
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        commands.push(Command::CloseWorkspaceDialog);
        return;
    }
    let title = match mode {
        WorkspaceDialogMode::Create => "Create workspace",
        WorkspaceDialogMode::Rename => "Rename workspace",
    };
    let action = match mode {
        WorkspaceDialogMode::Create => "Create",
        WorkspaceDialogMode::Rename => "Save",
    };
    let resp = egui::Window::new("workspace_dialog")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_size(Vec2::new(420.0, 224.0))
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .frame(popover_frame(palette))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(title)
                        .font(strong_font(18.0))
                        .color(palette.text),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("×").size(17.0).color(palette.muted),
                            )
                            .frame(false),
                        )
                        .clicked()
                    {
                        commands.push(Command::CloseWorkspaceDialog);
                    }
                });
            });
            ui.add_space(8.0);
            ui.label(egui::RichText::new("Name").size(11.0).color(palette.muted));
            let mut name = view.workspace_name.clone();
            if ui
                .add(
                    egui::TextEdit::singleline(&mut name)
                        .desired_width(f32::INFINITY)
                        .hint_text("Workspace name")
                        .text_color(palette.text),
                )
                .changed()
            {
                commands.push(Command::SetWorkspaceName(name));
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Icon").size(11.0).color(palette.muted));
                let mut icon = view.workspace_icon.clone();
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut icon)
                            .desired_width(56.0)
                            .text_color(palette.text),
                    )
                    .changed()
                {
                    commands.push(Command::SetWorkspaceIcon(icon));
                }
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("A short glyph or emoji")
                        .size(10.5)
                        .color(palette.muted),
                );
            });
            ui.add_space(18.0);
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new(action)
                                .size(12.5)
                                .color(egui::Color32::WHITE),
                        )
                        .fill(palette.accent)
                        .corner_radius(CornerRadius::same(6)),
                    )
                    .clicked()
                    && !view.workspace_name.trim().is_empty()
                {
                    commands.push(Command::ConfirmWorkspaceDialog);
                }
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Cancel")
                                .size(12.0)
                                .color(palette.muted),
                        )
                        .frame(false),
                    )
                    .clicked()
                {
                    commands.push(Command::CloseWorkspaceDialog);
                }
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
        commands.push(Command::CloseWorkspaceDialog);
    }
}
