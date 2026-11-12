//! Editable block rendering and block-level shortcuts.

use crate::model::{Block, BlockKind};
use crate::server::realtime::Awareness;
use egui::{Align2, Color32, CornerRadius, FontId, Margin, Pos2, Rect, Sense, Stroke, Vec2};

use crate::ui::constants::{
    BLOCK_BODY_SIZE, BLOCK_BULLET_SIZE, BLOCK_CALLOUT_PADDING, BLOCK_CODE_PADDING,
    BLOCK_CONTROL_GAP, BLOCK_CONTROL_HEIGHT, BLOCK_CONTROL_WIDTH, BLOCK_DIVIDER_HEIGHT,
    BLOCK_DRAG_OVER, BLOCK_MONO_SIZE, BLOCK_ROW_HEIGHT, BLOCK_SMALL_SIZE, BLOCK_TODO_SIZE,
    DRAG_GHOST_MAX_WIDTH, DRAG_GHOST_MIN_WIDTH, FONT_ROW, ICON_RADIUS, ROW_RADIUS, SPACE_2,
    SPACE_4, SPACE_6, SPACE_8, SPACE_12, SPACE_16, SPACE_18, SPACE_20, SPACE_26, SPACE_60,
    SPACE_70, STROKE_EMPHASIS, STROKE_HAIRLINE,
};

