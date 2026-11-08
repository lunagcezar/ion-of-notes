//! Floating windows: the "Open in new tab…" switcher, quick search / AI chat,
//! settings (preferences + AI assistant), share, inbox, shortcut sheet and the
//! slash menu (SRP). Drawing collects `Command`s; it never mutates state.

use crate::model::{Page, Workspace};
use crate::shortcuts::SHORTCUT_TABLE;
use egui::{Align2, Color32, CornerRadius, FontId, Margin, Pos2, Rect, Sense, Stroke, Vec2};

use super::app::{new_tab_choices, Command, FontStyle, NewTabChoice, ViewState};
use super::icons::{
    draw_grid, draw_help, draw_inbox, draw_link, draw_note, draw_page, draw_panel, draw_person,
    draw_search, draw_settings,
};
use super::palette::{strong_font, Palette};
use super::switcher::{render_switcher, SwitcherMode, SwitcherRow, SwitcherSpec};

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

// ---- Shared "Open in new tab…" / Search switcher (spec-013) ---------------

/// A breadcrumb-style suffix shown after a result title.
fn page_detail(ws: &Workspace, page: &Page) -> String {
    let trail = ws.breadcrumbs(&page.id);
    let parents: Vec<String> = trail[..trail.len().saturating_sub(1)].to_vec();
    let text = if parents.is_empty() {
        ws.name.clone()
    } else {
        parents.join(" / ")
    };
    format!("   — {text}")
}

fn build_new_tab_rows(ws: &Workspace, choices: &[NewTabChoice]) -> Vec<SwitcherRow> {
    choices
        .iter()
        .enumerate()
        .map(|(index, choice)| {
            let (label, icon, id, special, detail) = match choice {
                NewTabChoice::NewChat => (
                    "New chat".to_string(),
                    String::new(),
                    "new-chat".to_string(),
                    true,
                    String::new(),
                ),
                NewTabChoice::NewPage => (
                    "New page".to_string(),
                    String::new(),
                    "new-page".to_string(),
                    true,
                    String::new(),
                ),
                NewTabChoice::Page(id) => {
                    let page = ws.page(id);
                    (
                        page.map(|p| p.title.clone()).unwrap_or_else(|| "Untitled".to_string()),
                        page.map(|p| p.icon.clone()).unwrap_or_default(),
                        id.clone(),
                        false,
                        page.map(|p| page_detail(ws, p)).unwrap_or_default(),
                    )
                }
            };
            let group = match index {
                0 | 1 => "",
                2 => "Today",
                _ => "Past week",
            };
            SwitcherRow { label, detail, group, icon, id, special }
        })
        .collect()
}

fn build_search_rows(ws: &Workspace, view: &ViewState) -> Vec<SwitcherRow> {
    view.search_results
        .iter()
        .enumerate()
        .map(|(index, result)| {
            let page = ws.page(&result.page_id);
            SwitcherRow {
                label: result.page_title.clone(),
                detail: page.map(|p| page_detail(ws, p)).unwrap_or_default(),
                group: if index == 0 { "Today" } else { "Past week" },
                icon: page.map(|p| p.icon.clone()).unwrap_or_default(),
                id: result.page_id.clone(),
                special: false,
            }
        })
        .collect()
}

/// Build the shared spec, then hand it to the one switcher renderer (DRY).
fn switcher_spec<'a>(
    id: &'static str,
    hint: &'a str,
    query: &'a str,
    rows: &'a [SwitcherRow],
    selected: usize,
    mode: SwitcherMode,
    ws: &'a Workspace,
    view: &'a ViewState,
) -> SwitcherSpec<'a> {
    SwitcherSpec {
        id,
        hint,
        query,
        rows,
        selected,
        highlight: view.highlight_pane,
        filters: view.filters,
        mode,
        ws,
        ai_query: &view.ai_query,
        ai_status: &view.ai_status,
        ai_answer: view.ai_answer.as_deref(),
        ai_error: view.ai_error.as_deref(),
        ai_busy: view.ai_busy,
    }
}

