//! The shared "Search" / "Open in new tab" window (DRY, spec-013).
//!
//! The two opened states in the attachments use the *same* layout: a hint
//! field, show/hide highlight-pane and filter toggles, a grouped result list
//! (Today / Past week), an optional preview pane and a footer of keyboard
//! hints. Only the hint text and the activation behaviour differ, so one
//! renderer serves both.

use crate::model::Workspace;
use egui::{Align2, Color32, CornerRadius, FontId, Margin, Pos2, Rect, Sense, Stroke, Vec2};

use crate::ui::constants as metrics;

use super::app::{Command, NewTabChoice};
use super::icons::{draw_help, draw_page_icon, draw_search, draw_sliders, page_icon_letter};
use super::palette::{Palette, strong_font};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SwitcherMode {
    Search,
    NewTab,
}

/// One row in the result list. `id` is either a page id or one of the two
/// special keys used by the new-tab switcher.
pub(super) struct SwitcherRow {
    pub label: String,
    pub detail: String,
    pub group: &'static str,
    pub icon: String,
    pub id: String,
    pub special: bool,
}

/// Everything the shared window needs. A DTO-ish snapshot keeps the renderer
/// pure: it only emits `Command`s (SRP).
pub(super) struct SwitcherSpec<'a> {
    pub id: &'static str,
    pub hint: &'a str,
    pub query: &'a str,
    pub rows: &'a [SwitcherRow],
    pub selected: usize,
    pub highlight: bool,
    pub filters: bool,
    pub title_only: bool,
    pub created_by: Option<&'a str>,
    pub in_page: Option<&'a str>,
    pub has_comments: bool,
    pub members: &'a [crate::collab::Member],
    pub mode: SwitcherMode,
    pub ws: &'a Workspace,
}

/// A small speech-bubble glyph for the "New chat" row.
fn draw_chat(painter: &egui::Painter, rect: Rect, color: Color32) {
    let bubble = Rect::from_min_size(rect.min + Vec2::new(1.0, 1.0), Vec2::new(14.0, 11.0));
    painter.rect_stroke(
        bubble,
        CornerRadius::same(5),
        Stroke::new(1.3, color),
        egui::StrokeKind::Inside,
    );
    painter.line(
        vec![
            bubble.min + Vec2::new(3.0, 10.0),
            bubble.min + Vec2::new(1.0, 14.0),
            bubble.min + Vec2::new(7.0, 10.0),
        ],
        Stroke::new(1.3, color),
    );
}

/// A group caption above a run of rows.
fn group_label(ui: &mut egui::Ui, text: &str, palette: &Palette) {
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), metrics::SPACE_24),
        Sense::hover(),
    );
    ui.painter().text(
        rect.min + Vec2::new(0.0, metrics::SPACE_14),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(metrics::FONT_MICRO),
        palette.muted,
    );
}

/// The footer hint strip shared by both windows.
fn switcher_footer(ui: &mut egui::Ui, palette: &Palette) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = metrics::SECTION_GAP;
        for (keys, label) in [
            ("Ctrl+↵", "Open in new tab"),
            ("Ctrl+L", "Copy link"),
            ("Shift+Ctrl+K", "Command Search"),
        ] {
            ui.label(
                egui::RichText::new(keys)
                    .size(metrics::FONT_MICRO)
                    .color(palette.muted),
            );
            ui.label(
                egui::RichText::new(label)
                    .size(metrics::FONT_MICRO)
                    .color(palette.muted),
            );
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(metrics::ICON_SIZE), Sense::hover());
            draw_sliders(ui.painter(), rect, palette.muted);
        });
    });
}

fn activate(row: &SwitcherRow, spec: &SwitcherSpec<'_>, commands: &mut Vec<Command>) {
    match spec.mode {
        SwitcherMode::NewTab => {
            let choice = if row.special && row.id == "new-chat" {
                NewTabChoice::NewChat
            } else if row.special && row.id == "new-page" {
                NewTabChoice::NewPage
            } else {
                NewTabChoice::Page(row.id.clone())
            };
            commands.push(Command::ActivateNewTab(choice));
        }
        SwitcherMode::Search => {
            if !row.special {
                commands.push(Command::SelectPage(row.id.clone()));
                commands.push(Command::CloseSearch);
            }
        }
    }
}

