//! Floating windows: the "Open in new tab…" switcher, quick search / AI chat,
//! settings (preferences + AI assistant), share, inbox, shortcut sheet and the
//! slash menu (SRP). Drawing collects `Command`s; it never mutates state.

use crate::model::Workspace;
use crate::shortcuts::SHORTCUT_TABLE;
use egui::{Align2, Color32, CornerRadius, FontId, Margin, Pos2, Rect, Sense, Stroke, Vec2};

use super::app::{new_tab_choices, Command, NewTabChoice, ViewState};
use super::icons::{
    draw_book, draw_grid, draw_help, draw_inbox, draw_link, draw_note, draw_page, draw_panel,
    draw_person, draw_search, draw_settings, draw_sliders,
};
use super::palette::{block_menu_options, strong_font, Palette};

/// A compact window frame shared by the popovers.
fn popover_frame(palette: &Palette) -> egui::Frame {
    egui::Frame::NONE
        .fill(palette.content)
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(14))
        .stroke(Stroke::new(1.0, palette.border))
}

/// A settings navigation row with a leading glyph, as in the reference
/// pictures (preferences window layout).
fn nav_item(
    ui: &mut egui::Ui,
    label: &str,
    selected: bool,
    palette: &Palette,
    draw: fn(&egui::Painter, Rect, Color32),
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 30.0), Sense::click());
    let fill = if selected {
        palette.selected
    } else if response.hovered() {
        palette.hover
    } else {
        Color32::TRANSPARENT
    };
    if fill != Color32::TRANSPARENT {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), fill);
    }
    let icon_rect =
        Rect::from_min_size(rect.min + Vec2::new(12.0, 7.0), Vec2::splat(16.0));
    draw(ui.painter(), icon_rect, palette.muted);
    ui.painter().text(
        rect.min + Vec2::new(38.0, 15.0),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(13.0),
        if selected { palette.text } else { palette.muted },
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
        "Connections" | "Automation" => draw_link,
        _ => draw_page,
    }
}

fn section_label(ui: &mut egui::Ui, text: &str, palette: &Palette) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 24.0), Sense::hover());
    ui.painter().text(
        rect.min + Vec2::new(12.0, 14.0),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(11.0),
        palette.muted,
    );
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
    if nav_item(ui, label, selected, palette, settings_icon(label)).clicked() {
        commands.push(Command::SetSettingsPage(label.to_string()));
    }
}

// ---- "Open in new tab…" switcher (the tab-strip "+") ----------------------

/// A simple speech-bubble glyph for the "New chat" row.
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

/// The footer hint strip shared by the search popovers (attachments).
fn switcher_footer(ui: &mut egui::Ui, palette: &Palette) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 18.0;
        for (keys, label) in [
            ("Ctrl+↵", "Open in new tab"),
            ("Ctrl+L", "Copy link"),
            ("Shift+Ctrl+K", "Command Search"),
        ] {
            ui.label(
                egui::RichText::new(keys)
                    .size(11.0)
                    .color(palette.muted),
            );
            ui.label(
                egui::RichText::new(label)
                    .size(11.0)
                    .color(palette.muted),
            );
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
            draw_sliders(ui.painter(), rect, palette.muted);
        });
    });
}

