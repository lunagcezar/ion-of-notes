//! Page header, breadcrumbs and page body entry point.

use crate::i18n;
use crate::model::{Block, BlockKind, Workspace};
use egui::{Align2, CornerRadius, FontFamily, FontId, Pos2, Rect, Sense, Stroke, Vec2};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

use super::super::super::commands::{Command, FontStyle, SidebarCmd};
use super::super::super::format::format_timestamp;
use super::super::super::icons::{
    draw_ellipsis, draw_lines, draw_link, draw_page_icon, draw_pen, draw_plus, draw_share,
    draw_star, glyph_button, icon_button, page_icon_letter,
};
use super::super::super::palette::{Palette, strong_font};
use super::super::super::view::ViewState;
use super::super::database::render_database;
use super::super::library::render_library;
use super::blocks::render_block;
use crate::ui::constants::{
    BLOCK_GAP, BLOCK_HEADING_GAP, BREADCRUMB_ACTION_HEIGHT, BREADCRUMB_ACTION_SIZE, BREADCRUMB_GAP,
    BREADCRUMB_HEIGHT, BREADCRUMB_ICON_INSET, BREADCRUMB_ICON_SIZE, BREADCRUMB_LABEL_INSET,
    BREADCRUMB_PADDING, CONTENT_BOTTOM_SPACE, CONTENT_COLUMN_MAX, CONTENT_COLUMN_MIN,
    CONTENT_GUTTER, CONTENT_SIDE_MARGIN, CONTENT_TOP_SPACE, FLOATING_CONTROL_INSET,
    FLOATING_CONTROL_SIZE, FLOATING_LINES_SIZE, FLOATING_LINES_TOP, FONT_CAPTION, FONT_MICRO,
    FONT_PAGE_TITLE, FONT_ROW, FONT_SUBTITLE, PAGE_DISCUSSION_GAP, PAGE_ICON_RADIUS,
    PAGE_ICON_SIZE, PAGE_TITLE_BOTTOM, PAGE_TITLE_GAP, PAGE_TOC_GAP, SPACE_2, SPACE_4, SPACE_6,
    SPACE_8, SPACE_10, SPACE_12, SPACE_14, SPACE_20, SPACE_22, SPACE_24, SPACE_36, STROKE_HAIRLINE,
};
pub(crate) fn render_content(
    ui: &mut egui::Ui,
    ws: &mut Workspace,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let _span = tracing::trace_span!("ui.render_content").entered();
    // Compact breadcrumb bar (spec-013).
    let full_width = ui.available_width();
    let column_width = if view.settings.full_width {
        (full_width - CONTENT_SIDE_MARGIN).max(CONTENT_COLUMN_MIN)
    } else {
        (full_width - CONTENT_SIDE_MARGIN).clamp(CONTENT_COLUMN_MIN, CONTENT_COLUMN_MAX)
    };
    let gutter = CONTENT_GUTTER;
    let pad = ((full_width - column_width) / 2.0 - gutter).max(0.0);
    let breadcrumb_pad = BREADCRUMB_PADDING;
    ui.vertical(|ui| {
        ui.add_space(SPACE_2);
        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), BREADCRUMB_HEIGHT),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.spacing_mut().item_spacing.x = BREADCRUMB_GAP;
                ui.add_space(breadcrumb_pad);
                if !view.sidebar_open && ui.add(egui::Button::new(">").frame(false)).clicked() {
                    commands.push(Command::Sidebar(SidebarCmd::Toggle));
                }
                let mut crumbs = crumb_chain(ws, &view.current_page);
                if crumbs.is_empty() {
                    crumbs.push((String::new(), "Workspace".to_string()));
                }
                if crumbs.len() == 1 {
                    crumbs.insert(0, (ws.settings.icon.clone(), ws.name.clone()));
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
                    ui.spacing_mut().item_spacing.x = SPACE_4;
                    ui.add_space(SPACE_14);
                    if glyph_button(
                        ui,
                        Vec2::new(BREADCRUMB_ACTION_SIZE, BREADCRUMB_ACTION_HEIGHT),
                        palette,
                        draw_ellipsis,
                    )
                    .on_hover_text("Page settings and actions")
                    .clicked()
                    {
                        commands.push(Command::OpenPageActions);
                    }
                    if !view.library_open
                        && glyph_button(
                            ui,
                            Vec2::new(BREADCRUMB_ACTION_SIZE, BREADCRUMB_ACTION_HEIGHT),
                            palette,
                            draw_star,
                        )
                        .clicked()
                    {
                        commands.push(Command::Sidebar(SidebarCmd::ToggleFavorite(
                            view.current_page.clone(),
                        )));
                    }
                    if glyph_button(
                        ui,
                        Vec2::new(BREADCRUMB_ACTION_SIZE, BREADCRUMB_ACTION_HEIGHT),
                        palette,
                        draw_link,
                    )
                    .clicked()
                    {
                        commands.push(Command::OpenShare);
                    }
                    let share = ui.add(
                        egui::Button::new(
                            egui::RichText::new(i18n::t("share"))
                                .size(FONT_ROW)
                                .color(palette.text),
                        )
                        .frame(false),
                    );
                    if share.clicked() {
                        commands.push(Command::OpenShare);
                    }
                    if glyph_button(
                        ui,
                        Vec2::new(SPACE_20, BREADCRUMB_ACTION_HEIGHT),
                        palette,
                        draw_share,
                    )
                    .clicked()
                    {
                        commands.push(Command::OpenShare);
                    }
                });
            },
        );
        ui.add_space(SPACE_2);
    });
    ui.separator();

    if view.library_open {
        render_library(ui, ws, view, palette, commands);
    } else {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    ui.add_space(pad);
                    ui.vertical(|ui| {
                        ui.set_width(column_width + gutter);
                        ui.add_space(CONTENT_TOP_SPACE);
                        render_page(ui, ws, view, column_width, gutter, palette, commands);
                        ui.add_space(CONTENT_BOTTOM_SPACE);
                    });
                });
            });
    }

    // Floating controls on the right edge of the content, as in the pictures.
    let screen = ui.ctx().viewport_rect();
    let lines_center = Pos2::new(
        screen.right() - FLOATING_CONTROL_INSET,
        screen.top() + FLOATING_LINES_TOP,
    );
    draw_lines(
        ui.painter(),
        Rect::from_center_size(lines_center, Vec2::splat(FLOATING_LINES_SIZE)),
        palette.muted,
    );
    let button_center = Pos2::new(
        screen.right() - FLOATING_CONTROL_INSET,
        screen.bottom() - FLOATING_CONTROL_INSET,
    );
    ui.painter()
        .circle_filled(button_center, FLOATING_CONTROL_SIZE, palette.hover);
    ui.painter().circle_stroke(
        button_center,
        FLOATING_CONTROL_SIZE,
        Stroke::new(STROKE_HAIRLINE, palette.border),
    );
    draw_pen(
        ui.painter(),
        Rect::from_center_size(button_center, Vec2::splat(FLOATING_CONTROL_SIZE)),
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
    let size = if view.settings.small_text {
        size * 0.85
    } else {
        size
    };
    match view.settings.font_style {
        FontStyle::Default => FontId::proportional(size),
        FontStyle::Serif => FontId::new(size, FontFamily::Name("Serif".into())),
        FontStyle::Mono => FontId::monospace(size),
    }
}