pub(super) fn render_new_tab(
    ctx: &egui::Context,
    ws: &Workspace,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let choices = new_tab_choices(ws, &view.new_tab_query);
    let rows = build_new_tab_rows(ws, &choices);
    let spec = switcher_spec(
        "new_tab_switcher",
        "Open in new tab...",
        &view.new_tab_query,
        &rows,
        view.new_tab_selected,
        SwitcherMode::NewTab,
        ws,
        view,
    );
    render_switcher(ctx, &spec, palette, commands);
}

pub(super) fn render_search(
    ctx: &egui::Context,
    ws: &Workspace,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let rows = build_search_rows(ws, view);
    let spec = switcher_spec(
        "quick_search",
        "Search or ask a question...",
        &view.search_query,
        &rows,
        view.search_selected,
        SwitcherMode::Search,
        ws,
        view,
    );
    render_switcher(ctx, &spec, palette, commands);
}

// ---- Page settings and actions (the "..." menu, spec-013) ------------------

/// A clickable row with a label and an optional shortcut hint.
fn menu_button(
    ui: &mut egui::Ui,
    label: &str,
    hint: &str,
    palette: &Palette,
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 27.0), Sense::click());
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(5), palette.hover);
    }
    ui.painter().text(
        rect.min + Vec2::new(10.0, 13.5),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(13.0),
        palette.text,
    );
    if !hint.is_empty() {
        ui.painter().text(
            rect.max + Vec2::new(-8.0, -13.5),
            Align2::RIGHT_CENTER,
            hint,
            FontId::proportional(11.0),
            palette.muted,
        );
    }
    response
}

/// A row with a small on/off pill on the right.
fn toggle_button(
    ui: &mut egui::Ui,
    label: &str,
    on: bool,
    palette: &Palette,
) -> egui::Response {
    let response = menu_button(ui, label, "", palette);
    let rect = response.rect;
    let pill = Rect::from_min_size(
        rect.max + Vec2::new(-40.0, -21.5),
        Vec2::new(30.0, 16.0),
    );
    ui.painter().rect_filled(
        pill,
        CornerRadius::same(8),
        if on { palette.accent } else { palette.hover },
    );
    let knob_x = if on { pill.max.x - 9.0 } else { pill.min.x + 9.0 };
    ui.painter().circle_filled(
        Pos2::new(knob_x, pill.center().y),
        6.0,
        Color32::WHITE,
    );
    response
}

fn action_items(view: &ViewState) -> Vec<(&'static str, &'static str, Command)> {
    vec![
        ("Copy link", "Ctrl+L", Command::CopyLink),
        ("Copy page contents", "", Command::ClosePageActions),
        ("Duplicate", "Ctrl+D", Command::DuplicatePage),
        ("Move to", "Ctrl+Shift+P", Command::ClosePageActions),
        ("Version history", "", Command::ClosePageActions),
        ("Updates & analytics", "", Command::ClosePageActions),
        ("Suggest edits", "", Command::ClosePageActions),
        ("Use with AI", "", Command::OpenAiChat),
        ("Notify me", "Comments", Command::ClosePageActions),
        ("Archive", "", Command::ClosePageActions),
        (
            "Move to Trash",
            "",
            Command::DeletePage(view.current_page.clone()),
        ),
        ("Show / hide sidebar", "Ctrl+\\", Command::ToggleSidebar),
        ("More", "", Command::ClosePageActions),
    ]
}