pub(super) fn render_new_tab(
    ctx: &egui::Context,
    ws: &Workspace,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let screen = ctx.viewport_rect();
    let size = Vec2::new(1000.0, 690.0);
    let pos = Pos2::new(
        screen.left() + ((screen.width() - size.x) / 2.0).max(0.0),
        screen.top() + 110.0,
    );
    egui::Window::new("new_tab_switcher")
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
                .stroke(Stroke::new(1.0, palette.border)),
        )
        .show(ctx, |ui| {
            // Search header.
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                let (icon, _) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
                draw_search(ui.painter(), icon, palette.muted);
                ui.add_space(6.0);
                let mut query = view.new_tab_query.clone();
                let field = ui.add(
                    egui::TextEdit::singleline(&mut query)
                        .id(egui::Id::unique("new_tab_query"))
                        .frame(egui::Frame::NONE)
                        .font(FontId::proportional(16.0))
                        .desired_width(700.0)
                        .hint_text("Open in new tab...")
                        .text_color(palette.text),
                );
                field.request_focus();
                if field.changed() {
                    commands.push(Command::SetNewTabQuery(query));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (help, _) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
                    draw_help(ui.painter(), help, palette.accent);
                    let (panel, _) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
                    let blue = palette.accent;
                    ui.painter().rect_stroke(
                        panel.shrink(2.0),
                        CornerRadius::same(2),
                        Stroke::new(1.3, blue),
                        egui::StrokeKind::Inside,
                    );
                    ui.painter().vline(
                        panel.center().x + 2.0,
                        panel.y_range().shrink(2.0),
                        Stroke::new(1.3, blue),
                    );
                });
            });
            ui.add_space(12.0);

            // Filter row.
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 18.0;
                ui.add_space(24.0);
                for text in ["Aa  Title only", "Created by ⌄", "In ⌄", "+ Filter"] {
                    ui.label(
                        egui::RichText::new(text)
                            .size(12.0)
                            .color(palette.muted),
                    );
                }
            });
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            let body_height = (ui.available_height() - 44.0).max(200.0);
            let list_width = 545.0;
            let preview_width = ui.available_width() - list_width - 18.0;
            let choices = new_tab_choices(ws, &view.new_tab_query);

            ui.horizontal_top(|ui| {
                // Result list.
                ui.vertical(|ui| {
                    ui.set_width(list_width);
                    ui.set_height(body_height);
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for (index, choice) in choices.iter().enumerate() {
                                if index == 2 {
                                    section_label(ui, "Today", palette);
                                }
                                if index == 3 {
                                    section_label(ui, "Past week", palette);
                                }
                                let (rect, response) = ui.allocate_exact_size(
                                    Vec2::new(ui.available_width(), 34.0),
                                    Sense::click(),
                                );
                                let selected = view.new_tab_selected == index;
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
                                    rect.min + Vec2::new(10.0, 9.0),
                                    Vec2::splat(16.0),
                                );
                                let label = match choice {
                                    NewTabChoice::NewChat => {
                                        draw_chat(ui.painter(), icon_rect, palette.muted);
                                        "New chat".to_string()
                                    }
                                    NewTabChoice::NewPage => {
                                        draw_page(ui.painter(), icon_rect, palette.muted);
                                        "New page".to_string()
                                    }
                                    NewTabChoice::Page(id) => {
                                        draw_page(ui.painter(), icon_rect, palette.muted);
                                        ws.page(id)
                                            .map(|p| p.title.clone())
                                            .unwrap_or_else(|| "Untitled".to_string())
                                    }
                                };
                                ui.painter().text(
                                    rect.min + Vec2::new(36.0, 17.0),
                                    Align2::LEFT_CENTER,
                                    &label,
                                    FontId::proportional(13.5),
                                    palette.text,
                                );
                                if let NewTabChoice::Page(id) = choice {
                                    let crumb = ws
                                        .page(id)
                                        .and_then(|p| p.parent.clone())
                                        .and_then(|pid| ws.page(&pid).map(|p| p.title.clone()))
                                        .unwrap_or_else(|| ws.name.clone());
                                    let suffix = format!("— {crumb}");
                                    let x = rect.min.x
                                        + 40.0
                                        + label.chars().count() as f32 * 7.2;
                                    ui.painter().text(
                                        Pos2::new(x, rect.min.y + 17.0),
                                        Align2::LEFT_CENTER,
                                        suffix,
                                        FontId::proportional(12.0),
                                        palette.muted,
                                    );
                                }
                                if selected {
                                    ui.painter().text(
                                        rect.max + Vec2::new(-12.0, -17.0),
                                        Align2::RIGHT_CENTER,
                                        "↵",
                                        FontId::proportional(13.0),
                                        palette.muted,
                                    );
                                }
                                if response.hovered() && !selected {
                                    commands.push(Command::SetNewTabSelected(index));
                                }
                                if response.clicked() {
                                    commands
                                        .push(Command::ActivateNewTab(choice.clone()));
                                }
                            }
                        });
                });

                ui.add_space(18.0);

                // Preview pane.
                ui.vertical(|ui| {
                    ui.set_width(preview_width);
                    ui.set_height(body_height);
                    let (card, _) = ui.allocate_exact_size(
                        Vec2::new(preview_width, body_height),
                        Sense::hover(),
                    );
                    ui.painter().rect_filled(
                        card,
                        CornerRadius::same(8),
                        palette.callout,
                    );
                    ui.painter().rect_stroke(
                        card,
                        CornerRadius::same(8),
                        Stroke::new(1.0, palette.border),
                        egui::StrokeKind::Inside,
                    );
                    // Two small controls in the top-right of the preview.
                    let link = Rect::from_min_size(
                        card.max + Vec2::new(-56.0, 12.0),
                        Vec2::splat(16.0),
                    );
                    ui.painter().rect_stroke(
                        link.shrink(2.0),
                        CornerRadius::same(3),
                        Stroke::new(1.2, palette.muted),
                        egui::StrokeKind::Inside,
                    );
                    let expand = Rect::from_min_size(
                        card.max + Vec2::new(-34.0, 12.0),
                        Vec2::splat(16.0),
                    );
                    ui.painter().line(
                        vec![
                            expand.min + Vec2::new(3.0, 8.0),
                            expand.min + Vec2::new(13.0, 3.0),
                        ],
                        Stroke::new(1.2, palette.muted),
                    );

                    let selected = choices.get(view.new_tab_selected);
                    match selected {
                        Some(NewTabChoice::Page(id)) => {
                            if let Some(page) = ws.page(id) {
                                draw_book(
                                    ui.painter(),
                                    Rect::from_min_size(
                                        card.min + Vec2::new(24.0, 28.0),
                                        Vec2::splat(48.0),
                                    ),
                                );
                                ui.painter().text(
                                    card.min + Vec2::new(24.0, 90.0),
                                    Align2::LEFT_CENTER,
                                    &ws.name,
                                    FontId::proportional(11.0),
                                    palette.muted,
                                );
                                ui.painter().text(
                                    card.min + Vec2::new(24.0, 116.0),
                                    Align2::LEFT_CENTER,
                                    &page.title,
                                    strong_font(22.0),
                                    palette.text,
                                );
                                let snippet: String =
                                    page.plain_text().chars().take(220).collect();
                                let mut y = card.min.y + 164.0;
                                for line in wrap_text(&snippet, 44) {
                                    ui.painter().text(
                                        Pos2::new(card.min.x + 24.0, y),
                                        Align2::LEFT_CENTER,
                                        line,
                                        FontId::proportional(12.5),
                                        palette.muted,
                                    );
                                    y += 20.0;
                                    if y > card.max.y - 20.0 {
                                        break;
                                    }
                                }
                            }
                        }
                        _ => {
                            // A soft skeleton, as in attachments/picture-1.png.
                            ui.painter().circle_filled(
                                card.min + Vec2::new(60.0, 90.0),
                                22.0,
                                palette.hover,
                            );
                            for (i, width) in [220.0, 180.0, 120.0].iter().enumerate() {
                                let y = card.min.y + 130.0 + i as f32 * 30.0;
                                ui.painter().rect_filled(
                                    Rect::from_min_size(
                                        Pos2::new(card.min.x + 24.0, y),
                                        Vec2::new(*width, 9.0),
                                    ),
                                    CornerRadius::same(4),
                                    palette.hover,
                                );
                            }
                        }
                    }
                });
            });
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);
            switcher_footer(ui, palette);
        });
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

