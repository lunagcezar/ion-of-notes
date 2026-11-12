//! Settings window: navigation rail, search filtering and page routing.

use crate::i18n;
use crate::model::Workspace;
use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, Vec2};

use super::super::commands::Command;
use super::super::icons::{
    draw_grid, draw_help, draw_inbox, draw_link, draw_note, draw_page, draw_person, draw_search,
    draw_settings,
};
use super::super::palette::{Palette, strong_font};
use super::super::settings_search::{rows_for, section_matches, settings_matches, visible_rows};
use super::super::view::ViewState;
use super::super::widgets::section_label;
use super::settings_pages::{
    render_ai_settings, render_offline_settings, render_people_settings, render_preferences,
    render_server_settings, render_workspace_settings,
};

use crate::ui::constants::{
    CHIP_HEIGHT, CHIP_WIDTH, DIALOG_INSET, DIALOG_RADIUS, FONT_CAPTION, FONT_CLOSE, FONT_ROW,
    FONT_TITLE, ICON_SIZE, RADIUS_PILL, ROW_HEIGHT, ROW_RADIUS, SETTINGS_CLOSE_INSET,
    SETTINGS_CLOSE_SIZE, SETTINGS_CONTENT_MIN_WIDTH, SETTINGS_CONTENT_PADDING, SETTINGS_MAX_HEIGHT,
    SETTINGS_MAX_WIDTH, SETTINGS_MIN_HEIGHT, SETTINGS_MIN_WIDTH, SETTINGS_NAV_GAP,
    SETTINGS_NAV_ICON_INSET, SETTINGS_NAV_LABEL_INSET, SETTINGS_PADDING, SETTINGS_RAIL_PADDING_Y,
    SETTINGS_RAIL_WIDTH, SETTINGS_SEARCH_HEIGHT, SETTINGS_SEARCH_ICON_INSET,
    SETTINGS_SEARCH_TEXT_INSET, SETTINGS_SEARCH_WIDTH, SIDEBAR_CONTENT_PADDING, SPACE_4, SPACE_8,
    SPACE_14, SPACE_26, SPACE_32, SPACE_46, SPACE_120, STROKE_HAIRLINE,
};

/// A settings navigation row with a leading glyph, as in the reference
/// pictures (preferences window layout).
fn nav_item(
    ui: &mut egui::Ui,
    label: &str,
    selected: bool,
    palette: &Palette,
    draw: fn(&egui::Painter, Rect, Color32),
) -> egui::Response {
    let width = ui.available_width().max(SPACE_120);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, ROW_HEIGHT), Sense::click());
    let fill = if selected {
        palette.selected
    } else if response.hovered() {
        palette.hover
    } else {
        Color32::TRANSPARENT
    };
    if fill != Color32::TRANSPARENT {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(ROW_RADIUS), fill);
    }
    let icon_rect = Rect::from_min_size(
        rect.min + Vec2::new(SETTINGS_NAV_ICON_INSET, SPACE_8),
        Vec2::splat(ICON_SIZE),
    );
    draw(ui.painter(), icon_rect, palette.muted);
    ui.painter().text(
        rect.min + Vec2::new(SETTINGS_NAV_LABEL_INSET, ROW_HEIGHT / 2.0),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(FONT_ROW),
        if selected {
            palette.text
        } else {
            palette.muted
        },
    );
    response
}

/// Pick a small glyph for a settings section (original shape work).
fn settings_icon(label: &str) -> fn(&egui::Painter, Rect, Color32) {
    match label {
        "Local workspace" | "People" => draw_person,
        "Preferences" | "General" => draw_settings,
        "Notifications" | "Requests" => draw_inbox,
        "Analytics" => draw_grid,
        "Import" | "Offline" => draw_note,
        "AI assistant" => draw_help,
        "Connections" | "Automation" | "Server" => draw_link,
        _ => draw_page,
    }
}

fn settings_label(label: &str) -> String {
    match label {
        "Local workspace" => i18n::t("local_workspace"),
        "Preferences" => i18n::t("preferences"),
        "General" => i18n::t("general"),
        "People" => i18n::t("people"),
        "Analytics" => i18n::t("analytics"),
        "AI assistant" => i18n::t("ai_assistant"),
        "Offline" => i18n::t("offline"),
        _ => label.to_string(),
    }
}

