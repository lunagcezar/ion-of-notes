//! Floating toolbar for text and page-level block selections (specs 157-158).
//!
//! It renders beside the selection, offers rich inline/block formatting and
//! opens one compact panel at a time. Every click becomes a `Command`; nothing
//! is mutated here.

use crate::model::{Block, BlockKind};
use egui::{Align2, Color32, CornerRadius, FontId, Margin, Pos2, Rect, Sense, Stroke, Vec2};

use crate::ui::constants::{
    BLOCK_TYPE_MENU_MAX_HEIGHT, BLOCK_TYPE_MENU_WIDTH, FONT_CAPTION, FONT_MICRO, FONT_ROW,
    MENU_HINT_SIZE, MENU_ROW_HEIGHT, MENU_ROW_RADIUS, MENU_TEXT_SIZE, POPOVER_RADIUS,
    SELECTION_BUTTON_SIZE, SELECTION_TOOLBAR_GAP, SELECTION_TOOLBAR_HEIGHT,
    SELECTION_TOOLBAR_WIDTH, SELECTION_TYPE_WIDTH, SPACE_2, SPACE_4, SPACE_6, SPACE_8, SPACE_10,
    SPACE_12, SPACE_16, SPACE_24, SPACE_28, SPACE_36, STROKE_HAIRLINE,
};

use super::super::super::block_selection::{
    BlockFormat, block_has_format, text_color as range_color, text_has_format, text_has_link,
    text_is_equation, turn_into_kinds,
};
use super::super::super::commands::{BlockToolbarPanel, Command};
use super::super::super::emoji_picker::emoji_grid;
use super::super::super::icons::{
    draw_chevron, draw_ellipsis, draw_link, draw_note, draw_page_icon, draw_person, icon_button,
    page_icon_letter,
};
use super::super::super::palette::{Palette, TEXT_COLOR_NAMES, text_color as palette_text_color};
use super::super::super::view::ViewState;
use super::blocks::TextSelection;

/// A page title/icon pair shown as a recent link target.
pub(super) type RecentPage = (String, String, String);

enum Target<'a> {
    Blocks(Vec<&'a Block>),
    Text(&'a Block, TextSelection),
}

impl Target<'_> {
    fn type_label(&self, common: Option<BlockKind>) -> &'static str {
        common.map_or("Mixed", |kind| kind.label())
    }

    fn color(&self) -> String {
        match self {
            Target::Text(block, text) => {
                range_color(block, text.start, text.end).unwrap_or_else(|| "default".to_string())
            }
            Target::Blocks(blocks) => {
                let block = blocks[0];
                let len = block.text.chars().count();
                range_color(block, 0, len).unwrap_or_else(|| "default".to_string())
            }
        }
    }

    fn has_link(&self) -> bool {
        match self {
            Target::Text(block, text) => text_has_link(block, text.start, text.end),
            Target::Blocks(blocks) => blocks.iter().all(|block| {
                let len = block.text.chars().count();
                text_has_link(block, 0, len)
            }),
        }
    }
}