// ---- Quick search / AI chat ------------------------------------------------

pub(super) fn render_search(
    ctx: &egui::Context,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    egui::Window::new("quick_search")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_size(Vec2::new(640.0, 480.0))
        .anchor(Align2::CENTER_CENTER, Vec2::new(0.0, -60.0))
        .frame(popover_frame(palette))
        .show(ctx, |ui| {
            let mut query = view.ai_query.clone();
            let field = ui.add(
                egui::TextEdit::singleline(&mut query)
                    .id(egui::Id::unique("quick_search_query"))
                    .frame(egui::Frame::NONE)
                    .desired_width(f32::INFINITY)
                    .hint_text("Search or ask a question...")
                    .text_color(palette.text),
            );
            field.request_focus();
            if field.changed() {
                commands.push(Command::SetAiQuery(query.clone()));
                commands.push(Command::SetSearchQuery(query));
            }
            ui.add_space(8.0);
            ui.separator();
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(190.0)
                .show(ui, |ui| {
                    for result in &view.search_results {
                        let (rect, response) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), 46.0),
                            Sense::click(),
                        );
                        if response.hovered() {
                            ui.painter()
                                .rect_filled(rect, CornerRadius::same(6), palette.hover);
                        }
                        ui.painter().rect_filled(
                            Rect::from_min_size(
                                rect.min + Vec2::new(8.0, 10.0),
                                Vec2::splat(26.0),
                            ),
                            CornerRadius::same(5),
                            palette.icon_bg,
                        );
                        ui.painter().text(
                            rect.min + Vec2::new(44.0, 16.0),
                            Align2::LEFT_CENTER,
                            &result.page_title,
                            FontId::proportional(14.0),
                            palette.text,
                        );
                        let snippet: String = result.snippet.chars().take(70).collect();
                        ui.painter().text(
                            rect.min + Vec2::new(44.0, 33.0),
                            Align2::LEFT_CENTER,
                            snippet,
                            FontId::proportional(11.0),
                            palette.muted,
                        );
                        if response.clicked() {
                            commands.push(Command::SelectPage(result.page_id.clone()));
                            commands.push(Command::CloseSearch);
                        }
                    }
                });
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Ask AI")
                        .size(13.0)
                        .strong()
                        .color(palette.text),
                );
                ui.label(
                    egui::RichText::new("Ollama · CLI in PATH or service on the network")
                        .size(11.0)
                        .color(palette.muted),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Ask").size(12.0).color(Color32::WHITE),
                            )
                            .fill(palette.accent)
                            .corner_radius(CornerRadius::same(6)),
                        )
                        .clicked()
                    {
                        commands.push(Command::AskAi(view.ai_query.clone()));
                    }
                });
            });
            ui.add_space(6.0);
            if view.ai_busy {
                ui.label(
                    egui::RichText::new("Thinking…")
                        .size(12.0)
                        .color(palette.accent),
                );
            } else if let Some(answer) = &view.ai_answer {
                egui::Frame::NONE
                    .fill(palette.callout)
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(Margin::same(10))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(answer)
                                .size(12.5)
                                .color(palette.text),
                        );
                    });
            } else if let Some(error) = &view.ai_error {
                ui.label(
                    egui::RichText::new(error)
                        .size(11.5)
                        .color(Color32::from_rgb(224, 108, 108)),
                );
            } else {
                ui.label(
                    egui::RichText::new(&view.ai_status)
                        .size(11.0)
                        .color(palette.muted),
                );
            }
            ui.add_space(8.0);
            switcher_footer(ui, palette);
        });
}

