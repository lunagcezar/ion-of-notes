//! Page actions menu and the version-history window.
//!
//! Both surfaces belong together: version history is opened from the page
//! actions menu and restores through the same `Command` path.

use crate::model::{Page, Workspace};
use egui::{Align2, Color32, CornerRadius, FontId, Margin, Pos2, Sense, Stroke, Vec2};

use super::super::commands::{Command, FontStyle};
use super::super::palette::{Palette, strong_font};
use super::super::view::ViewState;
use super::super::widgets::{menu_button, toggle_button};

fn action_items(view: &ViewState) -> Vec<(&'static str, &'static str, Command)> {
    vec![
        ("Copy link", "Ctrl+L", Command::CopyLink),
        ("Copy page contents", "", Command::ClosePageActions),
        ("Duplicate", "Ctrl+D", Command::DuplicatePage),
        ("Move to", "Ctrl+Shift+P", Command::ClosePageActions),
        ("Version history", "", Command::OpenVersionHistory),
        ("Updates & analytics", "", Command::ClosePageActions),
        ("Suggest edits", "", Command::ClosePageActions),
        ("Use with AI", "", Command::OpenAiChat),
        ("Notify me", "Comments", Command::ClosePageActions),
        ("Archive", "", Command::ClosePageActions),
        (
            "Move to Trash",
            "",
            Command::DeletePage(view.current_page.clone()),
        ),
        ("Show / hide sidebar", "Ctrl+\\", Command::ToggleSidebar),
        ("More", "", Command::ClosePageActions),
    ]
}

pub(crate) fn render_page_actions(
    ctx: &egui::Context,
    ws: &Workspace,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let screen = ctx.viewport_rect();
    let resp = egui::Window::new("page_actions")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_size(Vec2::new(300.0, 620.0))
        .fixed_pos(Pos2::new(screen.right() - 316.0, screen.top() + 76.0))
        .frame(
            egui::Frame::NONE
                .fill(palette.content)
                .corner_radius(CornerRadius::same(10))
                .inner_margin(Margin::same(12))
                .stroke(Stroke::new(1.0, palette.border)),
        )
        .show(ctx, |ui| {
            // Search actions field (filters the list below).
            let id = egui::Id::unique("page_actions_query");
            let mut query: String = ctx.data(|d| d.get_temp(id).unwrap_or_default());
            let field = ui.add(
                egui::TextEdit::singleline(&mut query)
                    .desired_width(f32::INFINITY)
                    .hint_text("Search actions...")
                    .text_color(palette.text),
            );
            if field.changed() {
                ctx.data_mut(|d| d.insert_temp(id, query.clone()));
            }
            ui.add_space(8.0);

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let q = query.trim().to_lowercase();
                    for (label, hint, command) in action_items(view) {
                        if !q.is_empty() && !label.to_lowercase().contains(&q) {
                            continue;
                        }
                        if menu_button(ui, label, hint, palette).clicked() {
                            commands.push(command);
                            commands.push(Command::ClosePageActions);
                        }
                    }
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(6.0);

                    if q.is_empty() {
                        // Appearance & layout.
                        ui.label(
                            egui::RichText::new("Appearance & layout")
                                .size(11.0)
                                .color(palette.muted),
                        );
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            for (label, style) in [
                                ("Default", FontStyle::Default),
                                ("Serif", FontStyle::Serif),
                                ("Mono", FontStyle::Mono),
                            ] {
                                let active = view.font_style == style;
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new(label).size(12.0).color(
                                                if active { palette.text } else { palette.muted },
                                            ),
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
                                    commands.push(Command::SetFontStyle(style));
                                }
                            }
                        });
                        ui.add_space(4.0);
                        if toggle_button(ui, "Small text", view.small_text, palette).clicked() {
                            commands.push(Command::ToggleSmallText);
                        }
                        if toggle_button(ui, "Full width", view.full_width, palette).clicked() {
                            commands.push(Command::ToggleFullWidth);
                        }
                        if toggle_button(ui, "Available offline", view.available_offline, palette)
                            .clicked()
                        {
                            commands.push(Command::ToggleAvailableOffline);
                        }
                        if toggle_button(ui, "Lock page", view.lock_page, palette).clicked() {
                            commands.push(Command::ToggleLockPage);
                        }
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(6.0);

                        // Last edited footer.
                        let words = ws
                            .page(&view.current_page)
                            .map(|p| p.plain_text().split_whitespace().count())
                            .unwrap_or(0);
                        ui.label(
                            egui::RichText::new("Last edited")
                                .size(11.0)
                                .color(palette.muted),
                        );
                        ui.add_space(2.0);
                        ui.label(
                            egui::RichText::new("Local workspace · just now")
                                .size(11.0)
                                .color(palette.text),
                        );
                        ui.label(
                            egui::RichText::new(format!("{words} words"))
                                .size(10.5)
                                .color(palette.muted),
                        );
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
        commands.push(Command::ClosePageActions);
    }
}