fn crumb(ui: &mut egui::Ui, icon: &str, text: &str, palette: &Palette) -> egui::Response {
    let width = BREADCRUMB_ICON_SIZE + SPACE_8 + text.chars().count() as f32 * SPACE_8 + SPACE_10;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, SPACE_20), Sense::click());
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(4), palette.hover);
    }
    draw_page_icon(
        ui.painter(),
        Rect::from_min_size(
            rect.min + Vec2::new(BREADCRUMB_ICON_INSET, BREADCRUMB_ICON_INSET),
            Vec2::splat(BREADCRUMB_ICON_SIZE),
        ),
        &page_icon_letter(icon, text),
        palette,
        false,
    );
    ui.painter().text(
        rect.min + Vec2::new(BREADCRUMB_LABEL_INSET, SPACE_10),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(FONT_ROW),
        palette.text,
    );
    response
}

// ---- Page -----------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
/// Cached measured height for one block (spec-110).
#[derive(Clone, Copy)]
struct BlockHeight {
    fingerprint: u64,
    height: f32,
}

/// A cheap content fingerprint; a change invalidates the cached height.
fn block_fingerprint(block: &Block) -> u64 {
    let mut hasher = DefaultHasher::new();
    block.text.hash(&mut hasher);
    block.rich_text.len().hash(&mut hasher);
    std::mem::discriminant(&block.kind).hash(&mut hasher);
    block.children.len().hash(&mut hasher);
    hasher.finish()
}