/// One settings-nav entry that selects the page shown on the right.
fn settings_nav(
    ui: &mut egui::Ui,
    view: &ViewState,
    label: &str,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let selected = view.settings.page == label;
    if nav_item(
        ui,
        &settings_label(label),
        selected,
        palette,
        settings_icon(label),
    )
    .clicked()
    {
        commands.push(Command::SetSettingsPage(label.to_string()));
    }
}

// ---- Settings --------------------------------------------------------------

/// One settings-rail entry: the section label and the searchable description.
/// Keeping the list here means the rail and the search filter never disagree
/// (DRY, spec-015).
const SETTINGS_NAV: &[(&str, &[(&str, &str)])] = &[
    (
        "Account",
        &[
            ("Local workspace", "Workspace name, members and access"),
            (
                "Preferences",
                "Appearance, input options, language and time",
            ),
            ("Notifications", "Inbox and notification preferences"),
            ("Server", "Self-hosted server, login and session"),
        ],
    ),
    (
        "Workspace",
        &[
            ("General", "Workspace identity, members and data"),
            ("Requests", "Workspace access requests"),
            ("People", "Manage who can access this workspace"),
            ("Analytics", "Workspace analytics"),
            ("Import", "Import pages and data"),
        ],
    ),
    (
        "Features",
        &[
            ("AI assistant", "Local AI powered by Ollama"),
            ("Meeting notes", "Meeting notes and transcripts"),
            ("Connections", "Connected services"),
            ("Automation", "Workspace automation"),
            ("Offline", "Available offline pages"),
        ],
    ),
];

/// The first settings section whose title or description matches the query.
fn first_settings_match(query: &str) -> Option<&'static str> {
    SETTINGS_NAV
        .iter()
        .flat_map(|(_, items)| items.iter())
        .find(|(label, description)| section_matches(query, label, description))
        .map(|(label, _)| *label)
}

/// The searchable description of a section, used to keep the content pane in
/// sync with the filtered rail.
fn settings_description(label: &str) -> &'static str {
    SETTINGS_NAV
        .iter()
        .flat_map(|(_, items)| items.iter())
        .find(|(item, _)| *item == label)
        .map(|(_, description)| *description)
        .unwrap_or("")
}

fn page_label(page: &str) -> String {
    match page {
        "Preferences" => i18n::t("preferences"),
        "General" => i18n::t("general"),
        "People" => i18n::t("people"),
        "Offline" => i18n::t("offline"),
        "AI assistant" => i18n::t("ai_assistant"),
        _ => page.to_string(),
    }
}

