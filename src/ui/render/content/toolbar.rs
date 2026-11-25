//! Floating toolbar for text and page-level block selections (spec-157).
//!
//! It renders beside the selection, offers whole-block or selected-text
//! formats and opens the block-type menu. Every click becomes a `Command`;
//! nothing is mutated here.

use crate::model::{Block, BlockKind};
use egui::{Align2, Color32, CornerRadius, FontId, Margin, Pos2, Rect, Sense, Stroke, Vec2};

use crate::ui::constants::{
    BLOCK_TYPE_MENU_MAX_HEIGHT, BLOCK_TYPE_MENU_WIDTH, FONT_CAPTION, FONT_MICRO, FONT_ROW,
    MENU_HINT_SIZE, MENU_ROW_HEIGHT, MENU_ROW_RADIUS, MENU_TEXT_SIZE, POPOVER_RADIUS,
    SELECTION_BUTTON_SIZE, SELECTION_TOOLBAR_GAP, SELECTION_TOOLBAR_HEIGHT,
    SELECTION_TOOLBAR_WIDTH, SELECTION_TYPE_WIDTH, SPACE_2, SPACE_4, SPACE_6, SPACE_8, SPACE_10,
    SPACE_12, SPACE_24, SPACE_36, STROKE_HAIRLINE,
};

use super::super::super::block_selection::{
    BlockFormat, block_has_format, text_has_format, turn_into_kinds,
};
use super::super::super::commands::Command;
use super::super::super::icons::{
    draw_chevron, draw_close, draw_lines, draw_note, draw_page, icon_button,
};
use super::super::super::palette::Palette;
use super::super::super::view::ViewState;
use super::blocks::TextSelection;

enum Target<'a> {
    Blocks(Vec<&'a Block>),
    Text(&'a Block, &'a TextSelection),
}

impl Target<'_> {
    fn first_block(&self) -> &Block {
        match self {
            Target::Blocks(blocks) => blocks[0],
            Target::Text(block, _) => block,
        }
    }

    fn type_label(&self, common: Option<BlockKind>) -> &'static str {
        common.map_or("Mixed", |kind| kind.label())
    }
}