// ---- Settings --------------------------------------------------------------

pub(super) fn render_settings(
    ctx: &egui::Context,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    egui::Window::new("settings")
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
                // Left navigation column. Mail & Calendar is intentionally not
                // implemented, so it is not offered in the rail.
                ui.vertical(|ui| {
                    ui.set_width(288.0);
                    ui.add_space(18.0);
                    let (search, _) =
                        ui.allocate_exact_size(Vec2::new(264.0, 30.0), Sense::hover());
                    ui.painter().rect_filled(
                        search,
                        CornerRadius::same(6),
                        palette.hover,
                    );
                    draw_search(
                        ui.painter(),
                        Rect::from_min_size(search.min + Vec2::new(8.0, 7.0), Vec2::splat(16.0)),
                        palette.muted,
                    );
                    ui.painter().text(
                        search.min + Vec2::new(32.0, 15.0),
                        Align2::LEFT_CENTER,
                        "Search settings",
                        FontId::proportional(12.0),
                        palette.muted,
                    );
                    ui.add_space(14.0);
                    section_label(ui, "Account", palette);
                    settings_nav(ui, view, "Local workspace", palette, commands);
                    settings_nav(ui, view, "Preferences", palette, commands);
                    settings_nav(ui, view, "Notifications", palette, commands);
                    ui.add_space(14.0);
                    section_label(ui, "Workspace", palette);
                    for label in [
                        "General",
                        "Requests",
                        "People",
                        "Sidebar",
                        "Analytics",
                        "Import",
                    ] {
                        settings_nav(ui, view, label, palette, commands);
                    }
                    ui.add_space(14.0);
                    section_label(ui, "Features", palette);
                    for label in [
                        "AI assistant",
                        "Meeting notes",
                        "Connections",
                        "Automation",
                        "Offline",
                    ] {
                        settings_nav(ui, view, label, palette, commands);
                    }
                });

                ui.add_space(12.0);

                // Right content column.
                ui.vertical(|ui| {
                    ui.set_width(856.0);
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
                                egui::RichText::new(page)
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
                            if page == "AI assistant" {
                                render_ai_settings(ui, view, palette, commands);
                            } else if page == "Preferences" {
                                render_preferences(ui, view, palette, commands);
                            } else {
                                ui.label(
                                    egui::RichText::new(
                                        "Settings for this section live on your self-hosted workspace.",
                                    )
                                    .size(13.0)
                                    .color(palette.muted),
                                );
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
                Pos2::new(whole.right() - 32.0, whole.top() + 34.0),
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
}

/// A read-only preference row (label, description and a value chip on the
/// right), matching the two-column rows in the reference preferences window.
fn display_row(
    ui: &mut egui::Ui,
    title: &str,
    description: &str,
    value: &str,
    palette: &Palette,
) {
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 46.0), Sense::hover());
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

fn render_preferences(
    ui: &mut egui::Ui,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    ui.label(
        egui::RichText::new("Appearance")
            .font(strong_font(17.0))
            .color(palette.text),
    );
    ui.add_space(12.0);
    ui.separator();
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Theme").color(palette.text));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let label = if view.dark { "Dark" } else { "Light" };
            if ui.add(egui::Button::new(label)).clicked() {
                commands.push(Command::ToggleTheme);
            }
        });
    });
    ui.add_space(6.0);
    ui.label(
        egui::RichText::new("Choose a theme for this device (Ctrl+Shift+L)")
            .size(12.0)
            .color(palette.muted),
    );
    ui.add_space(10.0);
    display_row(
        ui,
        "High contrast  Beta",
        "Increase contrast for improved visibility",
        "Use system setting ⌄",
        palette,
    );

    ui.add_space(26.0);
    ui.label(
        egui::RichText::new("Input options")
            .font(strong_font(17.0))
            .color(palette.text),
    );
    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    display_row(
        ui,
        "Use Enter to add a new line",
        "Applies to chat and comments. Press Ctrl+Enter to send.",
        "Off",
        palette,
    );

    ui.add_space(26.0);
    ui.label(
        egui::RichText::new("Language & time")
            .font(strong_font(17.0))
            .color(palette.text),
    );
    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    display_row(
        ui,
        "Language",
        "Choose the language you want to use the app in",
        "English (US) ⌄",
        palette,
    );
    display_row(
        ui,
        "Number format",
        "Choose how numbers and currencies are formatted",
        "1.000.000,00 ⌄",
        palette,
    );

    ui.add_space(26.0);
    ui.label(
        egui::RichText::new("Workspace & sync")
            .font(strong_font(17.0))
            .color(palette.text),
    );
    ui.add_space(12.0);
    ui.separator();
    ui.add_space(12.0);
    ui.label(
        egui::RichText::new("Self-hosted server  ·  connected")
            .size(12.0)
            .color(palette.accent),
    );
    ui.add_space(6.0);
    ui.label(
        egui::RichText::new(
            "Documents, members and permissions live on your server and sync to every signed-in device.",
        )
        .size(12.0)
        .color(palette.muted),
    );
    ui.add_space(14.0);
    for (label, value) in [
        ("Accounts", "1 owner · 2 guests"),
        ("Sharing", "Invite by email · full / edit / view"),
        ("Sync", "Versioned changes across devices"),
        ("Real-time", "Live co-editing over WebSocket"),
        ("Export", "Markdown tree · People CSV"),
    ] {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(label)
                    .size(12.0)
                    .strong()
                    .color(palette.text),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(value)
                        .size(11.0)
                        .color(palette.muted),
                );
            });
        });
        ui.add_space(6.0);
    }

    ui.add_space(18.0);
    ui.separator();
    ui.add_space(12.0);
    ui.label(
        egui::RichText::new(
            "Open source under the MIT license. Run your own server; your data stays yours.",
        )
        .size(12.0)
        .color(palette.muted),
    );
    ui.add_space(12.0);
    if ui
        .add(egui::Button::new("Restore demo workspace"))
        .clicked()
    {
        commands.push(Command::ResetWorkspace);
    }
}