pub(super) fn render_page_actions(
    ctx: &egui::Context,
    ws: &Workspace,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let screen = ctx.viewport_rect();
    let resp = egui::Window::new("page_actions")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_size(Vec2::new(300.0, 620.0))
        .fixed_pos(Pos2::new(screen.right() - 316.0, screen.top() + 76.0))
        .frame(
            egui::Frame::NONE
                .fill(palette.content)
                .corner_radius(CornerRadius::same(10))
                .inner_margin(Margin::same(12))
                .stroke(Stroke::new(1.0, palette.border)),
        )
        .show(ctx, |ui| {
            // Search actions field (filters the list below).
            let id = egui::Id::unique("page_actions_query");
            let mut query: String = ctx.data(|d| d.get_temp(id).unwrap_or_default());
            let field = ui.add(
                egui::TextEdit::singleline(&mut query)
                    .desired_width(f32::INFINITY)
                    .hint_text("Search actions...")
                    .text_color(palette.text),
            );
            if field.changed() {
                ctx.data_mut(|d| d.insert_temp(id, query.clone()));
            }
            ui.add_space(8.0);

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let q = query.trim().to_lowercase();
                    for (label, hint, command) in action_items(view) {
                        if !q.is_empty() && !label.to_lowercase().contains(&q) {
                            continue;
                        }
                        if menu_button(ui, label, hint, palette).clicked() {
                            commands.push(command);
                            commands.push(Command::ClosePageActions);
                        }
                    }
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(6.0);

                    if q.is_empty() {
                        // Appearance & layout.
                        ui.label(
                            egui::RichText::new("Appearance & layout")
                                .size(11.0)
                                .color(palette.muted),
                        );
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            for (label, style) in [
                                ("Default", FontStyle::Default),
                                ("Serif", FontStyle::Serif),
                                ("Mono", FontStyle::Mono),
                            ] {
                                let active = view.font_style == style;
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new(label).size(12.0).color(
                                                if active { palette.text } else { palette.muted },
                                            ),
                                        )
                                        .fill(if active {
                                            palette.selected
                                        } else {
                                            Color32::TRANSPARENT
                                        })
                                        .corner_radius(CornerRadius::same(5)),
                                    )
                                    .clicked()
                                {
                                    commands.push(Command::SetFontStyle(style));
                                }
                            }
                        });
                        ui.add_space(4.0);
                        if toggle_button(ui, "Small text", view.small_text, palette).clicked() {
                            commands.push(Command::ToggleSmallText);
                        }
                        if toggle_button(ui, "Full width", view.full_width, palette).clicked() {
                            commands.push(Command::ToggleFullWidth);
                        }
                        if toggle_button(ui, "Available offline", view.available_offline, palette)
                            .clicked()
                        {
                            commands.push(Command::ToggleAvailableOffline);
                        }
                        if toggle_button(ui, "Lock page", view.lock_page, palette).clicked() {
                            commands.push(Command::ToggleLockPage);
                        }
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(6.0);

                        // Last edited footer.
                        let words = ws
                            .page(&view.current_page)
                            .map(|p| p.plain_text().split_whitespace().count())
                            .unwrap_or(0);
                        ui.label(
                            egui::RichText::new("Last edited")
                                .size(11.0)
                                .color(palette.muted),
                        );
                        ui.add_space(2.0);
                        ui.label(
                            egui::RichText::new("Local workspace · just now")
                                .size(11.0)
                                .color(palette.text),
                        );
                        ui.label(
                            egui::RichText::new(format!("{words} words"))
                                .size(10.5)
                                .color(palette.muted),
                        );
                    }
                });
        });
    if let Some(inner) = resp
        && ctx.input(|i| i.pointer.any_click())
        && ctx.input(|i| i.pointer.interact_pos().map_or(false, |p| !inner.response.rect.contains(p)))
    {
        commands.push(Command::ClosePageActions);
    }
}

// ---- Settings --------------------------------------------------------------