/// Paint the toolbar and, when enabled, its open panel.
#[allow(clippy::too_many_arguments)]
pub(super) fn render_block_toolbar(
    ctx: &egui::Context,
    anchor: Rect,
    blocks: &[Block],
    recent_pages: &[RecentPage],
    text_selection: Option<&TextSelection>,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let selected: Vec<&Block> = blocks
        .iter()
        .filter(|block| view.page_selection.iter().any(|id| id == &block.id))
        .collect();
    let target = if !selected.is_empty() {
        Target::Blocks(selected)
    } else {
        let text = text_selection.cloned().or_else(|| {
            view.inline_selection
                .as_ref()
                .and_then(|(block, start, end)| {
                    blocks
                        .iter()
                        .find(|candidate| &candidate.id == block)
                        .map(|_| TextSelection {
                            block: block.clone(),
                            start: *start,
                            end: *end,
                            rect: anchor,
                        })
                })
        });
        let Some(text) = text else {
            return;
        };
        let Some(block) = blocks.iter().find(|block| block.id == text.block) else {
            return;
        };
        Target::Text(block, text)
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
                            open_text_panel(&target, BlockToolbarPanel::Type, commands);
                        }
                        if color_button(ui, &target.color(), palette).clicked() {
                            open_text_panel(&target, BlockToolbarPanel::Color, commands);
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
                        if icon_button(ui, SELECTION_BUTTON_SIZE, palette, draw_link)
                            .on_hover_text(if target.has_link() {
                                "Edit link"
                            } else {
                                "Add link"
                            })
                            .clicked()
                        {
                            open_text_panel(&target, BlockToolbarPanel::Link, commands);
                        }
                        if icon_button(ui, SELECTION_BUTTON_SIZE, palette, draw_note)
                            .on_hover_text("Comment")
                            .clicked()
                        {
                            open_text_panel(&target, BlockToolbarPanel::Comment, commands);
                        }
                        if react_button(ui, palette).clicked() {
                            open_text_panel(&target, BlockToolbarPanel::React, commands);
                        }
                        if let Target::Text(block, text) = &target
                            && equation_button(
                                ui,
                                text_is_equation(block, text.start, text.end),
                                palette,
                            )
                            .on_hover_text("Mark as equation")
                            .clicked()
                        {
                            commands.push(Command::MarkBlockTextEquation {
                                block: block.id.clone(),
                                start: text.start,
                                end: text.end,
                            });
                        }
                        if icon_button(ui, SELECTION_BUTTON_SIZE, palette, draw_ellipsis)
                            .on_hover_text("More options")
                            .clicked()
                        {
                            open_text_panel(&target, BlockToolbarPanel::More, commands);
                        }
                    });
                });
        });
    ctx.data_mut(|data| {
        data.insert_temp(egui::Id::unique("block_toolbar_anchor"), area.response.rect);
    });

    let Some(panel) = view.block_toolbar_panel else {
        return;
    };
    let panel_rect = match panel {
        BlockToolbarPanel::Type => {
            render_type_menu(ctx, area.response.rect, common, &target, palette, commands)
        }
        BlockToolbarPanel::Color => {
            render_color_panel(ctx, area.response.rect, &target, palette, commands)
        }
        BlockToolbarPanel::Link => render_link_panel(
            ctx,
            area.response.rect,
            &target,
            recent_pages,
            view,
            palette,
            commands,
        ),
        BlockToolbarPanel::Comment => {
            render_comment_panel(ctx, area.response.rect, view, palette, commands)
        }
        BlockToolbarPanel::React => {
            render_react_panel(ctx, area.response.rect, &target, palette, commands)
        }
        BlockToolbarPanel::More => {
            render_more_panel(ctx, area.response.rect, &target, palette, commands)
        }
    };

    let Some(panel_rect) = panel_rect else {
        ctx.data_mut(|data| {
            data.insert_temp(egui::Id::unique("block_toolbar_hit"), area.response.rect);
        });
        return;
    };
    ctx.data_mut(|data| {
        data.insert_temp(
            egui::Id::unique("block_toolbar_hit"),
            area.response.rect.union(panel_rect),
        );
    });

    if ctx.input(|i| i.pointer.any_click())
        && let Some(pointer) = ctx.input(|i| i.pointer.interact_pos())
        && !area.response.rect.contains(pointer)
        && !panel_rect.contains(pointer)
    {
        commands.push(Command::CloseBlockToolbarPanel);
    }
}

fn open_text_panel(target: &Target<'_>, panel: BlockToolbarPanel, commands: &mut Vec<Command>) {
    match target {
        Target::Text(block, text) => {
            commands.push(Command::SetInlineSelection {
                block: block.id.clone(),
                start: text.start,
                end: text.end,
            });
        }
        Target::Blocks(blocks) => {
            let block = blocks[0];
            commands.push(Command::SetInlineSelection {
                block: block.id.clone(),
                start: 0,
                end: block.text.chars().count(),
            });
        }
    }
    commands.push(Command::ToggleBlockToolbarPanel(panel));
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

fn color_button(ui: &mut egui::Ui, color: &str, palette: &Palette) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::splat(SELECTION_BUTTON_SIZE), Sense::click());
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(MENU_ROW_RADIUS), palette.hover);
    }
    let swatch = palette_text_color(color, palette.dark);
    ui.painter().text(
        rect.center() - Vec2::new(0.0, SPACE_2),
        Align2::CENTER_CENTER,
        "A",
        FontId::proportional(FONT_ROW),
        palette.text,
    );
    ui.painter().hline(
        (rect.left() + SPACE_6)..=(rect.right() - SPACE_6),
        rect.bottom() - SPACE_4,
        Stroke::new(2.0, swatch),
    );
    response
}

fn react_button(ui: &mut egui::Ui, palette: &Palette) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::splat(SELECTION_BUTTON_SIZE), Sense::click());
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(MENU_ROW_RADIUS), palette.hover);
    }
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        "☺",
        FontId::proportional(FONT_ROW),
        palette.text,
    );
    response
}

fn equation_button(ui: &mut egui::Ui, active: bool, palette: &Palette) -> egui::Response {
    format_label_button(ui, "fx", active, palette)
}

