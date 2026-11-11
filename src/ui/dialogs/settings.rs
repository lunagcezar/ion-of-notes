//! Settings window: navigation rail, search filtering and page routing.

use crate::i18n;
use crate::model::Workspace;
use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, Vec2};

use super::super::commands::Command;
use super::super::icons::{
    draw_grid, draw_help, draw_inbox, draw_link, draw_note, draw_page, draw_panel, draw_person,
    draw_search, draw_settings,
};
use super::super::palette::{Palette, strong_font};
use super::super::settings_search::{rows_for, section_matches, settings_matches, visible_rows};
use super::super::view::ViewState;
use super::super::widgets::section_label;
use super::settings_pages::{
    render_ai_settings, render_offline_settings, render_people_settings, render_preferences,
    render_server_settings, render_sidebar_settings, render_workspace_settings,
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
    let width = ui.available_width().max(120.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 30.0), Sense::click());
    let fill = if selected {
        palette.selected
    } else if response.hovered() {
        palette.hover
    } else {
        Color32::TRANSPARENT
    };
    if fill != Color32::TRANSPARENT {
        ui.painter().rect_filled(rect, CornerRadius::same(6), fill);
    }
    let icon_rect = Rect::from_min_size(rect.min + Vec2::new(12.0, 7.0), Vec2::splat(16.0));
    draw(ui.painter(), icon_rect, palette.muted);
    ui.painter().text(
        rect.min + Vec2::new(38.0, 15.0),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(13.0),
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
        "Sidebar" => draw_panel,
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
        "Sidebar" => i18n::t("sidebar"),
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
    let selected = view.settings_page == label;
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
            ("Sidebar", "Choose what appears in the sidebar"),
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
        "Sidebar" => i18n::t("sidebar"),
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
    let resp = egui::Window::new("settings")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_size(Vec2::new(1200.0, 720.0))
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .frame(
            egui::Frame::NONE
                .fill(palette.content)
                .corner_radius(CornerRadius::same(10))
                .stroke(Stroke::new(1.0, palette.border)),
        )
        .show(ctx, |ui| {
            let whole = ui.max_rect();
            // Two-column layout: a dimmed navigation rail and the content pane.
            ui.painter().rect_filled(
                Rect::from_min_size(whole.min, Vec2::new(300.0, whole.height())),
                CornerRadius::ZERO,
                palette.sidebar,
            );
            ui.painter().vline(
                whole.min.x + 300.0,
                whole.y_range(),
                Stroke::new(1.0, palette.border),
            );

            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.add_space(16.0);
                // Left navigation column. Mail & Calendar is intentionally not
                // implemented, so it is not offered in the rail.
                ui.vertical(|ui| {
                    ui.set_width(268.0);
                    ui.add_space(18.0);
                    let (search, _) =
                        ui.allocate_exact_size(Vec2::new(268.0, 30.0), Sense::hover());
                    ui.painter().rect_filled(search, CornerRadius::same(6), palette.hover);
                    draw_search(
                        ui.painter(),
                        Rect::from_min_size(search.min + Vec2::new(8.0, 7.0), Vec2::splat(16.0)),
                        palette.muted,
                    );
                    let mut query = view.settings_query.clone();
                    let edit = ui.put(
                        Rect::from_min_size(search.min + Vec2::new(28.0, 4.0), Vec2::new(232.0, 22.0)),
                        egui::TextEdit::singleline(&mut query)
                            .frame(egui::Frame::NONE)
                            .hint_text("Search settings")
                            .text_color(palette.text),
                    );
                    if edit.changed() {
                        // Typing a query narrows the rail and moves to the
                        // first matching section (spec-015). An empty query
                        // leaves the current section alone.
                        if !query.trim().is_empty()
                            && let Some(first) = first_settings_match(&query)
                        {
                            commands.push(Command::SetSettingsPage(first.to_string()));
                        }
                        commands.push(Command::SetSettingsQuery(query));
                    }
                    ui.add_space(14.0);
                    for (section, items) in SETTINGS_NAV {
                        let visible: Vec<_> = items
                            .iter()
                            .filter(|(label, description)| {
                                section_matches(&view.settings_query, label, description)
                            })
                            .collect();
                        if visible.is_empty() {
                            continue;
                        }
                        section_label(ui, section, palette);
                        for (label, _) in visible {
                            settings_nav(ui, view, label, palette, commands);
                        }
                        ui.add_space(14.0);
                    }
                });

                ui.add_space(40.0);

                // Right content column. The right margin keeps the content
                // clear of the fixed close control in the header corner.
                ui.vertical(|ui| {
                    ui.set_width((ui.available_width() - 96.0).max(320.0));
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.add_space(24.0);
                            let page = view.settings_page.as_str();
                            let subtitle = if page == "Preferences" {
                                "Choose how you want the app to look and behave"
                            } else if page == "AI assistant" {
                                "Local AI powered by Ollama"
                            } else {
                                "Manage this section of your workspace"
                            };
                            ui.label(
                                egui::RichText::new(page_label(page))
                                    .font(strong_font(28.0))
                                    .color(palette.text),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(subtitle)
                                    .size(13.0)
                                    .color(palette.muted),
                            );
                            ui.add_space(26.0);
                            let query = view.settings_query.trim();
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
                                    .size(13.0)
                                    .color(palette.muted),
                                );
                            } else {
                                match page {
                                    "AI assistant" => {
                                        render_ai_settings(ui, view, palette, commands, &rows)
                                    }
                                    "Preferences" => {
                                        render_preferences(ui, view, palette, commands, &rows)
                                    }
                                    "General" => render_workspace_settings(
                                        ui, ws, view, palette, commands, &rows,
                                    ),
                                    "Sidebar" => {
                                        render_sidebar_settings(ui, ws, view, palette, commands, &rows)
                                    }
                                    "Offline" => {
                                        render_offline_settings(ui, ws, palette, &rows)
                                    }
                                    "People" => {
                                        render_people_settings(ui, ws, view, palette, commands, &rows)
                                    }
                                    "Server" => {
                                        render_server_settings(ui, view, palette, commands)
                                    }
                                    _ => {
                                        ui.label(
                                            egui::RichText::new(
                                                "Not available in this version. This section is reserved for a future update.",
                                            )
                                            .size(13.0)
                                            .color(palette.muted),
                                        );
                                    }
                                }
                            }
                            ui.add_space(24.0);
                        });
                });
            });

            // The close "×" is a fixed control in the top-right corner of the
            // preferences window (reference/picture-15.png .. picture-29.png),
            // NOT a button on the heading row. It stays put while content
            // scrolls, and sits in a subtle rounded hover well.
            let close_rect = Rect::from_center_size(
                Pos2::new(whole.right() - 58.0, whole.top() + 34.0),
                Vec2::splat(28.0),
            );
            ui.painter().rect_filled(
                close_rect,
                CornerRadius::same(14),
                palette.hover,
            );
            let close = ui.put(
                close_rect,
                egui::Button::new(
                    egui::RichText::new("×").size(18.0).color(palette.muted),
                )
                .frame(false),
            );
            if close.clicked() {
                commands.push(Command::CloseSettings);
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
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 46.0), Sense::hover());
    ui.painter().text(
        rect.min + Vec2::new(0.0, 14.0),
        Align2::LEFT_CENTER,
        title,
        FontId::proportional(12.5),
        palette.text,
    );
    ui.painter().text(
        rect.min + Vec2::new(0.0, 32.0),
        Align2::LEFT_CENTER,
        description,
        FontId::proportional(11.0),
        palette.muted,
    );
    let chip = Rect::from_min_size(
        Pos2::new(rect.max.x - 150.0, rect.min.y + 9.0),
        Vec2::new(150.0, 26.0),
    );
    ui.painter().rect_stroke(
        chip,
        CornerRadius::same(6),
        Stroke::new(1.0, palette.border),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        chip.center(),
        Align2::CENTER_CENTER,
        value,
        FontId::proportional(12.0),
        palette.text,
    );
}