pub(super) fn render_settings(
    ctx: &egui::Context,
    ws: &Workspace,
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
                // Left navigation column. Mail & Calendar is intentionally not
                // implemented, so it is not offered in the rail.
                ui.vertical(|ui| {
                    ui.set_width(288.0);
                    ui.add_space(18.0);
                    let (search, _) =
                        ui.allocate_exact_size(Vec2::new(264.0, 30.0), Sense::hover());
                    ui.painter().rect_filled(search, CornerRadius::same(6), palette.hover);
                    draw_search(
                        ui.painter(),
                        Rect::from_min_size(search.min + Vec2::new(8.0, 7.0), Vec2::splat(16.0)),
                        palette.muted,
                    );
                    let mut query = view.settings_query.clone();
                    let edit = ui.put(
                        Rect::from_min_size(search.min + Vec2::new(28.0, 2.0), Vec2::new(228.0, 26.0)),
                        egui::TextEdit::singleline(&mut query)
                            .frame(egui::Frame::NONE)
                            .hint_text("Search settings")
                            .text_color(palette.text),
                    );
                    if edit.changed() {
                        commands.push(Command::SetSettingsQuery(query));
                    }
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
                            match page {
                                "AI assistant" => render_ai_settings(ui, view, palette, commands),
                                "Preferences" => render_preferences(ui, view, palette, commands),
                                "General" => render_workspace_settings(ui, ws, view, palette, commands),
                                "Sidebar" => render_sidebar_settings(ui, view, palette, commands),
                                "Offline" => render_offline_settings(ui, ws, view, palette, commands),
                                "People" => render_people_settings(ui, ws, view, palette, commands),
                                _ => {
                                    ui.label(
                                        egui::RichText::new(
                                            "Settings for this section live on your self-hosted workspace.",
                                        )
                                        .size(13.0)
                                        .color(palette.muted),
                                    );
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
    if let Some(inner) = resp
        && ctx.input(|i| i.pointer.any_click())
        && ctx.input(|i| i.pointer.interact_pos().map_or(false, |p| !inner.response.rect.contains(p)))
    {
        commands.push(Command::CloseSettings);
    }
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
    let (lang_rect, lang_resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 46.0), Sense::click());
    if lang_resp.hovered() {
        ui.painter().rect_filled(lang_rect, CornerRadius::same(6), palette.hover);
    }
    ui.painter().text(
        lang_rect.min + Vec2::new(0.0, 14.0),
        Align2::LEFT_CENTER,
        rust_i18n::t!("language"),
        FontId::proportional(12.5),
        palette.text,
    );
    ui.painter().text(
        lang_rect.min + Vec2::new(0.0, 32.0),
        Align2::LEFT_CENTER,
        rust_i18n::t!("language_description"),
        FontId::proportional(11.0),
        palette.muted,
    );
    let lang_label = crate::i18n::label_for(&view.language);
    let chip = Rect::from_min_size(Pos2::new(lang_rect.max.x - 170.0, lang_rect.min.y + 9.0), Vec2::new(170.0, 26.0));
    ui.painter().rect_stroke(chip, CornerRadius::same(6), Stroke::new(1.0, palette.border), egui::StrokeKind::Inside);
    ui.painter().text(chip.center(), Align2::CENTER_CENTER, format!("{lang_label} ⌄"), FontId::proportional(12.0), palette.text);
    if lang_resp.clicked() {
        let codes: Vec<&str> = crate::i18n::SUPPORTED.iter().map(|(c, _)| *c).collect();
        let idx = codes.iter().position(|c| *c == view.language).unwrap_or(0);
        let next = codes[(idx + 1) % codes.len()];
        commands.push(Command::SetLanguage(next.to_string()));
    }
    display_row(
        ui,
        "Number format",
        "Choose how numbers and currencies are formatted",
        "1.000.000,00 ⌄",
        palette,
    );
    display_row(
        ui,
        "Time format",
        "Choose 12-hour or 24-hour time display",
        "24-hour",
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

fn render_workspace_settings(
    ui: &mut egui::Ui,
    ws: &Workspace,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    ui.label(
        egui::RichText::new("Workspace")
            .font(strong_font(17.0))
            .color(palette.text),
    );
    ui.add_space(12.0);
    ui.separator();
    ui.add_space(12.0);
    display_row(ui, "Workspace name", "Shown in the sidebar and exports", &ws.name, palette);
    display_row(ui, "Workspace icon", "Customize the workspace identity", "Default", palette);
    ui.add_space(18.0);
    ui.label(egui::RichText::new("Members & access").font(strong_font(17.0)).color(palette.text));
    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);
    display_row(ui, "Members", "Manage who can access this workspace", "Manage", palette);
    if ui.add(egui::Button::new("Invite members")).clicked() {
        commands.push(Command::OpenShare);
    }
    ui.add_space(18.0);
    ui.label(egui::RichText::new("Data & export").font(strong_font(17.0)).color(palette.text));
    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new("Export includes pages, version history metadata, and People.")
            .size(12.0)
            .color(palette.muted),
    );
    let _ = (view, commands);
}

fn render_sidebar_settings(
    ui: &mut egui::Ui,
    _view: &ViewState,
    palette: &Palette,
    _commands: &mut Vec<Command>,
) {
    ui.label(
        egui::RichText::new("Customize sidebar")
            .font(strong_font(17.0))
            .color(palette.text),
    );
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new("Choose what appears in the sidebar. Drag-and-drop ordering is planned; toggles are available here.")
            .size(12.0)
            .color(palette.muted),
    );
    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    for item in ["Search", "Library", "Settings", "Inbox", "Favorites", "Private pages", "Customize sidebar"] {
        display_row(ui, item, "Show in sidebar", "Visible", palette);
    }
}

fn render_offline_settings(
    ui: &mut egui::Ui,
    ws: &Workspace,
    view: &ViewState,
    palette: &Palette,
    _commands: &mut Vec<Command>,
) {
    ui.label(egui::RichText::new("Offline pages").font(strong_font(17.0)).color(palette.text));
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new("Pages marked \"Available offline\" are kept for offline access and listed here.")
            .size(12.0)
            .color(palette.muted),
    );
    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    let mut found = false;
    for page in ws.pages.iter().filter(|p| p.available_offline) {
        found = true;
        ui.label(egui::RichText::new(format!("• {}", page.title)).color(palette.text));
    }
    if !found {
        ui.label(
            egui::RichText::new("No offline pages yet. Use Page actions → Available offline.")
                .size(12.0)
                .color(palette.muted),
        );
    }
    let _ = view;
}

fn render_people_settings(
    ui: &mut egui::Ui,
    _ws: &Workspace,
    _view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    ui.label(egui::RichText::new("People").font(strong_font(17.0)).color(palette.text));
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new("Members and guests are managed by your self-hosted workspace server.")
            .size(12.0)
            .color(palette.muted),
    );
    ui.add_space(12.0);
    if ui.add(egui::Button::new("Invite members")).clicked() {
        commands.push(Command::OpenShare);
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
    let resp = egui::Window::new("shortcut_sheet")
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
    if let Some(inner) = resp
        && ctx.input(|i| i.pointer.any_click())
        && ctx.input(|i| i.pointer.interact_pos().map_or(false, |p| !inner.response.rect.contains(p)))
    {
        commands.push(Command::CloseShortcuts);
    }
}

// ---- App / workspace / grid menus -----------------------------------------

fn popup_menu(
    ctx: &egui::Context,
    id: &str,
    pos: Pos2,
    width: f32,
    palette: &Palette,
    items: &[(&str, &str, Command)],
    commands: &mut Vec<Command>,
    close: Command,
) {
    let resp = egui::Window::new(id)
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_pos(pos)
        .default_width(width)
        .frame(
            egui::Frame::NONE
                .fill(palette.content)
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::same(6))
                .stroke(Stroke::new(1.0, palette.border)),
        )
        .show(ctx, |ui| {
            for (label, hint, cmd) in items {
                if *label == "---" {
                    ui.separator();
                    continue;
                }
                let r = menu_button(ui, label, hint, palette);
                if r.clicked() {
                    commands.push(cmd.clone());
                    commands.push(close.clone());
                }
            }
        });
    if let Some(inner) = resp
        && ctx.input(|i| i.pointer.any_click())
        && ctx.input(|i| i.pointer.interact_pos().map_or(false, |p| !inner.response.rect.contains(p)))
    {
        commands.push(close);
    }
}