fn format_label_button(
    ui: &mut egui::Ui,
    label: &str,
    active: bool,
    palette: &Palette,
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::splat(SELECTION_BUTTON_SIZE), Sense::click());
    if active {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(MENU_ROW_RADIUS), palette.selected);
    } else if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(MENU_ROW_RADIUS), palette.hover);
    }
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(FONT_MICRO),
        if active { palette.accent } else { palette.text },
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
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        format.label(),
        FontId::proportional(if format == BlockFormat::Code {
            FONT_MICRO
        } else {
            FONT_ROW
        }),
        if active { palette.accent } else { palette.text },
    );
    response
}

/// Shared floating panel frame. Returns the panel rect, or `None` when the
/// panel did not render.
fn panel_area(
    ctx: &egui::Context,
    toolbar: Rect,
    width: f32,
    height: f32,
    palette: &Palette,
    add: impl FnOnce(&mut egui::Ui),
) -> Option<Rect> {
    let screen = ctx.viewport_rect();
    let mut pos = Pos2::new(toolbar.right() + SPACE_8, toolbar.top());
    if pos.x + width > screen.right() - SPACE_8 {
        pos.x = toolbar.left() - width - SPACE_8;
    }
    pos.x = pos.x.max(screen.left() + SPACE_8);
    pos.y = pos.y.clamp(
        screen.top() + SPACE_8,
        (screen.bottom() - height - SPACE_8).max(screen.top()),
    );
    let response = egui::Area::new(egui::Id::unique("block_toolbar_panel"))
        .order(egui::Order::Tooltip)
        .fixed_pos(pos)
        .show(ctx, |ui| {
            egui::Frame::NONE
                .fill(palette.content)
                .corner_radius(CornerRadius::same(POPOVER_RADIUS))
                .stroke(Stroke::new(STROKE_HAIRLINE, palette.border))
                .inner_margin(Margin::same(SPACE_6 as i8))
                .show(ui, |ui| {
                    ui.set_width(width);
                    add(ui);
                });
        });
    Some(response.response.rect)
}

fn render_type_menu(
    ctx: &egui::Context,
    toolbar: Rect,
    common: Option<BlockKind>,
    target: &Target<'_>,
    palette: &Palette,
    commands: &mut Vec<Command>,
) -> Option<Rect> {
    panel_area(
        ctx,
        toolbar,
        BLOCK_TYPE_MENU_WIDTH,
        BLOCK_TYPE_MENU_MAX_HEIGHT,
        palette,
        |ui| {
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
                            commands.push(Command::CloseBlockToolbarPanel);
                        }
                    }
                });
        },
    )
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

fn render_color_panel(
    ctx: &egui::Context,
    toolbar: Rect,
    target: &Target<'_>,
    palette: &Palette,
    commands: &mut Vec<Command>,
) -> Option<Rect> {
    let current = target.color();
    panel_area(ctx, toolbar, 220.0, 220.0, palette, |ui| {
        ui.label(
            egui::RichText::new("Text color")
                .size(FONT_CAPTION)
                .color(palette.muted),
        );
        ui.add_space(SPACE_6);
        for chunk in TEXT_COLOR_NAMES.chunks(5) {
            ui.horizontal(|ui| {
                for name in chunk {
                    let selected = current == *name;
                    let (rect, response) =
                        ui.allocate_exact_size(Vec2::splat(SELECTION_BUTTON_SIZE), Sense::click());
                    if response.hovered() {
                        ui.painter().rect_filled(
                            rect,
                            CornerRadius::same(MENU_ROW_RADIUS),
                            palette.hover,
                        );
                    }
                    ui.painter().circle_filled(
                        rect.center(),
                        SPACE_8,
                        palette_text_color(name, palette.dark),
                    );
                    if selected {
                        ui.painter().circle_stroke(
                            rect.center(),
                            SPACE_10,
                            Stroke::new(2.0, palette.accent),
                        );
                    }
                    if response.clicked() {
                        match target {
                            Target::Blocks(_) => commands
                                .push(Command::SetSelectedBlocksTextColor((*name).to_string())),
                            Target::Text(block, text) => {
                                commands.push(Command::SetBlockTextColor {
                                    block: block.id.clone(),
                                    start: text.start,
                                    end: text.end,
                                    color: (*name).to_string(),
                                });
                            }
                        }
                        commands.push(Command::CloseBlockToolbarPanel);
                    }
                }
            });
        }
    })
}