use super::super::super::commands::Command;
use super::super::super::icons::{draw_bullet_dot, draw_dots, draw_page, draw_plus, icon_button};
use super::super::super::palette::{Palette, strong_font};
use super::super::super::view::ViewState;
use super::cursors::draw_remote_cursors;
use super::page::body_font;
fn backspace_on_empty(ui: &egui::Ui, response: &egui::Response, text: &str) -> bool {
    response.has_focus() && text.is_empty() && ui.input(|i| i.key_pressed(egui::Key::Backspace))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render_block(
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
    let row_rect = Rect::from_min_size(row_min, Vec2::new(width + gutter, BLOCK_ROW_HEIGHT));
    let hover = ui
        .input(|i| i.pointer.hover_pos())
        .map(|p| row_rect.contains(p))
        .unwrap_or(false);
    let menu_pos = Pos2::new(row_rect.min.x + SPACE_60, row_rect.min.y);

    // Page-level selection is painted after the block layout so the overlay
    // covers the full multi-line text rect, not just the first line.
    if view.block_selecting
        && ui.input(|i| i.pointer.primary_down())
        && let Some(p) = ui.ctx().pointer_interact_pos()
        && row_rect.contains(p)
    {
        commands.push(Command::ExtendBlockSelection(block.id.clone()));
    }
    if view.slash_dismissed.as_deref() == Some(block.id.as_str()) && !block.text.starts_with('/') {
        commands.push(Command::ClearSlashDismissed);
    }

    // While the six-dot handle is being dragged, the nearest gap above or
    // below a block becomes the reorder target (spec-064).
    if view.drag_from.is_some()
        && let Some(p) = ui.ctx().pointer_interact_pos()
        && row_rect
            .expand2(Vec2::new(0.0, BLOCK_DRAG_OVER))
            .contains(p)
    {
        ui.ctx().data_mut(|data| {
            data.insert_temp(egui::Id::unique("block_drag_over"), true);
        });
        commands.push(Command::HoverBlockDrag {
            block: block.id.clone(),
            before: p.y < row_rect.center().y,
        });
    }
    if view.drag_to.as_deref() == Some(block.id.as_str())
        && view.drag_from.as_deref() != Some(block.id.as_str())
    {
        let y = if view.drag_before {
            row_rect.top()
        } else {
            row_rect.max.y
        };
        ui.painter().hline(
            row_rect.x_range(),
            y,
            Stroke::new(STROKE_EMPHASIS, palette.accent),
        );
    }

    let mut delete_empty = false;
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        // Keep layout stable regardless of hover to avoid stealing TextEdit focus.
        let control_h = if block.kind.is_heading() {
            block.kind.font_size().max(BLOCK_CONTROL_HEIGHT)
        } else {
            BLOCK_CONTROL_HEIGHT
        };
        let plus_rect = ui
            .allocate_exact_size(Vec2::new(BLOCK_CONTROL_WIDTH, control_h), Sense::hover())
            .0;
        let handle_rect = ui
            .allocate_exact_size(Vec2::new(BLOCK_CONTROL_WIDTH, control_h), Sense::hover())
            .0;
        ui.add_space(BLOCK_CONTROL_GAP);
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
                .on_hover_text("Drag to move this block");
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
        }

        let locked = view.lock_page;
        let block_id_for_edit = block.id.clone();
        let page_id_for_edit = view.current_page.clone();
        let edit_id = egui::Id::unique(("block_edit", &block.id, index));
        let pending_commands = std::cell::RefCell::new(Vec::new());
        if !locked && ui.memory(|memory| memory.has_focus(edit_id)) {
            // Notion-style split: Enter creates a new block; Shift+Enter keeps
            // the native newline behavior.
            if ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
                commands.push(Command::SplitBlock(block.id.clone()));
            }
        }
        if view.focus_block.as_deref() == Some(block.id.as_str()) {
            ui.memory_mut(|memory| memory.request_focus(edit_id));
            commands.push(Command::ClearBlockFocus);
        }
        let edit = |ui: &mut egui::Ui,
                    text: &mut String,
                    font: FontId,
                    w: f32|
         -> egui::widgets::text_edit::TextEditOutput {
            let output = egui::TextEdit::multiline(text)
                .id(edit_id)
                .font(font.clone())
                .frame(egui::Frame::NONE)
                .desired_width(w)
                .desired_rows(1)
                .interactive(!locked)
                .hint_text(crate::i18n::t("block_placeholder"))
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
            if output.response.dragged()
                && ui.input(|i| i.pointer.primary_down())
                && ui
                    .input(|i| i.pointer.interact_pos())
                    .is_some_and(|p| !output.response.rect.contains(p))
            {
                pending_commands
                    .borrow_mut()
                    .push(Command::ExtendBlockSelection(block_id_for_edit.clone()));
            }
            output
        };

        // Ctrl+A and Ctrl+C must win over the per-block TextEdit so the whole
        // page selection is used (spec-057).
        if !locked && ui.memory(|m| m.has_focus(edit_id)) {
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::A)) {
                commands.push(Command::SelectAllBlocks);
            }
            if !view.page_selection.is_empty()
                && ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::C))
            {
                commands.push(Command::CopyBlockSelection);
            }
        }

        let mut edit_response: Option<egui::widgets::text_edit::TextEditOutput> = None;
        match block.kind {
            BlockKind::Divider => {
                let (rect, _) =
                    ui.allocate_exact_size(Vec2::new(width, BLOCK_DIVIDER_HEIGHT), Sense::hover());
                ui.painter().hline(
                    rect.x_range(),
                    rect.center().y,
                    Stroke::new(STROKE_HAIRLINE, palette.border),
                );
            }
            BlockKind::Todo => {
                let checked = block.checked;
                let check =
                    icon_button(ui, BLOCK_TODO_SIZE, palette, move |painter, rect, color| {
                        if checked {
                            painter.rect_filled(
                                rect.shrink(SPACE_2),
                                CornerRadius::same(ICON_RADIUS),
                                color,
                            );
                            let c = rect.center();
                            let stroke = Stroke::new(STROKE_EMPHASIS, Color32::WHITE);
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
                                rect.shrink(SPACE_2),
                                CornerRadius::same(3),
                                Stroke::new(STROKE_HAIRLINE, color),
                                egui::StrokeKind::Inside,
                            );
                        }
                    });
                if check.clicked() && !locked {
                    commands.push(Command::ToggleTodo(block.id.clone()));
                }
                ui.add_space(SPACE_6);
                edit_response = Some(edit(
                    ui,
                    &mut block.text,
                    body_font(view, BLOCK_SMALL_SIZE),
                    width - SPACE_26,
                ));
            }
            BlockKind::Bulleted => {
                let (rect, _) = ui
                    .allocate_exact_size(Vec2::new(SPACE_20, BLOCK_CONTROL_HEIGHT), Sense::hover());
                draw_bullet_dot(ui.painter(), rect, palette.accent);
                edit_response = Some(edit(
                    ui,
                    &mut block.text,
                    body_font(view, BLOCK_SMALL_SIZE),
                    width - BLOCK_CONTROL_WIDTH,
                ));
            }
            BlockKind::Numbered => {
                *numbered += 1;
                let n = *numbered;
                let (rect, _) = ui
                    .allocate_exact_size(Vec2::new(SPACE_20, BLOCK_CONTROL_HEIGHT), Sense::hover());
                ui.painter().text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    format!("{n}."),
                    FontId::proportional(BLOCK_SMALL_SIZE),
                    palette.text,
                );
                edit_response = Some(edit(
                    ui,
                    &mut block.text,
                    body_font(view, BLOCK_SMALL_SIZE),
                    width - BLOCK_CONTROL_WIDTH,
                ));
            }
            BlockKind::Quote => {
                let (rect, _) = ui.allocate_exact_size(
                    Vec2::new(BLOCK_BULLET_SIZE, BLOCK_CONTROL_HEIGHT),
                    Sense::hover(),
                );
                ui.painter().vline(
                    rect.center().x,
                    rect.y_range(),
                    Stroke::new(SPACE_4, palette.text),
                );
                ui.add_space(SPACE_8);
                edit_response = Some(edit(
                    ui,
                    &mut block.text,
                    body_font(view, BLOCK_BODY_SIZE),
                    width - SPACE_16,
                ));
            }
            BlockKind::Callout => {
                egui::Frame::NONE
                    .fill(palette.callout)
                    .corner_radius(CornerRadius::same(ROW_RADIUS))
                    .inner_margin(Margin::same(BLOCK_CALLOUT_PADDING as i8))
                    .show(ui, |ui| {
                        ui.set_width(width - 2.0 * BLOCK_CALLOUT_PADDING);
                        ui.horizontal(|ui| {
                            let (icon, _) = ui.allocate_exact_size(
                                Vec2::new(BLOCK_TODO_SIZE, BLOCK_TODO_SIZE),
                                Sense::hover(),
                            );
                            draw_page(ui.painter(), icon, palette.muted);
                            ui.add_space(SPACE_8);
                            edit_response = Some(edit(
                                ui,
                                &mut block.text,
                                body_font(view, BLOCK_SMALL_SIZE),
                                width - SPACE_70,
                            ));
                        });
                    });
            }
            BlockKind::Code => {
                egui::Frame::NONE
                    .fill(palette.code_bg)
                    .corner_radius(CornerRadius::same(ROW_RADIUS))
                    .inner_margin(Margin::same(BLOCK_CODE_PADDING as i8))
                    .show(ui, |ui| {
                        ui.set_width(width - 2.0 * BLOCK_CODE_PADDING);
                        edit_response = Some(edit(
                            ui,
                            &mut block.text,
                            FontId::monospace(BLOCK_MONO_SIZE),
                            width - 2.0 * BLOCK_CODE_PADDING,
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
                    body_font(view, BLOCK_SMALL_SIZE)
                };
                edit_response = Some(edit(ui, &mut block.text, font, width));
            }
        }

        commands.append(&mut pending_commands.into_inner());

        if view.page_selection.iter().any(|id| id == &block.id)
            && let Some(resp) = &edit_response
        {
            ui.painter().rect_filled(
                resp.response.rect,
                CornerRadius::same(4),
                Color32::from_rgba_unmultiplied(35, 131, 226, 42),
            );
        }

        if !locked {
            if let Some(ref resp) = edit_response
                && backspace_on_empty(ui, &resp.response, &block.text)
                && block_count > 1
            {
                delete_empty = true;
            }
            if let Some(ref resp) = edit_response
                && resp.response.clicked()
                && !ui.input(|i| i.modifiers.shift)
            {
                commands.push(Command::ClearBlockSelection);
            }
            if let Some((kind, prefix)) = markdown_prefix(&block.text) {
                commands.push(Command::ApplyMarkdown {
                    block: block.id.clone(),
                    kind,
                    prefix: prefix.to_string(),
                });
            } else if block.text.starts_with('/')
                && view.slash_dismissed.as_deref() != Some(block.id.as_str())
            {
                let query = block
                    .text
                    .trim_start_matches('/')
                    .lines()
                    .next()
                    .unwrap_or("")
                    .to_string();
                let pos = edit_response.as_ref().map_or(menu_pos, |resp| {
                    resp.response.rect.left_bottom() + Vec2::new(0.0, SPACE_4)
                });
                commands.push(Command::OpenSlash {
                    block: block.id.clone(),
                    pos,
                    query,
                });
            }
        }
        let _ = index;
    });
    if view.drag_from.as_deref() == Some(block.id.as_str())
        && let Some(pointer) = ui.ctx().pointer_interact_pos()
    {
        // Live drag preview: a floating copy of the block follows the pointer
        // while the insertion line marks the drop target (spec-057).
        let ghost = Rect::from_min_size(
            pointer + Vec2::new(SPACE_18, SPACE_8),
            Vec2::new(
                (width * 0.8).clamp(DRAG_GHOST_MIN_WIDTH, DRAG_GHOST_MAX_WIDTH),
                BLOCK_ROW_HEIGHT,
            ),
        );
        let painter = ui.ctx().layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::unique("block_drag_ghost"),
        ));
        painter.rect(
            ghost,
            CornerRadius::same(ROW_RADIUS),
            palette.content,
            Stroke::new(STROKE_HAIRLINE, palette.accent),
            egui::StrokeKind::Inside,
        );
        let label = if block.text.is_empty() {
            "Empty block".to_string()
        } else {
            block.text.lines().next().unwrap_or("").to_string()
        };
        painter.text(
            ghost.min + Vec2::new(SPACE_12, BLOCK_ROW_HEIGHT / 2.0),
            Align2::LEFT_CENTER,
            label,
            FontId::proportional(FONT_ROW),
            palette.text,
        );
    }
    if delete_empty {
        commands.push(Command::DeleteBlock(block.id.clone()));
    }
}

/// Draw remote carets and selections at the best available text offset.
///
/// egui's multiline editor exposes a character index but not a per-line rect
/// here, so the marker uses the first-line width. It is a clear presence cue,
/// not a pixel-perfect multi-line caret.
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