pub(super) fn render_app_menu(
    ctx: &egui::Context,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let screen = ctx.viewport_rect();
    let items = vec![
        ("File", "", Command::NoOp),
        ("  New page", "Ctrl+N", Command::NewPage(None)),
        ("  New tab", "Ctrl+T", Command::OpenNewTab),
        ("  Close tab", "Ctrl+W", Command::CloseTab(usize::MAX)),
        ("---", "", Command::NoOp),
        ("Edit", "", Command::NoOp),
        ("  Undo", "Ctrl+Z", Command::Undo),
        ("  Redo", "Ctrl+Shift+Z", Command::Redo),
        ("  Copy link", "Ctrl+L", Command::CopyLink),
        ("---", "", Command::NoOp),
        ("View", "", Command::NoOp),
        ("  Toggle sidebar", "Ctrl+\\", Command::ToggleSidebar),
        ("  Zoom in", "Ctrl++", Command::ZoomIn),
        ("  Zoom out", "Ctrl+-", Command::ZoomOut),
        ("  Full screen", "F11", Command::ToggleFullscreen),
        ("---", "", Command::NoOp),
        ("History", "", Command::NoOp),
        ("  Back", "Ctrl+[", Command::GoBack),
        ("  Forward", "Ctrl+]", Command::GoForward),
        ("---", "", Command::NoOp),
        ("Window", "", Command::NoOp),
        ("  Minimize", "", Command::WindowMinimize),
        ("  Next tab", "Ctrl+Tab", Command::NextTab),
        ("---", "", Command::NoOp),
        ("Help", "", Command::NoOp),
        ("  Keyboard shortcuts", "F1", Command::OpenShortcuts),
        ("  Settings", "Ctrl+,", Command::OpenSettings),
    ];
    popup_menu(
        ctx,
        "app_menu",
        Pos2::new(screen.left() + 38.0, screen.top() + 42.0),
        260.0,
        palette,
        &items,
        commands,
        Command::CloseAppMenu,
    );
}

