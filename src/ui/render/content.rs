//! Content area: breadcrumbs, page header, editable blocks and the floating
//! controls.

use crate::i18n;
use crate::model::{Block, BlockKind, Workspace};
use crate::server::realtime::Awareness;
use egui::{
    Align2, Color32, CornerRadius, FontFamily, FontId, Margin, Pos2, Rect, Sense, Stroke, Vec2,
};

use super::super::commands::{Command, FontStyle};
use super::super::icons::{
    draw_bullet_dot, draw_dots, draw_ellipsis, draw_lines, draw_link, draw_page, draw_page_icon,
    draw_pen, draw_plus, draw_share, draw_star, glyph_button, icon_button,
};
use super::super::palette::{Palette, strong_font};
use super::super::view::ViewState;
use super::database::render_database;
use super::library::render_library;

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
                if glyph_button(ui, Vec2::new(26.0, 24.0), palette, draw_star).clicked() {
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
fn body_font(view: &ViewState, size: f32) -> FontId {
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
                            egui::RichText::new(&comment.created_at)
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

/// Returns true if backspace-on-empty was detected (caller should delete).
fn backspace_on_empty(ui: &egui::Ui, response: &egui::Response, text: &str) -> bool {
    response.has_focus() && text.is_empty() && ui.input(|i| i.key_pressed(egui::Key::Backspace))
}

#[allow(clippy::too_many_arguments)]
fn render_block(
    ui: &mut egui::Ui,
    block: &mut Block,
    index: usize,
    numbered: &mut usize,
    width: f32,
    gutter: f32,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
    block_count: usize,
) {
    let row_min = ui.available_rect_before_wrap().min;
    let row_rect = Rect::from_min_size(row_min, Vec2::new(width + gutter, 30.0));
    let hover = ui
        .input(|i| i.pointer.hover_pos())
        .map(|p| row_rect.contains(p))
        .unwrap_or(false);
    let menu_pos = Pos2::new(row_rect.min.x + 60.0, row_rect.min.y);

    // While the six-dot handle is being dragged, the block under the pointer
    // becomes the reorder target; draw an insertion marker at its lower edge.
    if view.drag_from.is_some()
        && let Some(p) = ui.ctx().pointer_interact_pos()
        && row_rect.contains(p)
    {
        commands.push(Command::HoverBlockDrag(block.id.clone()));
    }
    if view.drag_to.as_deref() == Some(block.id.as_str()) {
        ui.painter().hline(
            row_rect.x_range(),
            row_rect.max.y,
            Stroke::new(2.0, palette.accent),
        );
    }

    let mut delete_empty = false;
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        // Keep layout stable regardless of hover to avoid stealing TextEdit focus.
        let plus_rect = ui
            .allocate_exact_size(Vec2::new(22.0, 22.0), Sense::hover())
            .0;
        let handle_rect = ui
            .allocate_exact_size(Vec2::new(22.0, 22.0), Sense::hover())
            .0;
        ui.add_space(4.0);
        if hover && !view.lock_page {
            let plus = ui
                .interact(
                    plus_rect,
                    ui.make_persistent_id(("block_plus", &block.id)),
                    Sense::click(),
                )
                .on_hover_text("Click to add a block below · Alt-click to add above");
            let handle = ui
                .interact(
                    handle_rect,
                    ui.make_persistent_id(("block_handle", &block.id)),
                    Sense::click_and_drag(),
                )
                .on_hover_text("Drag to move · Click or Ctrl+/ to open menu");
            if plus.hovered() {
                ui.painter()
                    .rect_filled(plus_rect, CornerRadius::same(4), palette.hover);
                draw_plus(ui.painter(), plus_rect, palette.text);
            } else {
                draw_plus(ui.painter(), plus_rect, palette.muted);
            }
            if handle.hovered() || handle.dragged() {
                ui.painter()
                    .rect_filled(handle_rect, CornerRadius::same(4), palette.hover);
                draw_dots(ui.painter(), handle_rect, palette.text);
            } else {
                draw_dots(ui.painter(), handle_rect, palette.muted);
            }
            if plus.clicked() {
                let above = ui.input(|i| i.modifiers.alt);
                commands.push(Command::AddBlock {
                    block: block.id.clone(),
                    above,
                    pos: menu_pos,
                });
            }
            if handle.drag_started() {
                commands.push(Command::StartBlockDrag(block.id.clone()));
            }
            if handle.drag_stopped() {
                commands.push(Command::DropBlock);
            }
            if handle.clicked() {
                commands.push(Command::OpenSlash {
                    block: block.id.clone(),
                    pos: menu_pos,
                });
            }
        }

        let locked = view.lock_page;
        let block_id_for_edit = block.id.clone();
        let page_id_for_edit = view.current_page.clone();
        let pending_commands = std::cell::RefCell::new(Vec::new());
        let edit = |ui: &mut egui::Ui,
                    text: &mut String,
                    font: FontId,
                    w: f32|
         -> egui::widgets::text_edit::TextEditOutput {
            let output = egui::TextEdit::multiline(text)
                .id(ui.make_persistent_id(("block_edit", &block_id_for_edit, index)))
                .font(font.clone())
                .frame(egui::Frame::NONE)
                .desired_width(w)
                .desired_rows(1)
                .interactive(!locked)
                .text_color(palette.text)
                .show(ui);
            let remote: Vec<Awareness> = view
                .remote_cursors
                .iter()
                .filter(|cursor| {
                    cursor.page_id == page_id_for_edit && cursor.block_id == block_id_for_edit
                })
                .cloned()
                .collect();
            draw_remote_cursors(ui, &output.response.rect, text, &font, &remote, palette);
            if output.response.changed() {
                pending_commands
                    .borrow_mut()
                    .push(Command::CrdtTextChanged {
                        page_id: page_id_for_edit.clone(),
                        block_id: block_id_for_edit.clone(),
                        text: text.clone(),
                    });
            }
            if let Some(range) = output.cursor_range {
                let index = range.primary.index.0;
                let selection_end = (range.primary.index != range.secondary.index)
                    .then_some(range.secondary.index.0);
                pending_commands.borrow_mut().push(Command::CrdtCursor {
                    page_id: page_id_for_edit.clone(),
                    block_id: block_id_for_edit.clone(),
                    index,
                    selection_end,
                });
            }
            output
        };

        let mut edit_response: Option<egui::widgets::text_edit::TextEditOutput> = None;
        match block.kind {
            BlockKind::Divider => {
                let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 14.0), Sense::hover());
                ui.painter().hline(
                    rect.x_range(),
                    rect.center().y,
                    Stroke::new(1.0, palette.border),
                );
            }
            BlockKind::Todo => {
                let checked = block.checked;
                let check = icon_button(ui, 18.0, palette, move |painter, rect, color| {
                    if checked {
                        painter.rect_filled(rect.shrink(2.0), CornerRadius::same(3), color);
                        let c = rect.center();
                        let stroke = Stroke::new(1.5, Color32::WHITE);
                        painter.line(
                            vec![
                                c + Vec2::new(-3.0, 0.0),
                                c + Vec2::new(-1.0, 2.0),
                                c + Vec2::new(3.5, -2.5),
                            ],
                            stroke,
                        );
                    } else {
                        painter.rect_stroke(
                            rect.shrink(2.0),
                            CornerRadius::same(3),
                            Stroke::new(1.4, color),
                            egui::StrokeKind::Inside,
                        );
                    }
                });
                if check.clicked() && !locked {
                    commands.push(Command::ToggleTodo(block.id.clone()));
                }
                ui.add_space(6.0);
                edit_response = Some(edit(
                    ui,
                    &mut block.text,
                    body_font(view, 15.0),
                    width - 26.0,
                ));
            }
            BlockKind::Bulleted => {
                let (rect, _) = ui.allocate_exact_size(Vec2::new(20.0, 22.0), Sense::hover());
                draw_bullet_dot(ui.painter(), rect, palette.accent);
                edit_response = Some(edit(
                    ui,
                    &mut block.text,
                    body_font(view, 15.0),
                    width - 22.0,
                ));
            }
            BlockKind::Numbered => {
                *numbered += 1;
                let n = *numbered;
                let (rect, _) = ui.allocate_exact_size(Vec2::new(20.0, 22.0), Sense::hover());
                ui.painter().text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    format!("{n}."),
                    FontId::proportional(14.0),
                    palette.text,
                );
                edit_response = Some(edit(
                    ui,
                    &mut block.text,
                    body_font(view, 15.0),
                    width - 22.0,
                ));
            }
            BlockKind::Quote => {
                let (rect, _) = ui.allocate_exact_size(Vec2::new(6.0, 22.0), Sense::hover());
                ui.painter().vline(
                    rect.center().x,
                    rect.y_range(),
                    Stroke::new(3.0, palette.text),
                );
                ui.add_space(8.0);
                edit_response = Some(edit(
                    ui,
                    &mut block.text,
                    body_font(view, 16.0),
                    width - 16.0,
                ));
            }
            BlockKind::Callout => {
                egui::Frame::NONE
                    .fill(palette.callout)
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(Margin::same(12))
                    .show(ui, |ui| {
                        ui.set_width(width - 24.0);
                        ui.horizontal(|ui| {
                            let (icon, _) =
                                ui.allocate_exact_size(Vec2::new(18.0, 18.0), Sense::hover());
                            draw_page(ui.painter(), icon, palette.muted);
                            ui.add_space(8.0);
                            edit_response = Some(edit(
                                ui,
                                &mut block.text,
                                body_font(view, 15.0),
                                width - 70.0,
                            ));
                        });
                    });
            }
            BlockKind::Code => {
                egui::Frame::NONE
                    .fill(palette.code_bg)
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(Margin::same(12))
                    .show(ui, |ui| {
                        ui.set_width(width - 24.0);
                        edit_response = Some(edit(
                            ui,
                            &mut block.text,
                            FontId::monospace(13.0),
                            width - 24.0,
                        ));
                    });
            }
            BlockKind::Paragraph
            | BlockKind::Heading1
            | BlockKind::Heading2
            | BlockKind::Heading3
            | BlockKind::Heading4
            | BlockKind::Table
            | BlockKind::TableRow
            | BlockKind::Toggle
            | BlockKind::ChildPage
            | BlockKind::ColumnList
            | BlockKind::Column
            | BlockKind::Unsupported => {
                let font = if block.kind.is_heading() {
                    strong_font(block.kind.font_size())
                } else {
                    body_font(view, 15.0)
                };
                edit_response = Some(edit(ui, &mut block.text, font, width));
            }
        }

        commands.append(&mut pending_commands.into_inner());

        if !locked {
            if let Some(ref resp) = edit_response
                && backspace_on_empty(ui, &resp.response, &block.text)
                && block_count > 1
            {
                delete_empty = true;
            }
            if let Some((kind, prefix)) = markdown_prefix(&block.text) {
                commands.push(Command::ApplyMarkdown {
                    block: block.id.clone(),
                    kind,
                    prefix: prefix.to_string(),
                });
            } else if block.text.starts_with('/') {
                commands.push(Command::OpenSlash {
                    block: block.id.clone(),
                    pos: Pos2::new(ui.max_rect().min.x + 60.0, ui.max_rect().min.y),
                });
            }
        }
        let _ = index;
    });
    if delete_empty {
        commands.push(Command::DeleteBlock(block.id.clone()));
    }
}