/// Paint the toolbar and, when enabled, its block-type menu.
#[allow(clippy::too_many_arguments)]
pub(super) fn render_block_toolbar(
    ctx: &egui::Context,
    anchor: Rect,
    blocks: &[Block],
    inline_comments: bool,
    text_selection: Option<&TextSelection>,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let selected: Vec<&Block> = blocks
        .iter()
        .filter(|block| view.page_selection.iter().any(|id| id == &block.id))
        .collect();
    let target = if selected.is_empty() {
        let Some(text) = text_selection else {
            return;
        };
        let Some(block) = blocks.iter().find(|block| block.id == text.block) else {
            return;
        };
        Target::Text(block, text)
    } else {
        Target::Blocks(selected)
    };

    let common = match &target {
        Target::Blocks(blocks) => {
            let first = blocks[0].kind;
            blocks
                .iter()
                .all(|block| block.kind == first)
                .then_some(first)
        }
        Target::Text(block, _) => Some(block.kind),
    };
    let formats = [
        BlockFormat::Bold,
        BlockFormat::Italic,
        BlockFormat::Underline,
        BlockFormat::Strikethrough,
        BlockFormat::Code,
    ];
    let active: Vec<bool> = formats
        .iter()
        .map(|format| match &target {
            Target::Blocks(blocks) => blocks.iter().all(|block| block_has_format(block, *format)),
            Target::Text(block, text) => text_has_format(block, text.start, text.end, *format),
        })
        .collect();

    let screen = ctx.viewport_rect();
    let prefer_below = matches!(target, Target::Text(_, _));
    let mut pos = Pos2::new(anchor.right() + SELECTION_TOOLBAR_GAP, anchor.top());
    if prefer_below {
        pos.x = (anchor.right() - SELECTION_TOOLBAR_WIDTH)
            .max(screen.left() + SELECTION_TOOLBAR_GAP)
            .min(screen.right() - SELECTION_TOOLBAR_WIDTH - SELECTION_TOOLBAR_GAP);
        pos.y = anchor.bottom() + SELECTION_TOOLBAR_GAP;
        if pos.y + SELECTION_TOOLBAR_HEIGHT > screen.bottom() - SELECTION_TOOLBAR_GAP {
            pos.y = anchor.top() - SELECTION_TOOLBAR_HEIGHT - SELECTION_TOOLBAR_GAP;
        }
    } else {
        if pos.x + SELECTION_TOOLBAR_WIDTH > screen.right() - SELECTION_TOOLBAR_GAP {
            pos.x = anchor.left() - SELECTION_TOOLBAR_WIDTH - SELECTION_TOOLBAR_GAP;
        }
        pos.x = pos.x.max(screen.left() + SELECTION_TOOLBAR_GAP);
    }
    pos.y = pos.y.clamp(
        screen.top() + SELECTION_TOOLBAR_GAP,
        (screen.bottom() - SELECTION_TOOLBAR_HEIGHT - SELECTION_TOOLBAR_GAP).max(screen.top()),
    );

    let area = egui::Area::new(egui::Id::unique("block_selection_toolbar"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos)
        .show(ctx, |ui| {
            egui::Frame::NONE
                .fill(palette.content)
                .corner_radius(CornerRadius::same(POPOVER_RADIUS))
                .stroke(Stroke::new(STROKE_HAIRLINE, palette.border))
                .inner_margin(Margin::symmetric(SPACE_4 as i8, SPACE_4 as i8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(SPACE_2, 0.0);
                        let label = target.type_label(common);
                        if type_button(ui, label, palette).clicked() {
                            commands.push(Command::ToggleBlockToolbarMenu);
                        }
                        divider(ui, palette);
                        for (format, is_active) in formats.iter().zip(&active) {
                            if format_button(ui, *format, *is_active, palette)
                                .on_hover_text(format.name())
                                .clicked()
                            {
                                match &target {
                                    Target::Blocks(_) => {
                                        commands.push(Command::ToggleSelectedBlocksFormat(*format))
                                    }
                                    Target::Text(block, text) => {
                                        commands.push(Command::ToggleBlockTextFormat {
                                            block: block.id.clone(),
                                            start: text.start,
                                            end: text.end,
                                            format: *format,
                                        });
                                    }
                                }
                            }
                        }
                        divider(ui, palette);
                        if icon_button(ui, SELECTION_BUTTON_SIZE, palette, draw_lines)
                            .on_hover_text("Copy selected text")
                            .clicked()
                        {
                            match &target {
                                Target::Blocks(_) => commands.push(Command::CopyBlockSelection),
                                Target::Text(block, text) => {
                                    commands.push(Command::CopyBlockText {
                                        block: block.id.clone(),
                                        start: text.start,
                                        end: text.end,
                                    });
                                }
                            }
                        }
                        if icon_button(ui, SELECTION_BUTTON_SIZE, palette, draw_note)
                            .on_hover_text("Comment on the selection")
                            .clicked()
                        {
                            commands.push(Command::SetInlineCommentBlock(Some(
                                target.first_block().id.clone(),
                            )));
                            if !inline_comments {
                                commands.push(Command::TogglePageInlineComments);
                            }
                        }
                        match &target {
                            Target::Blocks(_) => {
                                if icon_button(ui, SELECTION_BUTTON_SIZE, palette, draw_page)
                                    .on_hover_text("Duplicate selected blocks")
                                    .clicked()
                                {
                                    commands.push(Command::DuplicateBlockSelection);
                                }
                                if icon_button(ui, SELECTION_BUTTON_SIZE, palette, draw_close)
                                    .on_hover_text("Delete selected blocks")
                                    .clicked()
                                {
                                    commands.push(Command::DeleteBlockSelection);
                                }
                            }
                            Target::Text(block, _) => {
                                if icon_button(ui, SELECTION_BUTTON_SIZE, palette, draw_close)
                                    .on_hover_text("Delete this block")
                                    .clicked()
                                {
                                    commands.push(Command::DeleteBlock(block.id.clone()));
                                }
                            }
                        }
                    });
                });
        });

    if view.block_toolbar_menu {
        render_type_menu(ctx, area.response.rect, common, &target, palette, commands);
        let clicked = ctx.input(|i| i.pointer.any_click());
        if clicked && let Some(pointer) = ctx.input(|i| i.pointer.interact_pos()) {
            let inside = |id| {
                ctx.memory(|memory| memory.area_rect(id))
                    .is_some_and(|rect| rect.contains(pointer))
            };
            let inside_toolbar = inside(egui::Id::unique("block_selection_toolbar"));
            let inside_menu = inside(egui::Id::unique("block_type_menu"));
            if !inside_toolbar && !inside_menu {
                commands.push(Command::CloseBlockToolbarMenu);
            }
        }
    }
}

fn type_button(ui: &mut egui::Ui, label: &str, palette: &Palette) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(SELECTION_TYPE_WIDTH, SELECTION_BUTTON_SIZE),
        Sense::click(),
    );
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(MENU_ROW_RADIUS), palette.hover);
    }
    let y = rect.center().y;
    ui.painter().text(
        Pos2::new(rect.left() + SPACE_8, y),
        Align2::LEFT_CENTER,
        "T",
        FontId::proportional(MENU_HINT_SIZE),
        palette.muted,
    );
    ui.painter().text(
        Pos2::new(rect.left() + SPACE_24, y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(FONT_ROW),
        palette.text,
    );
    draw_chevron(
        ui.painter(),
        Rect::from_center_size(Pos2::new(rect.right() - SPACE_10, y), Vec2::splat(SPACE_8)),
        false,
        palette.muted,
    );
    response
}