fn render_ai_settings(
    ui: &mut egui::Ui,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    ui.label(
        egui::RichText::new("Connection")
            .font(strong_font(17.0))
            .color(palette.text),
    );
    ui.add_space(10.0);
    ui.separator();
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        let dot_color = if view.ai_connected {
            palette.accent
        } else {
            palette.muted
        };
        let (dot, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
        ui.painter().circle_filled(dot.center(), 4.0, dot_color);
        ui.label(
            egui::RichText::new(&view.ai_status)
                .size(12.0)
                .color(if view.ai_connected {
                    palette.text
                } else {
                    palette.muted
                }),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.add(egui::Button::new("Refresh")).clicked() {
                commands.push(Command::RefreshAiStatus);
            }
        });
    });
    ui.add_space(18.0);

    ui.label(
        egui::RichText::new("Service endpoint (host:port on your network)")
            .size(12.0)
            .strong()
            .color(palette.text),
    );
    ui.add_space(6.0);
    let mut endpoint = view.ai_endpoint.clone();
    if ui
        .add(
            egui::TextEdit::singleline(&mut endpoint)
                .desired_width(f32::INFINITY)
                .text_color(palette.text),
        )
        .changed()
    {
        commands.push(Command::SetAiEndpoint(endpoint));
    }
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(
            "Start Ollama (`ollama serve`) and point this at its port, or leave the default loopback. OLLAMA_HOST also works.",
        )
        .size(11.0)
        .color(palette.muted),
    );
    ui.add_space(16.0);

    ui.label(
        egui::RichText::new("Model")
            .size(12.0)
            .strong()
            .color(palette.text),
    );
    ui.add_space(6.0);
    let mut model = view.ai_model.clone();
    if ui
        .add(
            egui::TextEdit::singleline(&mut model)
                .desired_width(f32::INFINITY)
                .text_color(palette.text),
        )
        .changed()
    {
        commands.push(Command::SetAiModel(model));
    }
    ui.add_space(16.0);

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Prefer the ollama CLI when it is in PATH")
                .size(12.0)
                .color(palette.text),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let label = if view.ai_prefer_cli { "On" } else { "Off" };
            if ui.add(egui::Button::new(label)).clicked() {
                commands.push(Command::ToggleAiCli);
            }
        });
    });
    ui.add_space(22.0);

    ui.label(
        egui::RichText::new("Try it")
            .font(strong_font(17.0))
            .color(palette.text),
    );
    ui.add_space(10.0);
    ui.separator();
    ui.add_space(12.0);
    if ui
        .add(egui::Button::new("Ask a test question"))
        .clicked()
    {
        commands.push(Command::AskAi(
            "Say hello in one short sentence.".to_string(),
        ));
    }
    ui.add_space(10.0);
    if view.ai_busy {
        ui.label(
            egui::RichText::new("Waiting for Ollama…")
                .size(12.0)
                .color(palette.accent),
        );
    } else if let Some(answer) = &view.ai_answer {
        egui::Frame::NONE
            .fill(palette.callout)
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::same(10))
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(answer)
                        .size(12.5)
                        .color(palette.text),
                );
            });
    } else if let Some(error) = &view.ai_error {
        ui.label(
            egui::RichText::new(error)
                .size(11.5)
                .color(Color32::from_rgb(224, 108, 108)),
        );
    }
    ui.add_space(18.0);
    ui.label(
        egui::RichText::new(
            "Shortcut: Shift+Ctrl+J opens the assistant. Ask anything from Search or ask.",
        )
        .size(11.0)
        .color(palette.muted),
    );
}