/// Draw remote carets and selections at the best available text offset.
///
/// egui's multiline editor exposes a character index but not a per-line rect
/// here, so the marker uses the first-line width. It is a clear presence cue,
/// not a pixel-perfect multi-line caret.
fn draw_remote_cursors(
    ui: &mut egui::Ui,
    rect: &Rect,
    text: &str,
    font: &FontId,
    cursors: &[Awareness],
    palette: &Palette,
) {
    for cursor in cursors {
        let color = parse_hex_color(&cursor.color).unwrap_or(palette.accent);
        if let Some(end) = cursor.selection_end {
            let start = cursor.index.min(end);
            let end = cursor.index.max(end);
            let start_x = rect.min.x + text_width(ui, text, start, font, color);
            let end_x = rect.min.x + text_width(ui, text, end, font, color);
            ui.painter().rect_filled(
                Rect::from_min_max(Pos2::new(start_x, rect.min.y), Pos2::new(end_x, rect.max.y)),
                CornerRadius::same(2),
                Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 40),
            );
        }
        let x = (rect.min.x + text_width(ui, text, cursor.index, font, color))
            .clamp(rect.min.x, rect.max.x - 2.0);
        ui.painter()
            .vline(x, rect.y_range(), Stroke::new(2.0, color));
        ui.painter().text(
            Pos2::new(x + 3.0, rect.min.y - 2.0),
            Align2::LEFT_BOTTOM,
            &cursor.name,
            FontId::proportional(10.0),
            color,
        );
    }
}