fn render_link_panel(
    ctx: &egui::Context,
    toolbar: Rect,
    target: &Target<'_>,
    recent_pages: &[RecentPage],
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) -> Option<Rect> {
    panel_area(ctx, toolbar, 320.0, 320.0, palette, |ui| {
        let mut value = view.inline_link_input.clone();
        let field = ui.add(
            egui::TextEdit::singleline(&mut value)
                .desired_width(f32::INFINITY)
                .hint_text("Paste a link or search pages")
                .font(FontId::proportional(FONT_ROW))
                .text_color(palette.text),
        );
        if field.changed() {
            commands.push(Command::SetInlineLinkInput(value.clone()));
        }
        let apply = ui
            .add(
                egui::Button::new(
                    egui::RichText::new("Apply link")
                        .size(FONT_CAPTION)
                        .color(Color32::WHITE),
                )
                .fill(palette.accent)
                .corner_radius(CornerRadius::same(MENU_ROW_RADIUS)),
            )
            .clicked()
            || (field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
        if apply && !value.trim().is_empty() {
            apply_link(target, value.trim(), commands);
            commands.push(Command::CloseBlockToolbarPanel);
        }
        ui.add_space(SPACE_8);
        ui.label(
            egui::RichText::new("Recent pages")
                .size(FONT_MICRO)
                .color(palette.muted),
        );
        for (id, title, icon) in recent_pages {
            if page_menu_row(ui, icon, title, palette).clicked() {
                apply_link(target, &format!("ion://page/{id}"), commands);
                commands.push(Command::CloseBlockToolbarPanel);
            }
        }
    })
}

fn apply_link(target: &Target<'_>, url: &str, commands: &mut Vec<Command>) {
    match target {
        Target::Blocks(_) => commands.push(Command::SetSelectedBlocksLink(url.to_string())),
        Target::Text(block, text) => {
            commands.push(Command::SetBlockTextLink {
                block: block.id.clone(),
                start: text.start,
                end: text.end,
                url: url.to_string(),
            });
        }
    }
}

fn render_comment_panel(
    ctx: &egui::Context,
    toolbar: Rect,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) -> Option<Rect> {
    panel_area(ctx, toolbar, 340.0, 300.0, palette, |ui| {
        let mut value = view.inline_comment_input.clone();
        let field = ui.add(
            egui::TextEdit::multiline(&mut value)
                .desired_width(f32::INFINITY)
                .desired_rows(2)
                .hint_text("Add a comment…")
                .font(FontId::proportional(FONT_ROW))
                .text_color(palette.text),
        );
        if field.changed() {
            commands.push(Command::SetInlineCommentInput(value));
        }
        let attach_id = egui::Id::unique("inline_comment_attach_open");
        let mut attach_open = ctx.data(|data| data.get_temp::<bool>(attach_id).unwrap_or(false));
        ui.horizontal(|ui| {
            if icon_button(ui, SELECTION_BUTTON_SIZE, palette, draw_link)
                .on_hover_text("Attach a file")
                .clicked()
            {
                attach_open = !attach_open;
            }
            if icon_button(ui, SELECTION_BUTTON_SIZE, palette, draw_person)
                .on_hover_text("Mention a person")
                .clicked()
            {
                commands.push(Command::ToggleInlineCommentMentions);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Send")
                                .size(FONT_CAPTION)
                                .color(Color32::WHITE),
                        )
                        .fill(palette.accent)
                        .corner_radius(CornerRadius::same(MENU_ROW_RADIUS)),
                    )
                    .clicked()
                {
                    commands.push(Command::AddInlineComment);
                    commands.push(Command::CloseBlockToolbarPanel);
                }
            });
        });
        ctx.data_mut(|data| data.insert_temp(attach_id, attach_open));
        if attach_open {
            let mut path = view.inline_comment_attachment.clone();
            let response = ui.add(
                egui::TextEdit::singleline(&mut path)
                    .desired_width(f32::INFINITY)
                    .hint_text("File path to attach")
                    .font(FontId::proportional(FONT_ROW))
                    .text_color(palette.text),
            );
            if response.changed() {
                commands.push(Command::SetInlineCommentAttachment(path));
            }
        }
        if view.inline_comment_mention_open {
            for member in view.sharing.members.iter().take(8) {
                if menu_row(ui, &member.name, palette).clicked() {
                    commands.push(Command::MentionInlineComment(member.name.clone()));
                }
            }
        }
        if let Some(status) = &view.inline_comment_status {
            ui.label(
                egui::RichText::new(status)
                    .size(FONT_MICRO)
                    .color(palette.muted),
            );
        }
    })
}