// ---- Keyboard shortcut sheet ----------------------------------------------

pub(super) fn render_shortcuts(
    ctx: &egui::Context,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    egui::Window::new("shortcut_sheet")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_size(Vec2::new(560.0, 620.0))
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .frame(popover_frame(palette))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Keyboard shortcuts")
                        .font(strong_font(18.0))
                        .color(palette.text),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("×").size(16.0).color(palette.muted),
                            )
                            .frame(false),
                        )
                        .clicked()
                    {
                        commands.push(Command::CloseShortcuts);
                    }
                });
            });
            ui.add_space(6.0);
            ui.separator();
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for doc in SHORTCUT_TABLE {
                        let (rect, _) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), 38.0),
                            Sense::hover(),
                        );
                        ui.painter().text(
                            rect.min + Vec2::new(4.0, 11.0),
                            Align2::LEFT_CENTER,
                            doc.label,
                            FontId::proportional(12.5),
                            palette.text,
                        );
                        ui.painter().text(
                            rect.max + Vec2::new(-4.0, -11.0),
                            Align2::RIGHT_CENTER,
                            doc.keys,
                            FontId::proportional(12.0),
                            palette.accent,
                        );
                        ui.painter().text(
                            rect.min + Vec2::new(4.0, 28.0),
                            Align2::LEFT_CENTER,
                            doc.source,
                            FontId::proportional(10.0),
                            palette.muted,
                        );
                    }
                });
            let _ = view;
        });
}