// ---- Version history (spec-015) -------------------------------------------

/// A short human-readable preview of a stored page snapshot.
fn snapshot_preview(snapshot: &str) -> String {
    match serde_json::from_str::<Page>(snapshot) {
        Ok(page) => {
            let body = page.plain_text();
            let body = if body.chars().count() > 220 {
                format!("{}…", body.chars().take(220).collect::<String>())
            } else {
                body
            };
            if body.is_empty() {
                page.title
            } else {
                format!("{}\n\n{}", page.title, body)
            }
        }
        Err(_) => snapshot.chars().take(220).collect(),
    }
}

pub(crate) fn render_version_history(
    ctx: &egui::Context,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let resp = egui::Window::new("version_history")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_size(Vec2::new(560.0, 560.0))
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .frame(
            egui::Frame::NONE
                .fill(palette.content)
                .corner_radius(CornerRadius::same(10))
                .inner_margin(Margin::same(16))
                .stroke(Stroke::new(1.0, palette.border)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Version history")
                        .font(strong_font(20.0))
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
                        commands.push(Command::CloseVersionHistory);
                    }
                });
            });
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            if view.versions.is_empty() {
                ui.label(
                    egui::RichText::new("No versions have been recorded for this page yet.")
                        .size(13.0)
                        .color(palette.muted),
                );
                return;
            }

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(250.0)
                .show(ui, |ui| {
                    for (index, version) in view.versions.iter().enumerate() {
                        let selected = index == view.version_selected;
                        let (rect, response) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), 46.0),
                            Sense::click(),
                        );
                        if selected || response.hovered() {
                            ui.painter().rect_filled(
                                rect,
                                CornerRadius::same(6),
                                if selected {
                                    palette.selected
                                } else {
                                    palette.hover
                                },
                            );
                        }
                        ui.painter().text(
                            rect.min + Vec2::new(10.0, 14.0),
                            Align2::LEFT_CENTER,
                            format!("Version {} · {}", version.version, version.title),
                            FontId::proportional(13.0),
                            palette.text,
                        );
                        ui.painter().text(
                            rect.min + Vec2::new(10.0, 32.0),
                            Align2::LEFT_CENTER,
                            format!("{} · {}", version.actor, version.updated_at),
                            FontId::proportional(11.0),
                            palette.muted,
                        );
                        if response.clicked() {
                            commands.push(Command::SetVersionSelected(index));
                        }
                    }
                });

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(8.0);
            if let Some(version) = view.versions.get(view.version_selected) {
                ui.label(
                    egui::RichText::new("Snapshot preview")
                        .size(12.0)
                        .strong()
                        .color(palette.text),
                );
                ui.add_space(4.0);
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .max_height(120.0)
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(snapshot_preview(&version.snapshot))
                                .size(12.0)
                                .color(palette.muted),
                        );
                    });
                ui.add_space(8.0);
                if ui.add(egui::Button::new("Restore this version")).clicked() {
                    commands.push(Command::RestoreVersion(view.version_selected));
                }
            }
        });

    if let Some(inner) = resp
        && ctx.input(|i| i.pointer.any_click())
        && ctx.input(|i| {
            i.pointer
                .interact_pos()
                .is_some_and(|p| !inner.response.rect.contains(p))
        })
    {
        commands.push(Command::CloseVersionHistory);
    }
}
