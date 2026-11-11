//! Page header, breadcrumbs and page body entry point.

use crate::i18n;
use crate::model::{BlockKind, Workspace};
use egui::{Align2, CornerRadius, FontFamily, FontId, Pos2, Rect, Sense, Stroke, Vec2};

use super::super::super::commands::{Command, FontStyle};
use super::super::super::format::format_timestamp;
use super::super::super::icons::{
    draw_ellipsis, draw_lines, draw_link, draw_page_icon, draw_pen, draw_plus, draw_share,
    draw_star, glyph_button, icon_button,
};
use super::super::super::palette::{Palette, strong_font};
use super::super::super::view::ViewState;
use super::super::database::render_database;
use super::super::library::render_library;
use super::blocks::render_block;
pub(crate) fn render_content(
    ui: &mut egui::Ui,
    ws: &mut Workspace,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    // Compact breadcrumb bar (spec-013).
    ui.vertical(|ui| {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.add_space(16.0);
            if !view.sidebar_open && ui.add(egui::Button::new(">").frame(false)).clicked() {
                commands.push(Command::ToggleSidebar);
            }
            let mut crumbs = crumb_chain(ws, &view.current_page);
            if crumbs.is_empty() {
                crumbs.push((String::new(), "Workspace".to_string()));
            }
            if crumbs.len() == 1 {
                crumbs.insert(0, (String::new(), ws.name.clone()));
            }
            if view.library_open {
                crumbs = vec![(String::new(), "Library".to_string())];
            }
            for (i, (icon, title)) in crumbs.iter().enumerate() {
                if i > 0 {
                    ui.label(egui::RichText::new("›").color(palette.muted));
                }
                crumb(ui, icon, title, palette);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.add_space(14.0);
                if glyph_button(ui, Vec2::new(26.0, 24.0), palette, draw_ellipsis)
                    .on_hover_text("Page settings and actions")
                    .clicked()
                {
                    commands.push(Command::OpenPageActions);
                }
                if !view.library_open
                    && glyph_button(ui, Vec2::new(26.0, 24.0), palette, draw_star).clicked()
                {
                    commands.push(Command::ToggleFavorite(view.current_page.clone()));
                }
                if glyph_button(ui, Vec2::new(26.0, 24.0), palette, draw_link).clicked() {
                    commands.push(Command::OpenShare);
                }
                let share = ui.add(
                    egui::Button::new(
                        egui::RichText::new(i18n::t("share"))
                            .size(13.0)
                            .color(palette.text),
                    )
                    .frame(false),
                );
                if share.clicked() {
                    commands.push(Command::OpenShare);
                }
                if glyph_button(ui, Vec2::new(20.0, 24.0), palette, draw_share).clicked() {
                    commands.push(Command::OpenShare);
                }
            });
        });
        ui.add_space(6.0);
    });
    ui.separator();

    let full_width = ui.available_width();
    if view.library_open {
        render_library(ui, ws, view, palette, commands);
    } else {
        let column_width = if view.full_width {
            (full_width - 96.0).max(360.0)
        } else {
            (full_width - 96.0).clamp(360.0, 708.0)
        };
        let gutter = 48.0;
        let pad = ((full_width - column_width) / 2.0 - gutter).max(0.0);
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    ui.add_space(pad);
                    ui.vertical(|ui| {
                        ui.set_width(column_width + gutter);
                        ui.add_space(88.0);
                        render_page(ui, ws, view, column_width, gutter, palette, commands);
                        ui.add_space(80.0);
                    });
                });
            });
    }

    // Floating controls on the right edge of the content, as in the pictures.
    let screen = ui.ctx().viewport_rect();
    let lines_center = Pos2::new(screen.right() - 42.0, screen.top() + 280.0);
    draw_lines(
        ui.painter(),
        Rect::from_center_size(lines_center, Vec2::splat(20.0)),
        palette.muted,
    );
    let button_center = Pos2::new(screen.right() - 44.0, screen.bottom() - 44.0);
    ui.painter()
        .circle_filled(button_center, 24.0, palette.hover);
    ui.painter()
        .circle_stroke(button_center, 24.0, Stroke::new(1.0, palette.border));
    draw_pen(
        ui.painter(),
        Rect::from_center_size(button_center, Vec2::splat(24.0)),
        palette.text,
    );
}

/// Titles and icons from the root page down to the current page. Keeping the
/// icons here means breadcrumbs show the same customization as the sidebar.
fn crumb_chain(ws: &Workspace, id: &str) -> Vec<(String, String)> {
    let mut chain = Vec::new();
    let mut cursor = Some(id.to_string());
    let mut guard = 0;
    while let Some(current) = cursor {
        guard += 1;
        if guard > 32 {
            break;
        }
        match ws.page(&current) {
            Some(page) => {
                chain.push((page.icon.clone(), page.title.clone()));
                cursor = page.parent.clone();
            }
            None => break,
        }
    }
    chain.reverse();
    chain
}

/// Body font for a block, honouring the Appearance & layout choices (spec-013).
pub(super) fn body_font(view: &ViewState, size: f32) -> FontId {
    let size = if view.small_text { size * 0.85 } else { size };
    match view.font_style {
        FontStyle::Default => FontId::proportional(size),
        FontStyle::Serif => FontId::new(size, FontFamily::Name("Serif".into())),
        FontStyle::Mono => FontId::monospace(size),
    }
}