pub(super) fn render_workspace_menu(
    ctx: &egui::Context,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let screen = ctx.viewport_rect();
    let resp = egui::Window::new("workspace_menu")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_pos(Pos2::new(screen.left() + 12.0, screen.bottom() - 250.0))
        .default_width(240.0)
        .frame(
            egui::Frame::NONE
                .fill(palette.content)
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::same(6))
                .stroke(Stroke::new(1.0, palette.border)),
        )
        .show(ctx, |ui| {
            let items: &[(&str, Command, Option<&str>)] = &[
                ("Workspace settings", Command::OpenSettings, Some("General")),
                ("Invite members", Command::OpenShare, None),
                ("Add account", Command::OpenSettings, Some("People")),
                ("---", Command::NoOp, None),
                ("Switch workspace", Command::OpenLibrary, None),
                ("Create new workspace", Command::OpenSettings, Some("General")),
            ];
            for (label, cmd, page) in items {
                if *label == "---" {
                    ui.separator();
                    continue;
                }
                let r = menu_button(ui, label, "", palette);
                if r.clicked() {
                    if let Some(p) = page {
                        commands.push(Command::SetSettingsPage((*p).to_string()));
                    }
                    commands.push(cmd.clone());
                    commands.push(Command::CloseWorkspaceMenu);
                }
            }
        });
    if let Some(inner) = resp
        && ctx.input(|i| i.pointer.any_click())
        && ctx.input(|i| i.pointer.interact_pos().map_or(false, |p| !inner.response.rect.contains(p)))
    {
        commands.push(Command::CloseWorkspaceMenu);
    }
}

pub(super) fn render_grid_menu(
    ctx: &egui::Context,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let screen = ctx.viewport_rect();
    let items = vec![
        ("Library", "", Command::OpenLibrary),
        ("Skills", "", Command::SetSettingsPage("AI assistant".into())),
        ("Trash", "", Command::OpenLibrary),
        ("---", "", Command::NoOp),
        ("Settings", "", Command::OpenSettings),
    ];
    popup_menu(
        ctx,
        "grid_menu",
        Pos2::new(screen.left() + 220.0, screen.bottom() - 220.0),
        180.0,
        palette,
        &items,
        commands,
        Command::CloseGridMenu,
    );
}

// ---- Share, inbox and slash menu -------------------------------------------

pub(super) fn render_share(
    ctx: &egui::Context,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let screen = ctx.viewport_rect();
    let resp = egui::Window::new("share")
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
    if let Some(inner) = resp
        && ctx.input(|i| i.pointer.any_click())
        && ctx.input(|i| i.pointer.interact_pos().map_or(false, |p| !inner.response.rect.contains(p)))
    {
        commands.push(Command::CloseShare);
    }
}

pub(super) fn render_inbox(
    ctx: &egui::Context,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let screen = ctx.viewport_rect();
    let resp = egui::Window::new("inbox")
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
    if let Some(inner) = resp
        && ctx.input(|i| i.pointer.any_click())
        && ctx.input(|i| i.pointer.interact_pos().map_or(false, |p| !inner.response.rect.contains(p)))
    {
        commands.push(Command::CloseInbox);
    }
}