pub(crate) fn render_settings(
    ctx: &egui::Context,
    ws: &mut Workspace,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let screen = ctx.viewport_rect();
    let size = Vec2::new(
        (screen.width() - 2.0 * DIALOG_INSET).clamp(SETTINGS_MIN_WIDTH, SETTINGS_MAX_WIDTH),
        (screen.height() - 2.0 * DIALOG_INSET).clamp(SETTINGS_MIN_HEIGHT, SETTINGS_MAX_HEIGHT),
    );
    let resp = egui::Window::new("settings")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_size(size)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .frame(
            egui::Frame::NONE
                .fill(palette.content)
                .corner_radius(CornerRadius::same(DIALOG_RADIUS))
                .stroke(Stroke::new(STROKE_HAIRLINE, palette.border)),
        )
        .show(ctx, |ui| {
            let whole = ui.max_rect();
            // Two-column layout: a dimmed navigation rail and the content pane.
            ui.painter().rect_filled(
                Rect::from_min_size(whole.min, Vec2::new(SETTINGS_RAIL_WIDTH, whole.height())),
                CornerRadius {
                    nw: DIALOG_RADIUS,
                    sw: DIALOG_RADIUS,
                    ne: 0,
                    se: 0,
                },
                palette.sidebar,
            );
            ui.painter().vline(
                whole.min.x + SETTINGS_RAIL_WIDTH,
                whole.y_range(),
                Stroke::new(STROKE_HAIRLINE, palette.border),
            );

            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                // Left navigation column. Mail & Calendar is intentionally not
                // implemented, so it is not offered in the rail.
                ui.allocate_ui_with_layout(
                    Vec2::new(SETTINGS_RAIL_WIDTH, whole.height()),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        ui.set_width(SETTINGS_RAIL_WIDTH);
                        ui.set_min_height(whole.height());
                        ui.spacing_mut().scroll.fade.strength = 0.0;
                        ui.add_space(SETTINGS_RAIL_PADDING_Y);
                        // Search is fixed; only the navigation list scrolls.
                        ui.horizontal(|ui| {
                            ui.add_space(SIDEBAR_CONTENT_PADDING);
                            let (search, _) = ui.allocate_exact_size(
                                Vec2::new(SETTINGS_SEARCH_WIDTH, SETTINGS_SEARCH_HEIGHT),
                                Sense::hover(),
                            );
                            ui.painter().rect_filled(
                                search,
                                CornerRadius::same(ROW_RADIUS),
                                palette.hover,
                            );
                            draw_search(
                                ui.painter(),
                                Rect::from_min_size(
                                    search.min + Vec2::new(SETTINGS_SEARCH_ICON_INSET, SPACE_8),
                                    Vec2::splat(ICON_SIZE),
                                ),
                                palette.muted,
                            );
                            let mut query = view.settings.query.clone();
                            if query.is_empty() {
                                ui.painter().text(
                                    Pos2::new(search.min.x + SETTINGS_SEARCH_TEXT_INSET, search.center().y),
                                    Align2::LEFT_CENTER,
                                    "Search settings",
                                    FontId::proportional(FONT_ROW),
                                    palette.muted,
                                );
                            }
                            let edit = ui.put(
                                Rect::from_min_size(
                                    search.min + Vec2::new(SETTINGS_SEARCH_TEXT_INSET, 0.0),
                                    Vec2::new(SETTINGS_SEARCH_WIDTH - SETTINGS_SEARCH_TEXT_INSET - SETTINGS_SEARCH_ICON_INSET, SETTINGS_SEARCH_HEIGHT),
                                ),
                                egui::TextEdit::singleline(&mut query)
                                    .frame(egui::Frame::NONE)
                                    .margin(egui::Margin::symmetric(SPACE_4 as i8, SPACE_4 as i8))
                                    .align(egui::Align2::LEFT_CENTER)
                                    .text_color(palette.text),
                            );
                            if edit.changed() {
                                if !query.trim().is_empty()
                                    && let Some(first) = first_settings_match(&query)
                                {
                                    commands.push(Command::SetSettingsPage(first.to_string()));
                                }
                                commands.push(Command::SetSettingsQuery(query));
                            }
                        });
                        ui.add_space(SETTINGS_NAV_GAP);
                        let rail_height = ui.available_height().max(SPACE_120);
                        egui::ScrollArea::vertical()
                            .id_salt("settings_rail_scroll")
                            .max_height(rail_height)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.set_min_height(rail_height);
                                egui::Frame::NONE
                                    .inner_margin(egui::Margin::symmetric(SIDEBAR_CONTENT_PADDING as i8, 0))
                                    .show(ui, |ui| {
                                        ui.set_width(SETTINGS_SEARCH_WIDTH);
                                        for (section, items) in SETTINGS_NAV {
                                            let visible: Vec<_> = items
                                                .iter()
                                                .filter(|(label, description)| {
                                                    section_matches(
                                                        &view.settings.query,
                                                        label,
                                                        description,
                                                    )
                                                })
                                                .collect();
                                            if visible.is_empty() {
                                                continue;
                                            }
                                            section_label(ui, section, palette);
                                            for (label, _) in visible {
                                                settings_nav(
                                                    ui, view, label, palette, commands,
                                                );
                                            }
                                            ui.add_space(SETTINGS_NAV_GAP);
                                        }
                                        ui.add_space(SETTINGS_RAIL_PADDING_Y);
                                    });
                            });
                    },
                );

                // Right content column spans from the rail to the dialog's
                // right border; the scrollbar sits on that border and the
                // frame supplies the internal content padding (spec-069).
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width().max(SETTINGS_CONTENT_MIN_WIDTH));
                    let content_height = ui.available_height().max(SPACE_120);
                    egui::ScrollArea::vertical()
                        .max_height(content_height)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.add_space(SETTINGS_CONTENT_PADDING);
                            egui::Frame::NONE
                                .inner_margin(egui::Margin::symmetric(SETTINGS_CONTENT_PADDING as i8, 0))
                                .show(ui, |ui| {
                            let page = view.settings.page.as_str();
                            let subtitle = if page == "Preferences" {
                                "Choose how you want the app to look and behave"
                            } else if page == "AI assistant" {
                                "Local AI powered by Ollama"
                            } else {
                                "Manage this section of your workspace"
                            };
                            ui.label(
                                egui::RichText::new(page_label(page))
                                    .font(strong_font(FONT_TITLE))
                                    .color(palette.text),
                            );
                            ui.add_space(SPACE_4);
                            ui.label(
                                egui::RichText::new(subtitle)
                                    .size(FONT_ROW)
                                    .color(palette.muted),
                            );
                            ui.add_space(SPACE_26);
                            let query = view.settings.query.trim();
                            let catalog = rows_for(page);
                            let rows = visible_rows(page, query);
                            let matches = if catalog.is_empty() {
                                settings_matches(query, page, settings_description(page))
                            } else {
                                !rows.is_empty()
                            };
                            if !query.is_empty() && !matches {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "No settings match “{query}”. Try a different word."
                                    ))
                                    .size(FONT_ROW)
                                    .color(palette.muted),
                                );
                            } else {
                                match page {
                                    "AI assistant" => render_ai_settings(
                                        ui,
                                        &view.ai,
                                        palette,
                                        commands,
                                        &rows,
                                    ),
                                    "Preferences" => render_preferences(
                                        ui,
                                        &view.settings,
                                        &view.server,
                                        palette,
                                        commands,
                                        &rows,
                                    ),
                                    "General" => render_workspace_settings(
                                        ui,
                                        ws,
                                        &view.settings,
                                        palette,
                                        commands,
                                        &rows,
                                    ),
                                    "Offline" => {
                                        render_offline_settings(ui, ws, palette, &rows)
                                    }
                                    "People" => render_people_settings(
                                        ui,
                                        ws,
                                        &view.sharing,
                                        &view.settings,
                                        palette,
                                        commands,
                                        &rows,
                                    ),
                                    "Server" => {
                                        render_server_settings(ui, &view.server, palette, commands)
                                    }
                                    _ => {
                                        ui.label(
                                            egui::RichText::new(
                                                "Not available in this version. This section is reserved for a future update.",
                                            )
                                            .size(FONT_ROW)
                                            .color(palette.muted),
                                        );
                                    }
                                }
                            }
                            ui.add_space(SETTINGS_CONTENT_PADDING);
                                });
                        });
                });
            });

            // The close "×" is a fixed control in the top-right corner of the
            // preferences window (reference/picture-15.png .. picture-29.png),
            // NOT a button on the heading row. It stays put while content
            // scrolls, and sits in a subtle rounded hover well.
            // The close "x" lives in a foreground layer so scrolling content
            // can never paint over it (spec-069).
            let close_pos = Pos2::new(
                whole.right() - SETTINGS_PADDING - SETTINGS_CLOSE_INSET,
                whole.top() + SETTINGS_PADDING,
            );
            egui::Area::new(egui::Id::unique("settings_close"))
                .order(egui::Order::Foreground)
                .fixed_pos(close_pos)
                .show(ctx, |ui| {
                    let (rect, response) =
                        ui.allocate_exact_size(Vec2::splat(SETTINGS_CLOSE_SIZE), Sense::click());
                    if response.hovered() {
                        ui.painter().rect_filled(
                            rect,
                            CornerRadius::same(RADIUS_PILL),
                            palette.hover,
                        );
                    }
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "×",
                        egui::FontId::proportional(FONT_CLOSE),
                        palette.muted,
                    );
                    if response.clicked() {
                        commands.push(Command::CloseSettings);
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
        commands.push(Command::CloseSettings);
    }
}