fn divider(ui: &mut egui::Ui, palette: &Palette) {
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(SPACE_12, SELECTION_BUTTON_SIZE), Sense::hover());
    ui.painter().vline(
        rect.center().x,
        rect.shrink(SPACE_6).y_range(),
        Stroke::new(STROKE_HAIRLINE, palette.border),
    );
}

fn format_button(
    ui: &mut egui::Ui,
    format: BlockFormat,
    active: bool,
    palette: &Palette,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(
            if format == BlockFormat::Code {
                SPACE_36
            } else {
                SELECTION_BUTTON_SIZE
            },
            SELECTION_BUTTON_SIZE,
        ),
        Sense::click(),
    );
    if active {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(MENU_ROW_RADIUS), palette.selected);
    } else if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(MENU_ROW_RADIUS), palette.hover);
    }
    let color = if active { palette.accent } else { palette.text };
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        format.label(),
        FontId::proportional(if format == BlockFormat::Code {
            FONT_MICRO
        } else {
            FONT_ROW
        }),
        color,
    );
    response
}

fn render_type_menu(
    ctx: &egui::Context,
    toolbar: Rect,
    common: Option<BlockKind>,
    target: &Target<'_>,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let screen = ctx.viewport_rect();
    let mut pos = Pos2::new(toolbar.right() + SPACE_8, toolbar.top());
    if pos.x + BLOCK_TYPE_MENU_WIDTH > screen.right() - SPACE_8 {
        pos.x = toolbar.left() - BLOCK_TYPE_MENU_WIDTH - SPACE_8;
    }
    pos.x = pos.x.max(screen.left() + SPACE_8);
    pos.y = pos
        .y
        .clamp(screen.top() + SPACE_8, screen.bottom() - SPACE_10);

    egui::Area::new(egui::Id::unique("block_type_menu"))
        .order(egui::Order::Tooltip)
        .fixed_pos(pos)
        .show(ctx, |ui| {
            egui::Frame::NONE
                .fill(palette.content)
                .corner_radius(CornerRadius::same(POPOVER_RADIUS))
                .stroke(Stroke::new(STROKE_HAIRLINE, palette.border))
                .inner_margin(Margin::same(SPACE_6 as i8))
                .show(ui, |ui| {
                    ui.set_width(BLOCK_TYPE_MENU_WIDTH);
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, true])
                        .max_height(BLOCK_TYPE_MENU_MAX_HEIGHT)
                        .show(ui, |ui| {
                            for kind in turn_into_kinds() {
                                if type_row(ui, *kind, common == Some(*kind), palette).clicked() {
                                    commands.push(match target {
                                        Target::Blocks(_) => Command::SetSelectedBlocksKind(*kind),
                                        Target::Text(block, _) => {
                                            Command::SetBlockKind(block.id.clone(), *kind)
                                        }
                                    });
                                    commands.push(Command::CloseBlockToolbarMenu);
                                }
                            }
                        });
                });
        });
}

fn type_row(ui: &mut egui::Ui, kind: BlockKind, active: bool, palette: &Palette) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), MENU_ROW_HEIGHT),
        Sense::click(),
    );
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(MENU_ROW_RADIUS), palette.hover);
    }
    let y = rect.center().y;
    ui.painter().text(
        Pos2::new(rect.left() + SPACE_8, y),
        Align2::LEFT_CENTER,
        kind_glyph(kind),
        FontId::monospace(FONT_CAPTION),
        palette.muted,
    );
    ui.painter().text(
        Pos2::new(rect.left() + SPACE_36, y),
        Align2::LEFT_CENTER,
        kind.label(),
        FontId::proportional(MENU_TEXT_SIZE),
        palette.text,
    );
    if active {
        draw_check(
            ui.painter(),
            Rect::from_center_size(Pos2::new(rect.right() - SPACE_10, y), Vec2::splat(SPACE_10)),
            palette.accent,
        );
    }
    response
}

fn kind_glyph(kind: BlockKind) -> &'static str {
    match kind {
        BlockKind::Paragraph => "T",
        BlockKind::Heading1 => "H1",
        BlockKind::Heading2 => "H2",
        BlockKind::Heading3 => "H3",
        BlockKind::Heading4 => "H4",
        BlockKind::Bulleted => "*",
        BlockKind::Numbered => "1.",
        BlockKind::Todo => "[ ]",
        BlockKind::Toggle => ">",
        BlockKind::Code => "</>",
        BlockKind::Quote => "\"",
        BlockKind::Callout => "!",
        BlockKind::Divider => "---",
        BlockKind::Table => "Tb",
        BlockKind::TableRow => "R",
        BlockKind::ChildPage => "P",
        BlockKind::ColumnList => "[c]",
        BlockKind::Column => "c",
        BlockKind::Unsupported => "?",
    }
}

fn draw_check(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    painter.line(
        vec![
            c + Vec2::new(-SPACE_4, 0.0),
            c + Vec2::new(-1.0, SPACE_4),
            c + Vec2::new(SPACE_4, -SPACE_4),
        ],
        Stroke::new(STROKE_HAIRLINE + 1.0, color),
    );
}