fn render_page(
    ui: &mut egui::Ui,
    ws: &mut Workspace,
    view: &ViewState,
    width: f32,
    gutter: f32,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let cursors_by_block = super::cursors::index_cursors(&view.remote_cursors, &view.current_page);
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
                ui.allocate_exact_size(Vec2::splat(PAGE_ICON_SIZE), Sense::click());
            draw_page_icon(
                ui.painter(),
                icon_rect,
                &page_icon_letter(&page.icon, &page.title),
                palette,
                true,
            );
            if icon_response.hovered() {
                ui.painter().rect_stroke(
                    icon_rect,
                    CornerRadius::same(PAGE_ICON_RADIUS),
                    Stroke::new(STROKE_HAIRLINE, palette.border),
                    egui::StrokeKind::Inside,
                );
            }
            if icon_response.on_hover_text("Change icon").clicked() {
                commands.push(Command::OpenIconPicker);
            }
            ui.add_space(PAGE_TITLE_GAP);
            let _ = ui.add(
                egui::TextEdit::singleline(&mut page.title)
                    .font(strong_font(FONT_PAGE_TITLE))
                    .frame(egui::Frame::NONE)
                    .desired_width(width)
                    .text_color(palette.text)
                    .hint_text("Untitled"),
            );
            ui.add_space(PAGE_TITLE_BOTTOM);
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
                        .size(FONT_CAPTION)
                        .strong()
                        .color(palette.muted),
                );
                if headings.is_empty() {
                    ui.label(
                        egui::RichText::new("Add a heading to build the table of contents.")
                            .size(FONT_CAPTION)
                            .color(palette.muted),
                    );
                }
                for (text, kind) in &headings {
                    let indent = match kind {
                        BlockKind::Heading2 => SPACE_12,
                        BlockKind::Heading3 => SPACE_24,
                        BlockKind::Heading4 => SPACE_36,
                        _ => 0.0,
                    };
                    ui.horizontal(|ui| {
                        ui.add_space(indent);
                        ui.label(
                            egui::RichText::new(text)
                                .size(FONT_CAPTION)
                                .color(palette.accent),
                        );
                    });
                }
            });
        });
        ui.add_space(PAGE_TOC_GAP);
    }

    let mut numbered: usize = 0;
    let block_count = page.blocks.len();
    let height_id = egui::Id::unique(("block_heights", page.id.as_str()));
    let mut heights: HashMap<String, BlockHeight> = ui
        .ctx()
        .data(|data| data.get_temp::<HashMap<String, BlockHeight>>(height_id))
        .unwrap_or_default();
    let viewport = ui.clip_rect();
    let overscan = (viewport.height() * 1.5).max(400.0);
    let interactive = view.block_selecting || view.drag_from.is_some();
    for index in 0..page.blocks.len() {
        let block = &mut page.blocks[index];
        let top = ui.cursor().top();
        let forced = view.focus_block.as_deref() == Some(block.id.as_str())
            || view.slash_for.as_deref() == Some(block.id.as_str())
            || view.page_selection.iter().any(|id| id == &block.id);
        let cached = heights.get(&block.id).copied();
        let fingerprint = block_fingerprint(block);
        if !interactive
            && !forced
            && let Some(entry) = cached
            && entry.fingerprint == fingerprint
            && crate::ui::block_outside_viewport(
                top,
                entry.height,
                (viewport.top(), viewport.bottom()),
                overscan,
            )
        {
            if block.kind.is_heading() {
                ui.add_space(BLOCK_HEADING_GAP);
            }
            ui.add_space(entry.height + BLOCK_GAP);
            continue;
        }
        if block.kind.is_heading() {
            ui.add_space(BLOCK_HEADING_GAP);
        }
        let before = ui.cursor().top();
        render_block(
            ui,
            block,
            index,
            &mut numbered,
            width,
            gutter,
            view,
            &cursors_by_block,
            palette,
            commands,
            block_count,
        );
        let height = (ui.cursor().top() - before).max(0.0);
        heights.insert(
            block.id.clone(),
            BlockHeight {
                fingerprint,
                height,
            },
        );
        ui.add_space(BLOCK_GAP);
    }
    ui.ctx().data_mut(|data| {
        data.insert_temp(height_id, heights);
    });

    // Subtle add-block row.
    ui.horizontal(|ui| {
        ui.add_space(gutter);
        if icon_button(ui, SPACE_22, palette, draw_plus).clicked() {
            commands.push(Command::AppendBlock);
        }
        ui.label(
            egui::RichText::new("Click to add a block")
                .size(FONT_ROW)
                .color(palette.muted),
        );
    });

    if page.discussions {
        ui.add_space(PAGE_DISCUSSION_GAP);
        ui.horizontal(|ui| {
            ui.add_space(gutter);
            ui.vertical(|ui| {
                ui.set_width(width);
                ui.label(
                    egui::RichText::new(i18n::t("page_discussions"))
                        .font(strong_font(FONT_SUBTITLE))
                        .color(palette.text),
                );
                ui.add_space(SPACE_6);
                if page.comments.is_empty() {
                    ui.label(
                        egui::RichText::new(i18n::t("no_comments"))
                            .size(FONT_CAPTION)
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
                                view.settings.time_format,
                            ))
                            .size(FONT_MICRO)
                            .color(palette.muted),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button(i18n::t("delete")).clicked() {
                                commands.push(Command::DeletePageComment(comment.id.clone()));
                            }
                        });
                    });
                    ui.label(egui::RichText::new(&comment.text).color(palette.text));
                    ui.add_space(SPACE_6);
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
                    ui.add_space(SPACE_6);
                    ui.label(
                        egui::RichText::new(i18n::t("inline_note"))
                            .size(FONT_CAPTION)
                            .color(palette.muted),
                    );
                }
            });
        });
    }

    if !page.rows.is_empty() || page.title == "Tasks" {
        ui.add_space(PAGE_DISCUSSION_GAP);
        render_database(ui, page, view, width, gutter, palette, commands);
    }
}