// ---- Share, inbox and slash menu -------------------------------------------

pub(super) fn render_share(
    ctx: &egui::Context,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let screen = ctx.viewport_rect();
    egui::Window::new("share")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_size(Vec2::new(460.0, 340.0))
        .fixed_pos(Pos2::new(screen.right() - 480.0, screen.top() + 82.0))
        .frame(popover_frame(palette))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add(
                    egui::Button::new(
                        egui::RichText::new("Share").size(13.0).strong().color(palette.text),
                    )
                    .frame(false),
                );
                ui.add(
                    egui::Button::new(
                        egui::RichText::new("Publish").size(13.0).color(palette.muted),
                    )
                    .frame(false),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("×").size(16.0).color(palette.muted),
                            )
                            .frame(false),
                        )
                        .clicked()
                    {
                        commands.push(Command::CloseShare);
                    }
                    ui.label(
                        egui::RichText::new("Learn about sharing")
                            .size(11.0)
                            .color(palette.accent),
                    );
                });
            });
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                let mut invite = String::new();
                ui.add(
                    egui::TextEdit::singleline(&mut invite)
                        .desired_width(280.0)
                        .hint_text("Email or group, separated by commas")
                        .text_color(palette.text),
                );
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Invite").size(13.0).color(Color32::WHITE),
                        )
                        .fill(palette.accent)
                        .corner_radius(CornerRadius::same(6)),
                    )
                    .clicked()
                {
                    commands.push(Command::CloseShare);
                }
            });
            ui.add_space(14.0);

            let (row, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 44.0), Sense::hover());
            draw_person(
                ui.painter(),
                Rect::from_min_size(row.min + Vec2::new(2.0, 12.0), Vec2::splat(20.0)),
                palette.muted,
            );
            ui.painter().text(
                row.min + Vec2::new(32.0, 16.0),
                Align2::LEFT_CENTER,
                "Local workspace (You)",
                FontId::proportional(13.0),
                palette.text,
            );
            ui.painter().text(
                row.min + Vec2::new(32.0, 32.0),
                Align2::LEFT_CENTER,
                "Owner",
                FontId::proportional(11.0),
                palette.muted,
            );
            ui.painter().text(
                row.max + Vec2::new(-4.0, -22.0),
                Align2::RIGHT_CENTER,
                "Full access",
                FontId::proportional(12.0),
                palette.muted,
            );

            ui.add_space(8.0);
            section_label(ui, "General access", palette);
            let (general, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 32.0), Sense::hover());
            ui.painter().text(
                general.min + Vec2::new(4.0, 16.0),
                Align2::LEFT_CENTER,
                "Everyone with the link",
                FontId::proportional(13.0),
                palette.text,
            );
            ui.painter().text(
                general.max + Vec2::new(-4.0, -16.0),
                Align2::RIGHT_CENTER,
                "Full access",
                FontId::proportional(12.0),
                palette.muted,
            );
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Advanced").size(13.0).color(palette.text),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Copy link").size(13.0).color(palette.text),
                            )
                            .frame(false),
                        )
                        .clicked()
                    {
                        commands.push(Command::CloseShare);
                    }
                });
            });
            let _ = view;
        });
}