fn text_width(ui: &egui::Ui, text: &str, chars: usize, font: &FontId, color: Color32) -> f32 {
    let prefix: String = text.chars().take(chars).collect();
    ui.painter()
        .layout_no_wrap(prefix, font.clone(), color)
        .size()
        .x
}

fn parse_hex_color(value: &str) -> Option<Color32> {
    let hex = value.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color32::from_rgb(red, green, blue))
}

/// Leading markdown that should convert the current block (help-site shortcuts).
fn markdown_prefix(text: &str) -> Option<(BlockKind, &'static str)> {
    const RULES: &[(&str, BlockKind)] = &[
        ("### ", BlockKind::Heading3),
        ("## ", BlockKind::Heading2),
        ("# ", BlockKind::Heading1),
        ("[] ", BlockKind::Todo),
        ("1. ", BlockKind::Numbered),
        ("* ", BlockKind::Bulleted),
        ("- ", BlockKind::Bulleted),
        ("+ ", BlockKind::Bulleted),
        ("> ", BlockKind::Quote),
        ("\" ", BlockKind::Quote),
        ("---", BlockKind::Divider),
    ];
    RULES
        .iter()
        .find(|(prefix, _)| text.starts_with(prefix))
        .map(|(prefix, kind)| (*kind, *prefix))
}