/// A read-only preference row (label, description and a value chip on the
/// right), matching the two-column rows in the reference preferences window.
pub(super) fn display_row(
    ui: &mut egui::Ui,
    title: &str,
    description: &str,
    value: &str,
    palette: &Palette,
) {
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), SPACE_46), Sense::hover());
    ui.painter().text(
        rect.min + Vec2::new(0.0, SPACE_14),
        Align2::LEFT_CENTER,
        title,
        FontId::proportional(FONT_CAPTION),
        palette.text,
    );
    ui.painter().text(
        rect.min + Vec2::new(0.0, SPACE_32),
        Align2::LEFT_CENTER,
        description,
        FontId::proportional(FONT_CAPTION),
        palette.muted,
    );
    let chip = Rect::from_min_size(
        Pos2::new(rect.max.x - CHIP_WIDTH, rect.min.y + SPACE_8),
        Vec2::new(CHIP_WIDTH, CHIP_HEIGHT),
    );
    ui.painter().rect_stroke(
        chip,
        CornerRadius::same(ROW_RADIUS),
        Stroke::new(STROKE_HAIRLINE, palette.border),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        chip.center(),
        Align2::CENTER_CENTER,
        value,
        FontId::proportional(FONT_CAPTION),
        palette.text,
    );
}