pub(super) fn render_inbox(
    ctx: &egui::Context,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let screen = ctx.viewport_rect();
    egui::Window::new("inbox")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_size(Vec2::new(420.0, 600.0))
        .fixed_pos(Pos2::new(screen.left() + 266.0, screen.top() + 78.0))
        .frame(popover_frame(palette))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Inbox")
                        .font(strong_font(18.0))
                        .color(palette.text),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("«").size(16.0).color(palette.muted),
                            )
                            .frame(false),
                        )
                        .clicked()
                    {
                        commands.push(Command::CloseInbox);
                    }
                });
            });
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(16.0);
            ui.label(
                egui::RichText::new("Last week").size(12.0).color(palette.muted),
            );
            ui.add_space(12.0);

            for (title, day) in [
                ("Your workspace export is ready", "Oct 30"),
                ("Your workspace export is ready", "Oct 30"),
            ] {
                let (row, _) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), 96.0),
                    Sense::hover(),
                );
                draw_inbox(
                    ui.painter(),
                    Rect::from_min_size(row.min + Vec2::new(2.0, 6.0), Vec2::splat(18.0)),
                    palette.accent,
                );
                ui.painter().text(
                    row.min + Vec2::new(30.0, 14.0),
                    Align2::LEFT_CENTER,
                    title,
                    FontId::proportional(13.0),
                    palette.text,
                );
                ui.painter().text(
                    row.min + Vec2::new(30.0, 34.0),
                    Align2::LEFT_CENTER,
                    "The download link will expire in 7 days.",
                    FontId::proportional(11.5),
                    palette.muted,
                );
                ui.painter().text(
                    row.max + Vec2::new(-6.0, -82.0),
                    Align2::RIGHT_CENTER,
                    day,
                    FontId::proportional(11.0),
                    palette.muted,
                );
                ui.painter().circle_filled(
                    row.max + Vec2::new(-6.0, -66.0),
                    3.5,
                    palette.accent,
                );
                let button_rect = Rect::from_min_size(
                    row.min + Vec2::new(30.0, 52.0),
                    Vec2::new(84.0, 26.0),
                );
                ui.painter().rect_stroke(
                    button_rect,
                    CornerRadius::same(5),
                    Stroke::new(1.0, palette.border),
                    egui::StrokeKind::Inside,
                );
                ui.painter().text(
                    button_rect.center(),
                    Align2::CENTER_CENTER,
                    "Download",
                    FontId::proportional(12.0),
                    palette.text,
                );
                ui.painter().hline(
                    row.x_range(),
                    row.max.y,
                    Stroke::new(1.0, palette.border),
                );
            }
            let _ = view;
        });
}

pub(super) fn render_slash(
    ctx: &egui::Context,
    block: &str,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    egui::Window::new("slash_menu")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_pos(view.slash_pos)
        .default_width(220.0)
        .frame(popover_frame(palette))
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new("Turn into")
                    .size(11.0)
                    .color(palette.muted),
            );
            ui.add_space(4.0);
            for (kind, label) in block_menu_options() {
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new(label).color(palette.text),
                        )
                        .frame(false),
                    )
                    .clicked()
                {
                    commands.push(Command::SetBlockKind(block.to_string(), kind));
                    commands.push(Command::CloseSlash);
                }
            }
            ui.separator();
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("Delete block").color(palette.text),
                    )
                    .frame(false),
                )
                .clicked()
            {
                commands.push(Command::DeleteBlock(block.to_string()));
                commands.push(Command::CloseSlash);
            }
        });
}