fn crumb(ui: &mut egui::Ui, icon: &str, text: &str, palette: &Palette) -> egui::Response {
    let width = 16.0 + 8.0 + text.chars().count() as f32 * 7.0 + 10.0;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 20.0), Sense::click());
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(4), palette.hover);
    }
    draw_page_icon(
        ui.painter(),
        Rect::from_min_size(rect.min + Vec2::new(2.0, 2.0), Vec2::splat(16.0)),
        icon,
        palette,
        false,
    );
    ui.painter().text(
        rect.min + Vec2::new(24.0, 10.0),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(13.0),
        palette.text,
    );
    response
}

// ---- Page -----------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn render_page(
    ui: &mut egui::Ui,
    ws: &mut Workspace,
    view: &ViewState,
    width: f32,
    gutter: f32,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let Some(page) = ws.page_mut(&view.current_page) else {
        ui.label(
            egui::RichText::new("No page selected. Create one from the sidebar.")
                .color(palette.muted),
        );
        return;
    };

    // Big page icon and the very large bold title, aligned with the body text.
    // Clicking the icon opens the emoji / icon / upload picker (spec-013).
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.add_space(gutter);
        ui.vertical(|ui| {
            ui.set_width(width);
            let (icon_rect, icon_response) =
                ui.allocate_exact_size(Vec2::splat(72.0), Sense::click());
            draw_page_icon(ui.painter(), icon_rect, &page.icon, palette, true);
            if icon_response.hovered() {
                ui.painter().rect_stroke(
                    icon_rect,
                    CornerRadius::same(10),
                    Stroke::new(1.0, palette.border),
                    egui::StrokeKind::Inside,
                );
            }
            if icon_response.on_hover_text("Change icon").clicked() {
                commands.push(Command::OpenIconPicker);
            }
            ui.add_space(30.0);
            let _ = ui.add(
                egui::TextEdit::singleline(&mut page.title)
                    .font(strong_font(44.0))
                    .frame(egui::Frame::NONE)
                    .desired_width(width)
                    .text_color(palette.text)
                    .hint_text("Untitled"),
            );
            ui.add_space(16.0);
        });
    });

    if page.show_toc {
        let headings: Vec<(String, BlockKind)> = page
            .headings()
            .into_iter()
            .map(|block| (block.text.clone(), block.kind))
            .collect();
        ui.horizontal(|ui| {
            ui.add_space(gutter);
            ui.vertical(|ui| {
                ui.set_width(width);
                ui.label(
                    egui::RichText::new(i18n::t("table_of_contents"))
                        .size(12.0)
                        .strong()
                        .color(palette.muted),
                );
                if headings.is_empty() {
                    ui.label(
                        egui::RichText::new("Add a heading to build the table of contents.")
                            .size(12.0)
                            .color(palette.muted),
                    );
                }
                for (text, kind) in &headings {
                    let indent = match kind {
                        BlockKind::Heading2 => 12.0,
                        BlockKind::Heading3 => 24.0,
                        BlockKind::Heading4 => 36.0,
                        _ => 0.0,
                    };
                    ui.horizontal(|ui| {
                        ui.add_space(indent);
                        ui.label(egui::RichText::new(text).size(12.5).color(palette.accent));
                    });
                }
            });
        });
        ui.add_space(20.0);
    }

    let mut numbered: usize = 0;
    let block_count = page.blocks.len();
    for index in 0..page.blocks.len() {
        if page.blocks[index].kind.is_heading() {
            ui.add_space(22.0);
        }
        let block = &mut page.blocks[index];
        render_block(
            ui,
            block,
            index,
            &mut numbered,
            width,
            gutter,
            view,
            palette,
            commands,
            block_count,
        );
        ui.add_space(14.0);
    }

    // Subtle add-block row.
    ui.horizontal(|ui| {
        ui.add_space(gutter);
        if icon_button(ui, 22.0, palette, draw_plus).clicked() {
            commands.push(Command::AppendBlock);
        }
        ui.label(
            egui::RichText::new("Click to add a block")
                .size(13.0)
                .color(palette.muted),
        );
    });

    if page.discussions {
        ui.add_space(26.0);
        ui.horizontal(|ui| {
            ui.add_space(gutter);
            ui.vertical(|ui| {
                ui.set_width(width);
                ui.label(
                    egui::RichText::new(i18n::t("page_discussions"))
                        .font(strong_font(18.0))
                        .color(palette.text),
                );
                ui.add_space(6.0);
                if page.comments.is_empty() {
                    ui.label(
                        egui::RichText::new(i18n::t("no_comments"))
                            .size(12.0)
                            .color(palette.muted),
                    );
                }
                for comment in &page.comments {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(&comment.author)
                                .strong()
                                .color(palette.text),
                        );
                        ui.label(
                            egui::RichText::new(format_timestamp(
                                &comment.created_at,
                                view.time_format,
                            ))
                            .size(10.5)
                            .color(palette.muted),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button(i18n::t("delete")).clicked() {
                                commands.push(Command::DeletePageComment(comment.id.clone()));
                            }
                        });
                    });
                    ui.label(egui::RichText::new(&comment.text).color(palette.text));
                    ui.add_space(6.0);
                }
                let mut input = view.comment_input.clone();
                let edit = ui.add(
                    egui::TextEdit::multiline(&mut input)
                        .desired_rows(2)
                        .desired_width(width)
                        .hint_text(i18n::t("add_comment"))
                        .text_color(palette.text),
                );
                if edit.changed() {
                    commands.push(Command::SetCommentInput(input));
                }
                if ui.button(i18n::t("comment")).clicked() {
                    commands.push(Command::AddPageComment);
                }
                if page.inline_comments {
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(i18n::t("inline_note"))
                            .size(11.0)
                            .color(palette.muted),
                    );
                }
            });
        });
    }

    if !page.rows.is_empty() || page.title == "Tasks" {
        ui.add_space(26.0);
        render_database(ui, page, view, width, gutter, palette, commands);
    }
}