/// One entry of the block menu: a label and the kind it turns the block into.
/// Entries without a kind are display-only placeholders for features that are
/// deliberately out of scope (media, embeds, databases).
fn slash_item(
    ui: &mut egui::Ui,
    label: &str,
    hint: &str,
    palette: &Palette,
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 28.0), Sense::click());
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(5), palette.hover);
    }
    ui.painter().text(
        rect.min + Vec2::new(8.0, 14.0),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(13.0),
        palette.text,
    );
    if !hint.is_empty() {
        ui.painter().text(
            rect.max + Vec2::new(-8.0, -14.0),
            Align2::RIGHT_CENTER,
            hint,
            FontId::proportional(11.0),
            palette.muted,
        );
    }
    response
}

fn slash_group(ui: &mut egui::Ui, title: &str, palette: &Palette) {
    ui.add_space(6.0);
    ui.label(
        egui::RichText::new(title)
            .size(10.5)
            .color(palette.muted),
    );
    ui.add_space(2.0);
}

/// The block menu opened by the "+", the six-dot handle or typing "/".
/// It mirrors the categorized menu in the attachments (Suggested, Basic
/// blocks, Media, Database, Advanced blocks, Inline, Embeds).
#[allow(clippy::too_many_lines)]
pub(super) fn render_slash(
    ctx: &egui::Context,
    block: &str,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    use crate::model::BlockKind;
    egui::Window::new("slash_menu")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_pos(view.slash_pos)
        .default_width(260.0)
        .frame(popover_frame(palette))
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(330.0)
                .show(ui, |ui| {
                    let apply = |commands: &mut Vec<Command>, kind: BlockKind| {
                        commands.push(Command::SetBlockKind(block.to_string(), kind));
                    };
                    slash_group(ui, "Suggested", palette);
                    for label in ["AI Meeting Notes", "HTML · Embeds", "AI Image"] {
                        slash_item(ui, label, "", palette);
                    }
                    slash_group(ui, "Basic blocks", palette);
                    for (label, hint, kind) in [
                        ("Text", "", BlockKind::Paragraph),
                        ("Heading 1", "#", BlockKind::Heading1),
                        ("Heading 2", "##", BlockKind::Heading2),
                        ("Heading 3", "###", BlockKind::Heading3),
                        ("Bulleted list", "-", BlockKind::Bulleted),
                        ("Numbered list", "1.", BlockKind::Numbered),
                        ("To-do list", "[]", BlockKind::Todo),
                        ("Quote", "\"", BlockKind::Quote),
                        ("Callout", "", BlockKind::Callout),
                        ("Code", "", BlockKind::Code),
                        ("Divider", "---", BlockKind::Divider),
                    ] {
                        if slash_item(ui, label, hint, palette).clicked() {
                            apply(commands, kind);
                        }
                    }
                    slash_group(ui, "Media", palette);
                    for label in ["Image", "Video", "Audio", "File", "Web bookmark"] {
                        slash_item(ui, label, "", palette);
                    }
                    slash_group(ui, "Database", palette);
                    for label in ["Table view", "Board view", "Gallery view", "List view"] {
                        slash_item(ui, label, "", palette);
                    }
                    slash_group(ui, "Advanced blocks", palette);
                    for label in [
                        "Table of contents", "Block equation", "Button", "Breadcrumb",
                        "Tabs", "Synced block",
                    ] {
                        slash_item(ui, label, "", palette);
                    }
                    slash_group(ui, "Inline", palette);
                    for label in ["Mention a person", "Mention a page", "Date or reminder", "Emoji"] {
                        slash_item(ui, label, "", palette);
                    }
                    slash_group(ui, "Embeds", palette);
                    for label in ["Embed", "Google Drive", "Tweet"] {
                        slash_item(ui, label, "", palette);
                    }
                });
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(4.0);
            if slash_item(ui, "Delete block", "", palette).clicked() {
                commands.push(Command::DeleteBlock(block.to_string()));
                commands.push(Command::CloseSlash);
            }
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Type to filter…")
                        .size(11.0)
                        .color(palette.muted),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new("Close menu  esc")
                            .size(10.5)
                            .color(palette.muted),
                    );
                });
            });
        });
}