/// A soft skeleton for rows without a page preview.
fn preview_skeleton(ui: &mut egui::Ui, card: Rect, palette: &Palette) {
    ui.painter()
        .circle_filled(card.min + Vec2::new(60.0, 90.0), 22.0, palette.hover);
    for (i, width) in [220.0, 180.0, 120.0].iter().enumerate() {
        let y = card.min.y + 130.0 + i as f32 * 30.0;
        ui.painter().rect_filled(
            Rect::from_min_size(Pos2::new(card.min.x + 24.0, y), Vec2::new(*width, 9.0)),
            CornerRadius::same(4),
            palette.hover,
        );
    }
}

#[allow(clippy::too_many_lines)]
pub(super) fn render_switcher(
    ctx: &egui::Context,
    spec: &SwitcherSpec<'_>,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let screen = ctx.viewport_rect();
    let width = (if spec.highlight {
        metrics::SWITCHER_WIDTH_WIDE
    } else {
        metrics::SWITCHER_WIDTH
    })
    .min(screen.width() - metrics::SWITCHER_MARGIN);
    let height = metrics::SWITCHER_MAX_HEIGHT
        .min(screen.height() - metrics::SPACE_120)
        .max(metrics::SWITCHER_MIN_HEIGHT);
    let size = Vec2::new(width, height);
    let pos = Pos2::new(
        screen.left() + ((screen.width() - size.x) / 2.0).max(0.0),
        screen.top() + metrics::SWITCHER_TOP,
    );
    let resp = egui::Window::new(spec.id)
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_size(size)
        .fixed_pos(pos)
        .frame(
            egui::Frame::NONE
                .fill(palette.content)
                .corner_radius(CornerRadius::same(10))
                .inner_margin(Margin::same(16))
                .stroke(Stroke::new(metrics::STROKE_HAIRLINE, palette.border)),
        )
        .show(ctx, |ui| {
            // Search header + the two pane toggles.
            ui.horizontal(|ui| {
                ui.add_space(metrics::SPACE_4);
                let (icon, _) =
                    ui.allocate_exact_size(Vec2::splat(metrics::SPACE_18), Sense::hover());
                draw_search(ui.painter(), icon, palette.muted);
                ui.add_space(metrics::SPACE_6);
                let mut query = spec.query.to_string();
                let field = ui.add(
                    egui::TextEdit::singleline(&mut query)
                        .id(egui::Id::unique(spec.id).with("query"))
                        .frame(egui::Frame::NONE)
                        .font(FontId::proportional(metrics::FONT_BODY))
                        .desired_width(width - metrics::SWITCHER_HINT_WIDTH)
                        .hint_text(spec.hint)
                        .text_color(palette.text),
                );
                field.request_focus();
                if field.changed() {
                    match spec.mode {
                        SwitcherMode::NewTab => commands.push(Command::SetNewTabQuery(query)),
                        SwitcherMode::Search => commands.push(Command::SetSearchQuery(query)),
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (filter_rect, filter) =
                        ui.allocate_exact_size(Vec2::splat(metrics::SPACE_24), Sense::click());
                    if filter.hovered() {
                        ui.painter()
                            .rect_filled(filter_rect, CornerRadius::same(4), palette.hover);
                    }
                    draw_sliders(ui.painter(), filter_rect.shrink(4.0), palette.muted);
                    if filter.clicked() {
                        commands.push(Command::ToggleFilters);
                    }
                    filter.on_hover_text(if spec.filters {
                        "Hide filters"
                    } else {
                        "Show filters"
                    });
                    let highlight = ui
                        .add_sized(
                            Vec2::splat(metrics::SPACE_24),
                            egui::Button::new(
                                egui::RichText::new("▥")
                                    .size(metrics::FONT_ROW)
                                    .color(palette.muted),
                            )
                            .frame(false),
                        )
                        .on_hover_text(if spec.highlight {
                            "Hide highlight pane"
                        } else {
                            "Show highlight pane"
                        });
                    if highlight.clicked() {
                        commands.push(Command::ToggleHighlightPane);
                    }
                    let (help, _) =
                        ui.allocate_exact_size(Vec2::splat(metrics::SPACE_18), Sense::hover());
                    draw_help(ui.painter(), help, palette.accent);
                });
            });
            ui.add_space(metrics::SPACE_12);

            if spec.filters {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = metrics::SPACE_12;
                    ui.add_space(metrics::SPACE_24);
                    if ui
                        .selectable_label(
                            spec.title_only,
                            egui::RichText::new("Aa  Title only").size(metrics::FONT_CAPTION),
                        )
                        .clicked()
                    {
                        commands.push(Command::ToggleSearchTitleOnly);
                    }
                    let created = spec.created_by.unwrap_or("Created by");
                    egui::ComboBox::from_id_salt((spec.id, "created_by"))
                        .selected_text(egui::RichText::new(created).size(metrics::FONT_CAPTION))
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(spec.created_by.is_none(), "Anyone")
                                .clicked()
                            {
                                commands.push(Command::SetSearchCreatedBy(None));
                            }
                            for member in spec.members {
                                if ui
                                    .selectable_label(
                                        spec.created_by == Some(member.name.as_str()),
                                        &member.name,
                                    )
                                    .clicked()
                                {
                                    commands.push(Command::SetSearchCreatedBy(Some(
                                        member.name.clone(),
                                    )));
                                }
                            }
                        });
                    let in_label = spec
                        .in_page
                        .and_then(|id| spec.ws.page(id))
                        .map(|page| page.title.as_str())
                        .unwrap_or("In");
                    egui::ComboBox::from_id_salt((spec.id, "in_page"))
                        .selected_text(egui::RichText::new(in_label).size(metrics::FONT_CAPTION))
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(spec.in_page.is_none(), "Anywhere")
                                .clicked()
                            {
                                commands.push(Command::SetSearchIn(None));
                            }
                            for page in &spec.ws.pages {
                                if ui
                                    .selectable_label(
                                        spec.in_page == Some(page.id.as_str()),
                                        &page.title,
                                    )
                                    .clicked()
                                {
                                    commands.push(Command::SetSearchIn(Some(page.id.clone())));
                                }
                            }
                        });
                    if spec.has_comments {
                        if ui
                            .selectable_label(
                                true,
                                egui::RichText::new("Has comments  ×").size(metrics::FONT_CAPTION),
                            )
                            .clicked()
                        {
                            commands.push(Command::ToggleSearchHasComments);
                        }
                    } else if ui
                        .button(egui::RichText::new("+ Filter").size(metrics::FONT_CAPTION))
                        .clicked()
                    {
                        commands.push(Command::ToggleSearchHasComments);
                    }
                });
                ui.add_space(metrics::SPACE_8);
            }
            ui.separator();
            ui.add_space(metrics::SPACE_8);

            let body_height = (ui.available_height() - metrics::SWITCHER_BODY_OFFSET)
                .max(metrics::SWITCHER_BODY_MIN);
            let list_width = if spec.highlight {
                (width - metrics::SPACE_32) * (metrics::SWITCHER_LIST_PERCENT / 100.0)
            } else {
                ui.available_width()
            };

            ui.horizontal_top(|ui| {
                // Result list.
                ui.vertical(|ui| {
                    ui.set_width(list_width);
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .max_height(body_height)
                        .show(ui, |ui| {
                            let mut last_group = "";
                            for (index, row) in spec.rows.iter().enumerate() {
                                if !row.group.is_empty() && row.group != last_group {
                                    group_label(ui, row.group, palette);
                                    last_group = row.group;
                                }
                                let (rect, response) = ui.allocate_exact_size(
                                    Vec2::new(ui.available_width(), metrics::SWITCHER_ROW_HEIGHT),
                                    Sense::click(),
                                );
                                let selected = spec.selected == index;
                                if selected {
                                    ui.painter().rect_filled(
                                        rect,
                                        CornerRadius::same(6),
                                        palette.selected,
                                    );
                                } else if response.hovered() {
                                    ui.painter().rect_filled(
                                        rect,
                                        CornerRadius::same(6),
                                        palette.hover,
                                    );
                                }
                                let icon_rect = Rect::from_min_size(
                                    rect.min
                                        + Vec2::new(metrics::ROW_ICON_INSET, metrics::SPACE_10),
                                    Vec2::splat(metrics::ICON_SIZE),
                                );
                                if row.special && row.id == "new-chat" {
                                    draw_chat(ui.painter(), icon_rect, palette.muted);
                                } else {
                                    draw_page_icon(
                                        ui.painter(),
                                        icon_rect,
                                        &page_icon_letter(&row.icon, &row.label),
                                        palette,
                                        false,
                                    );
                                }
                                ui.painter().text(
                                    rect.min
                                        + Vec2::new(metrics::ROW_LABEL_INSET, metrics::SPACE_18),
                                    Align2::LEFT_CENTER,
                                    &row.label,
                                    FontId::proportional(metrics::FONT_ROW),
                                    palette.text,
                                );
                                if !row.detail.is_empty() {
                                    let x =
                                        rect.min.x + 40.0 + row.label.chars().count() as f32 * 7.2;
                                    ui.painter().text(
                                        Pos2::new(x, rect.min.y + metrics::SPACE_18),
                                        Align2::LEFT_CENTER,
                                        &row.detail,
                                        FontId::proportional(metrics::FONT_CAPTION),
                                        palette.muted,
                                    );
                                }
                                if selected {
                                    ui.painter().text(
                                        rect.max
                                            + Vec2::new(-metrics::SPACE_12, -metrics::SPACE_18),
                                        Align2::RIGHT_CENTER,
                                        "↵",
                                        FontId::proportional(metrics::FONT_ROW),
                                        palette.muted,
                                    );
                                }
                                if response.hovered() && !selected {
                                    match spec.mode {
                                        SwitcherMode::NewTab => {
                                            commands.push(Command::SetNewTabSelected(index))
                                        }
                                        SwitcherMode::Search => {
                                            commands.push(Command::SetSearchSelected(index))
                                        }
                                    }
                                }
                                if response.clicked() {
                                    activate(row, spec, commands);
                                }
                            }
                        });
                });

                // Preview ("highlight") pane.
                if spec.highlight {
                    ui.add_space(metrics::SECTION_GAP);
                    let preview_width = width - metrics::SPACE_32 - list_width - metrics::SPACE_18;
                    ui.vertical(|ui| {
                        ui.set_width(preview_width);
                        let (card, _) = ui.allocate_exact_size(
                            Vec2::new(preview_width, body_height),
                            Sense::hover(),
                        );
                        ui.painter()
                            .rect_filled(card, CornerRadius::same(8), palette.callout);
                        ui.painter().rect_stroke(
                            card,
                            CornerRadius::same(8),
                            Stroke::new(metrics::STROKE_HAIRLINE, palette.border),
                            egui::StrokeKind::Inside,
                        );
                        let selected = spec.rows.get(spec.selected);
                        match selected {
                            Some(row) if !row.special => {
                                let page = spec.ws.page(&row.id);
                                if let Some(page) = page {
                                    draw_page_icon(
                                        ui.painter(),
                                        Rect::from_min_size(
                                            card.min + Vec2::new(24.0, 28.0),
                                            Vec2::splat(48.0),
                                        ),
                                        &page_icon_letter(&page.icon, &page.title),
                                        palette,
                                        true,
                                    );
                                    ui.painter().text(
                                        card.min + Vec2::new(24.0, 100.0),
                                        Align2::LEFT_CENTER,
                                        &spec.ws.name,
                                        FontId::proportional(metrics::FONT_MICRO),
                                        palette.muted,
                                    );
                                    ui.painter().text(
                                        card.min + Vec2::new(24.0, 126.0),
                                        Align2::LEFT_CENTER,
                                        &page.title,
                                        strong_font(metrics::SPACE_22),
                                        palette.text,
                                    );
                                    let snippet: String =
                                        page.plain_text().chars().take(220).collect();
                                    let mut y = card.min.y + 174.0;
                                    for line in wrap_text(&snippet, 44) {
                                        ui.painter().text(
                                            Pos2::new(card.min.x + 24.0, y),
                                            Align2::LEFT_CENTER,
                                            line,
                                            FontId::proportional(metrics::FONT_CAPTION),
                                            palette.muted,
                                        );
                                        y += metrics::SPACE_20;
                                        if y > card.max.y - metrics::SPACE_20 {
                                            break;
                                        }
                                    }
                                } else {
                                    preview_skeleton(ui, card, palette);
                                }
                            }
                            _ => preview_skeleton(ui, card, palette),
                        }
                    });
                }
            });
            ui.add_space(metrics::SPACE_8);
            ui.separator();
            ui.add_space(metrics::SPACE_6);
            switcher_footer(ui, palette);
        });
    if let Some(inner) = resp {
        let rect = inner.response.rect;
        if ctx.input(|i| i.pointer.any_click())
            && ctx.input(|i| i.pointer.interact_pos().is_some_and(|p| !rect.contains(p)))
        {
            match spec.mode {
                SwitcherMode::NewTab => commands.push(Command::CloseNewTab),
                SwitcherMode::Search => commands.push(Command::CloseSearch),
            }
        }
    }
}

/// Rough word wrap for the preview text (display only).
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if line.len() + word.len() + 1 > width && !line.is_empty() {
            lines.push(line.clone());
            line.clear();
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}