fn render_react_panel(
    ctx: &egui::Context,
    toolbar: Rect,
    target: &Target<'_>,
    palette: &Palette,
    commands: &mut Vec<Command>,
) -> Option<Rect> {
    panel_area(ctx, toolbar, 340.0, 300.0, palette, |ui| {
        let query_id = egui::Id::unique("reaction_query");
        let mut query = ctx.data(|data| data.get_temp::<String>(query_id).unwrap_or_default());
        let field = ui.add(
            egui::TextEdit::singleline(&mut query)
                .desired_width(f32::INFINITY)
                .hint_text("Filter…")
                .font(FontId::proportional(FONT_ROW))
                .text_color(palette.text),
        );
        if field.changed() {
            ctx.data_mut(|data| data.insert_temp(query_id, query.clone()));
        }
        if let Some(emoji) = emoji_grid(ui, &query, None, palette, 230.0) {
            match target {
                Target::Blocks(_) => commands.push(Command::ToggleSelectedBlocksReaction(emoji)),
                Target::Text(block, text) => commands.push(Command::ToggleBlockReaction {
                    block: block.id.clone(),
                    start: text.start,
                    end: text.end,
                    emoji,
                }),
            }
            commands.push(Command::CloseBlockToolbarPanel);
        }
    })
}

fn render_more_panel(
    ctx: &egui::Context,
    toolbar: Rect,
    target: &Target<'_>,
    palette: &Palette,
    commands: &mut Vec<Command>,
) -> Option<Rect> {
    panel_area(ctx, toolbar, 220.0, 300.0, palette, |ui| {
        let row = |ui: &mut egui::Ui, label: &str| menu_row(ui, label, palette).clicked();
        if row(ui, "Clear formatting") {
            match target {
                Target::Blocks(_) => commands.push(Command::ClearSelectedBlocksFormatting),
                Target::Text(block, text) => {
                    commands.push(Command::ClearBlockTextFormatting {
                        block: block.id.clone(),
                        start: text.start,
                        end: text.end,
                    });
                }
            }
            commands.push(Command::CloseBlockToolbarPanel);
        }
        if row(ui, "Add link…") {
            open_text_panel(target, BlockToolbarPanel::Link, commands);
        }
        if row(ui, "Comment…") {
            open_text_panel(target, BlockToolbarPanel::Comment, commands);
        }
        if row(ui, "React…") {
            open_text_panel(target, BlockToolbarPanel::React, commands);
        }
        if row(ui, "Copy") {
            match target {
                Target::Blocks(_) => commands.push(Command::CopyBlockSelection),
                Target::Text(block, text) => commands.push(Command::CopyBlockText {
                    block: block.id.clone(),
                    start: text.start,
                    end: text.end,
                }),
            }
            commands.push(Command::CloseBlockToolbarPanel);
        }
        match target {
            Target::Blocks(_) => {
                if row(ui, "Duplicate") {
                    commands.push(Command::DuplicateBlockSelection);
                    commands.push(Command::CloseBlockToolbarPanel);
                }
                if row(ui, "Delete") {
                    commands.push(Command::DeleteBlockSelection);
                    commands.push(Command::CloseBlockToolbarPanel);
                }
            }
            Target::Text(block, _) => {
                if row(ui, "Delete block") {
                    commands.push(Command::DeleteBlock(block.id.clone()));
                    commands.push(Command::CloseBlockToolbarPanel);
                }
            }
        }
    })
}

fn menu_row(ui: &mut egui::Ui, label: &str, palette: &Palette) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), MENU_ROW_HEIGHT),
        Sense::click(),
    );
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(MENU_ROW_RADIUS), palette.hover);
    }
    ui.painter().text(
        rect.min + Vec2::new(SPACE_8, MENU_ROW_HEIGHT / 2.0),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(MENU_TEXT_SIZE),
        palette.text,
    );
    response
}

fn page_menu_row(ui: &mut egui::Ui, icon: &str, title: &str, palette: &Palette) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), MENU_ROW_HEIGHT),
        Sense::click(),
    );
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(MENU_ROW_RADIUS), palette.hover);
    }
    draw_page_icon(
        ui.painter(),
        Rect::from_min_size(
            rect.min + Vec2::new(SPACE_8, (MENU_ROW_HEIGHT - SPACE_16) / 2.0),
            Vec2::splat(SPACE_16),
        ),
        &page_icon_letter(icon, title),
        palette,
        false,
    );
    ui.painter().text(
        rect.min + Vec2::new(SPACE_28, MENU_ROW_HEIGHT / 2.0),
        Align2::LEFT_CENTER,
        title,
        FontId::proportional(MENU_TEXT_SIZE),
        palette.text,
    );
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
