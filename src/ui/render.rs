//! Pure rendering of the window chrome, sidebar, page, blocks and the
//! inline database (SRP). Render functions collect `Command`s; they never
//! mutate state directly.

use crate::model::{Block, BlockKind, Page, Workspace};
use egui::{
    Align2, Color32, CornerRadius, FontFamily, FontId, Margin, Pos2, Rect, Sense, Stroke, Vec2,
};

use super::app::{Command, FontStyle, ViewState};
use super::icons::{
    draw_arrow_left, draw_arrow_right, draw_bullet_dot, draw_chevron, draw_close, draw_dots,
    draw_ellipsis, draw_grid, draw_help, draw_inbox, draw_library, draw_lines, draw_link,
    draw_menu, draw_minus, draw_page, draw_page_icon, draw_panel, draw_pen,
    draw_person, draw_plus, draw_search, draw_settings, draw_share, draw_sliders, draw_square,
    draw_star, glyph_button, icon_button,
};
use super::palette::{status_color, strong_font, tab_title, Palette};

/// Allocate a sidebar row and paint its hover / selection background.
/// Shared by the labelled rows and the page-icon rows (DRY).
fn nav_row_rect(
    ui: &mut egui::Ui,
    width: f32,
    selected: bool,
    palette: &Palette,
) -> (Rect, egui::Response) {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 30.0), Sense::click());
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
    (rect, response)
}

/// A full-width sidebar row with an icon and a label.
fn nav_row(
    ui: &mut egui::Ui,
    width: f32,
    label: &str,
    selected: bool,
    palette: &Palette,
    draw: impl FnOnce(&egui::Painter, Rect, Color32),
) -> egui::Response {
    let (rect, response) = nav_row_rect(ui, width, selected, palette);
    let icon_rect = Rect::from_min_size(rect.min + Vec2::new(10.0, 7.0), Vec2::splat(16.0));
    draw(ui.painter(), icon_rect, palette.muted);
    ui.painter().text(
        rect.min + Vec2::new(36.0, 15.0),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(13.5),
        palette.text,
    );
    response
}

fn section_label(ui: &mut egui::Ui, text: &str, palette: &Palette) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 24.0), Sense::hover());
    ui.painter().text(
        rect.min + Vec2::new(12.0, 12.0),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(11.5),
        palette.muted,
    );
}

// ---- Custom window chrome -------------------------------------------------

/// Height of the compact, Notion-like tab strip (spec-013). The breadcrumb row
/// sits directly below it, so the whole top chrome stays small.
pub(super) const TITLE_BAR_HEIGHT: f32 = 36.0;

pub(super) fn render_title_bar(
    ui: &mut egui::Ui,
    ws: &Workspace,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let h = TITLE_BAR_HEIGHT;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        ui.add_space(6.0);

        // Compact show/hide sidebar (Ctrl+\\).
        let panel = glyph_button(ui, Vec2::new(26.0, h - 6.0), palette, draw_panel)
            .on_hover_text("Show / hide sidebar (Ctrl+\\)");
        if panel.clicked() {
            commands.push(Command::ToggleSidebar);
        }
        // App menu: File, Edit, View, History, Window, Help.
        let menu = glyph_button(ui, Vec2::new(26.0, h - 6.0), palette, draw_menu)
            .on_hover_text("Menu");
        if menu.clicked() {
            commands.push(Command::OpenAppMenu);
        }

        // A compact strip: no person/settings control (the reference top bar
        // has none — Settings lives in the sidebar), just back/forward, the
        // tab(s), the "+" and the window controls.
        ui.add_space(186.0);
        if glyph_button(ui, Vec2::new(26.0, h), palette, draw_arrow_left).clicked() {
            commands.push(Command::GoBack);
        }
        if glyph_button(ui, Vec2::new(26.0, h), palette, draw_arrow_right).clicked() {
            commands.push(Command::GoForward);
        }
        ui.add_space(12.0);

        for (i, tab_id) in view.tabs.iter().enumerate() {
            let active = i == view.active_tab;
            let label = if view.library_open && active {
                "Library".to_string()
            } else {
                ws.page(tab_id)
                    .map(|p| tab_title(&p.title))
                    .unwrap_or_else(|| "Untitled".to_string())
            };
            let icon = if view.library_open && active {
                String::new()
            } else {
                ws.page(tab_id).map(|p| p.icon.clone()).unwrap_or_default()
            };
            let (tab_rect, response) =
                ui.allocate_exact_size(Vec2::new(190.0, h - 8.0), Sense::click());
            if active || response.hovered() {
                ui.painter()
                    .rect_filled(tab_rect, CornerRadius::same(6), palette.hover);
            }
            let icon_rect =
                Rect::from_min_size(tab_rect.min + Vec2::new(10.0, 6.0), Vec2::splat(16.0));
            draw_page_icon(ui.painter(), icon_rect, &icon, palette, false);
            ui.painter().text(
                tab_rect.min + Vec2::new(34.0, (h - 8.0) / 2.0),
                Align2::LEFT_CENTER,
                &label,
                FontId::proportional(13.0),
                palette.text,
            );
            if response.clicked() && !active {
                commands.push(Command::SelectTab(i));
            }
            ui.add_space(6.0);
        }
        // The "+" opens the "Open in new tab…" switcher, not a blank page.
        if glyph_button(ui, Vec2::new(26.0, h), palette, draw_plus)
            .on_hover_text("Open in new tab")
            .clicked()
        {
            commands.push(Command::OpenNewTab);
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_space(6.0);
            if glyph_button(ui, Vec2::new(38.0, h), palette, draw_close).clicked() {
                commands.push(Command::WindowClose);
            }
            if glyph_button(ui, Vec2::new(38.0, h), palette, draw_square).clicked() {
                commands.push(Command::WindowMaximize);
            }
            if glyph_button(ui, Vec2::new(38.0, h), palette, draw_minus).clicked() {
                commands.push(Command::WindowMinimize);
            }
        });
    });
}

// ---- Sidebar --------------------------------------------------------------

pub(super) fn render_sidebar(
    ui: &mut egui::Ui,
    ws: &Workspace,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let width = ui.available_width() - 20.0;
    ui.add_space(12.0);

    // Search box with the Ctrl+K hint.
    let (rect, search_resp) = ui.allocate_exact_size(Vec2::new(width, 34.0), Sense::click());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(6), palette.hover);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(6),
        Stroke::new(1.0, palette.border),
        egui::StrokeKind::Inside,
    );
    draw_search(
        ui.painter(),
        Rect::from_min_size(rect.min + Vec2::new(9.0, 9.0), Vec2::splat(16.0)),
        palette.muted,
    );
    ui.painter().text(
        rect.min + Vec2::new(34.0, 17.0),
        Align2::LEFT_CENTER,
        "Search or ask",
        FontId::proportional(13.0),
        palette.muted,
    );
    ui.painter().text(
        rect.max + Vec2::new(-10.0, -17.0),
        Align2::RIGHT_CENTER,
        "Ctrl+K",
        FontId::proportional(11.0),
        palette.muted,
    );
    if search_resp.clicked() {
        commands.push(Command::OpenSearch);
    }

    ui.add_space(16.0);

    // The page list scrolls when it grows beyond the space left above the
    // fixed bottom group (spec-013: sidebar overflow).
    let bottom_reserve = 176.0;
    let scroll_height = (ui.available_height() - bottom_reserve).max(120.0);
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .max_height(scroll_height)
        .show(ui, |ui| {
            ui.set_width(width);
            if nav_row(ui, width, "Library", view.library_open, palette, draw_library)
                .clicked()
            {
                commands.push(Command::OpenLibrary);
            }
            if nav_row(ui, width, "Settings", false, palette, draw_settings).clicked() {
                commands.push(Command::OpenSettings);
            }
            if nav_row(ui, width, "Inbox", false, palette, draw_inbox).clicked() {
                commands.push(Command::OpenInbox);
            }

            ui.add_space(18.0);
            section_label(ui, "Favorites", palette);
            let favorites = ws.favorites();
            if favorites.is_empty() {
                section_label(ui, "No favorites yet", palette);
            } else {
                for page in favorites {
                    let selected = view.current_page == page.id && !view.library_open;
                    let (rect, response) = nav_row_rect(ui, width, selected, palette);
                    draw_page_icon(
                        ui.painter(),
                        Rect::from_min_size(rect.min + Vec2::new(10.0, 7.0), Vec2::splat(16.0)),
                        &page.icon,
                        palette,
                        false,
                    );
                    ui.painter().text(
                        rect.min + Vec2::new(36.0, 15.0),
                        Align2::LEFT_CENTER,
                        &page.title,
                        FontId::proportional(13.5),
                        palette.text,
                    );
                    if response.clicked() {
                        commands.push(Command::SelectPage(page.id.clone()));
                    }
                }
            }

            ui.add_space(18.0);
            section_label(ui, "Private", palette);
            render_tree(ui, ws, None, 0, width, view, palette, commands);
            if nav_row(ui, width, "New page", false, palette, draw_plus).clicked() {
                commands.push(Command::NewPage(None));
            }
        });

    // Bottom group: anchored to the bottom of the sidebar.
    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
        // Account row + workspace menu + grid menu.
        let (row, row_resp) = ui.allocate_exact_size(Vec2::new(width, 40.0), Sense::click());
        if row_resp.hovered() {
            ui.painter().rect_filled(row, CornerRadius::same(6), palette.hover);
        }
        ui.painter().circle_filled(row.min + Vec2::new(22.0, 20.0), 13.0, palette.accent);
        ui.painter().text(
            row.min + Vec2::new(22.0, 20.0),
            Align2::CENTER_CENTER,
            "N",
            FontId::proportional(12.0),
            Color32::WHITE,
        );
        ui.painter().text(
            row.min + Vec2::new(44.0, 20.0),
            Align2::LEFT_CENTER,
            "Local workspace",
            FontId::proportional(13.0),
            palette.text,
        );
        draw_chevron(
            ui.painter(),
            Rect::from_min_size(row.min + Vec2::new(164.0, 12.0), Vec2::splat(16.0)),
            view.workspace_menu_open,
            palette.muted,
        );
        let help_rect = Rect::from_min_size(row.min + Vec2::new(width - 46.0, 12.0), Vec2::splat(16.0));
        let grid_rect = Rect::from_min_size(row.min + Vec2::new(width - 26.0, 12.0), Vec2::splat(16.0));
        draw_help(ui.painter(), help_rect, palette.muted);
        draw_grid(ui.painter(), grid_rect, palette.muted);
        if row_resp.clicked() {
            if let Some(pos) = row_resp.interact_pointer_pos() {
                if grid_rect.expand(4.0).contains(pos) {
                    commands.push(Command::ToggleGridMenu);
                } else if help_rect.expand(4.0).contains(pos) {
                    commands.push(Command::OpenShortcuts);
                } else {
                    commands.push(Command::ToggleWorkspaceMenu);
                }
            } else {
                commands.push(Command::ToggleWorkspaceMenu);
            }
        }

        // Invite-members card (same footprint as the pictured sidebar card).
        // Members, sharing and collaboration live on the self-hosted server.
        let (card, card_resp) = ui.allocate_exact_size(Vec2::new(width, 76.0), Sense::click());
        ui.painter()
            .rect_filled(card, CornerRadius::same(8), palette.callout);
        if card_resp.hovered() {
            ui.painter().rect_stroke(
                card,
                CornerRadius::same(8),
                Stroke::new(1.0, palette.accent),
                egui::StrokeKind::Inside,
            );
        }
        draw_person(
            ui.painter(),
            Rect::from_min_size(card.min + Vec2::new(12.0, 12.0), Vec2::splat(16.0)),
            palette.muted,
        );
        ui.painter().text(
            card.min + Vec2::new(36.0, 20.0),
            Align2::LEFT_CENTER,
            "Invite members",
            FontId::proportional(13.0),
            palette.accent,
        );
        ui.painter().text(
            card.min + Vec2::new(12.0, 42.0),
            Align2::LEFT_CENTER,
            "Collaborate with your team.",
            FontId::proportional(11.0),
            palette.muted,
        );
        if card_resp.clicked() {
            commands.push(Command::OpenShare);
        }
        ui.add_space(8.0);

        // Customize sidebar (placeholder for drag/drop customization).
        let (custom, custom_resp) = ui.allocate_exact_size(Vec2::new(width, 28.0), Sense::click());
        if custom_resp.hovered() {
            ui.painter().rect_filled(custom, CornerRadius::same(6), palette.hover);
        }
        draw_sliders(
            ui.painter(),
            Rect::from_min_size(custom.min + Vec2::new(10.0, 6.0), Vec2::splat(16.0)),
            palette.muted,
        );
        ui.painter().text(
            custom.min + Vec2::new(36.0, 14.0),
            Align2::LEFT_CENTER,
            "Customize sidebar",
            FontId::proportional(12.5),
            palette.muted,
        );
        if custom_resp.clicked() {
            commands.push(Command::SetSettingsPage("Sidebar".to_string()));
            commands.push(Command::OpenSettings);
        }
        ui.add_space(10.0);
    });
}

#[allow(clippy::too_many_arguments)]
fn render_tree(
    ui: &mut egui::Ui,
    ws: &Workspace,
    parent: Option<&str>,
    depth: usize,
    width: f32,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let children = ws.children(parent);
    for page in children {
        let has_children = !ws.children(Some(&page.id)).is_empty();
        let open = view.expanded.contains(&page.id);
        let indent = 10.0 + depth as f32 * 16.0;
        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 28.0), Sense::click());
        let fill = if view.current_page == page.id && !view.library_open {
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
        let glyph_rect =
            Rect::from_min_size(rect.min + Vec2::new(indent, 6.0), Vec2::splat(16.0));
        if has_children {
            draw_chevron(ui.painter(), glyph_rect, open, palette.muted);
        } else {
            draw_page_icon(ui.painter(), glyph_rect, &page.icon, palette, false);
        }
        ui.painter().text(
            rect.min + Vec2::new(indent + 24.0, 14.0),
            Align2::LEFT_CENTER,
            &page.title,
            FontId::proportional(13.0),
            palette.text,
        );
        if response.clicked() {
            let on_chevron = response
                .interact_pointer_pos()
                .map(|pos| glyph_rect.contains(pos))
                .unwrap_or(false);
            if has_children && on_chevron {
                commands.push(Command::ToggleExpand(page.id.clone()));
            } else {
                commands.push(Command::SelectPage(page.id.clone()));
            }
        }
        if open {
            render_tree(
                ui,
                ws,
                Some(&page.id),
                depth + 1,
                width,
                view,
                palette,
                commands,
            );
        }
    }
}

// ---- Content area ---------------------------------------------------------

pub(super) fn render_content(
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
            if !view.sidebar_open
                && ui.add(egui::Button::new(">").frame(false)).clicked()
            {
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
                        egui::RichText::new("Share").size(13.0).color(palette.text),
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

fn crumb(
    ui: &mut egui::Ui,
    icon: &str,
    text: &str,
    palette: &Palette,
) -> egui::Response {
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
            if icon_response
                .on_hover_text("Change icon")
                .clicked()
            {
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
        let plus_rect = ui.allocate_exact_size(Vec2::new(22.0, 22.0), Sense::hover()).0;
        let handle_rect = ui.allocate_exact_size(Vec2::new(22.0, 22.0), Sense::hover()).0;
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
        let edit = |ui: &mut egui::Ui, text: &mut String, font: FontId, w: f32| -> egui::Response {
            ui.add(
                egui::TextEdit::multiline(text)
                    .id(ui.make_persistent_id(("block_edit", &block_id_for_edit, index)))
                    .font(font)
                    .frame(egui::Frame::NONE)
                    .desired_width(w)
                    .desired_rows(1)
                    .interactive(!locked)
                    .text_color(palette.text),
            )
        };

        let mut edit_response: Option<egui::Response> = None;
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
                edit_response = Some(edit(ui, &mut block.text, body_font(view, 15.0), width - 26.0));
            }
            BlockKind::Bulleted => {
                let (rect, _) = ui.allocate_exact_size(Vec2::new(20.0, 22.0), Sense::hover());
                draw_bullet_dot(ui.painter(), rect, palette.accent);
                edit_response = Some(edit(ui, &mut block.text, body_font(view, 15.0), width - 22.0));
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
                edit_response = Some(edit(ui, &mut block.text, body_font(view, 15.0), width - 22.0));
            }
            BlockKind::Quote => {
                let (rect, _) = ui.allocate_exact_size(Vec2::new(6.0, 22.0), Sense::hover());
                ui.painter().vline(
                    rect.center().x,
                    rect.y_range(),
                    Stroke::new(3.0, palette.text),
                );
                ui.add_space(8.0);
                edit_response = Some(edit(ui, &mut block.text, body_font(view, 16.0), width - 16.0));
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
                        edit_response =
                            Some(edit(ui, &mut block.text, FontId::monospace(13.0), width - 24.0));
                    });
            }
            BlockKind::Paragraph
            | BlockKind::Heading1
            | BlockKind::Heading2
            | BlockKind::Heading3 => {
                let font = if block.kind.is_heading() {
                    strong_font(block.kind.font_size())
                } else {
                    body_font(view, 15.0)
                };
                edit_response = Some(edit(ui, &mut block.text, font, width));
            }
        }

        if !locked {
            if let Some(ref resp) = edit_response
                && backspace_on_empty(ui, resp, &block.text)
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

// ---- Library view ---------------------------------------------------------

fn render_library(
    ui: &mut egui::Ui,
    ws: &Workspace,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let full = ui.available_width();
    ui.horizontal_top(|ui| {
        ui.add_space(96.0);
        ui.vertical(|ui| {
            ui.set_width(full - 192.0);
            ui.add_space(40.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Library")
                        .font(strong_font(32.0))
                        .color(palette.text),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("New page")
                                    .size(13.0)
                                    .color(Color32::WHITE),
                            )
                            .fill(palette.accent)
                            .corner_radius(CornerRadius::same(6)),
                        )
                        .clicked()
                    {
                        commands.push(Command::NewPage(None));
                    }
                });
            });
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("Every page in this workspace")
                    .size(13.0)
                    .color(palette.muted),
            );
            ui.add_space(22.0);

            ui.horizontal(|ui| {
                for tab in ["Recents", "Favorites", "Shared", "Private", "More"] {
                    let active = view.library_tab == tab;
                    let btn = ui.add(
                        egui::Button::new(
                            egui::RichText::new(tab)
                                .size(13.0)
                                .color(if active { palette.text } else { palette.muted }),
                        )
                        .fill(if active { palette.selected } else { Color32::TRANSPARENT })
                        .corner_radius(CornerRadius::same(6)),
                    );
                    if btn.clicked() {
                        commands.push(Command::SetLibraryTab(tab.to_string()));
                    }
                }
            });
            ui.add_space(18.0);

            let name_w = (full - 192.0) * 0.42;
            let created_w = (full - 192.0) * 0.24;
            let source_w = (full - 192.0) * 0.20;
            ui.horizontal(|ui| {
                header_cell(ui, name_w, "Page name", palette);
                header_cell(ui, created_w, "Created by", palette);
                header_cell(ui, source_w, "Source", palette);
                header_cell(ui, 100.0, "Last edited", palette);
            });
            ui.separator();

            let pages: Vec<&Page> = match view.library_tab.as_str() {
                "Favorites" => ws.pages.iter().filter(|p| p.favorite).collect(),
                "Private" => ws.pages.iter().filter(|p| p.parent.is_none()).collect(),
                "Shared" => ws.pages.iter().collect(),
                _ => ws.pages.iter().collect(),
            };
            for page in pages {
                let (rect, response) = ui.allocate_exact_size(
                    Vec2::new(full - 192.0, 34.0),
                    Sense::click(),
                );
                if response.hovered() {
                    ui.painter()
                        .rect_filled(rect, CornerRadius::same(4), palette.hover);
                }
                draw_page(
                    ui.painter(),
                    Rect::from_min_size(rect.min + Vec2::new(8.0, 9.0), Vec2::splat(16.0)),
                    palette.muted,
                );
                ui.painter().text(
                    rect.min + Vec2::new(34.0, 17.0),
                    Align2::LEFT_CENTER,
                    &page.title,
                    FontId::proportional(13.0),
                    palette.text,
                );
                ui.painter().text(
                    rect.min + Vec2::new(name_w + 12.0, 17.0),
                    Align2::LEFT_CENTER,
                    "You",
                    FontId::proportional(12.0),
                    palette.muted,
                );
                ui.painter().text(
                    rect.min + Vec2::new(name_w + created_w + 12.0, 17.0),
                    Align2::LEFT_CENTER,
                    &ws.name,
                    FontId::proportional(12.0),
                    palette.muted,
                );
                ui.painter().text(
                    rect.min + Vec2::new(name_w + created_w + source_w + 12.0, 17.0),
                    Align2::LEFT_CENTER,
                    "6d ago",
                    FontId::proportional(12.0),
                    palette.muted,
                );
                ui.painter().hline(
                    rect.x_range(),
                    rect.max.y,
                    Stroke::new(1.0, palette.border),
                );
                if response.clicked() {
                    commands.push(Command::SelectPage(page.id.clone()));
                }
            }
        });
    });
}

// ---- Inline database (used by the Tasks page) -----------------------------

#[allow(clippy::too_many_arguments)]
fn render_database(
    ui: &mut egui::Ui,
    page: &mut Page,
    view: &ViewState,
    width: f32,
    gutter: f32,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    ui.horizontal(|ui| {
        ui.add_space(gutter);
        let table = ui.add(
            egui::Button::new(
                egui::RichText::new("Table").color(if view.table_view {
                    palette.text
                } else {
                    palette.muted
                }),
            )
            .frame(false),
        );
        if table.clicked() {
            commands.push(Command::SetTableView(true));
        }
        let board = ui.add(
            egui::Button::new(
                egui::RichText::new("Board").color(if view.table_view {
                    palette.muted
                } else {
                    palette.text
                }),
            )
            .frame(false),
        );
        if board.clicked() {
            commands.push(Command::SetTableView(false));
        }
        ui.add_space(12.0);
        ui.label(
            egui::RichText::new(format!("{} rows", page.rows.len()))
                .size(12.0)
                .color(palette.muted),
        );
    });
    ui.add_space(8.0);
    ui.separator();

    if view.table_view {
        render_table(ui, page, width, gutter, palette, commands);
    } else {
        render_board(ui, page, width, palette);
    }
}

fn header_cell(ui: &mut egui::Ui, width: f32, text: &str, palette: &Palette) {
    ui.add_sized(
        Vec2::new(width, 22.0),
        egui::Label::new(
            egui::RichText::new(text)
                .size(12.0)
                .strong()
                .color(palette.muted),
        ),
    );
}

fn render_table(
    ui: &mut egui::Ui,
    page: &mut Page,
    width: f32,
    gutter: f32,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let name_w = (width * 0.40).max(170.0);
    let status_w = (width * 0.22).max(110.0);
    let assignee_w = (width * 0.16).max(80.0);
    let due_w = (width * 0.16).max(80.0);

    ui.horizontal(|ui| {
        ui.add_space(gutter);
        header_cell(ui, name_w, "Name", palette);
        header_cell(ui, status_w, "Status", palette);
        header_cell(ui, assignee_w, "Assignee", palette);
        header_cell(ui, due_w, "Due", palette);
    });
    ui.separator();

    for (index, row) in page.rows.iter_mut().enumerate() {
        let row_id = row.id.clone();
        let clicked = egui::Frame::NONE
            .fill(if index % 2 == 1 {
                palette.callout
            } else {
                Color32::TRANSPARENT
            })
            .corner_radius(CornerRadius::same(4))
            .inner_margin(Margin::symmetric(4, 2))
            .show(ui, |ui| {
                let mut status_clicked = false;
                ui.horizontal(|ui| {
                    ui.add_space(gutter);
                    ui.add_sized(
                        Vec2::new(name_w, 24.0),
                        egui::TextEdit::singleline(&mut row.name)
                            .frame(egui::Frame::NONE)
                            .text_color(palette.text),
                    );
                    let (rect, response) = ui.allocate_exact_size(
                        Vec2::new(status_w - 12.0, 22.0),
                        Sense::click(),
                    );
                    let chip = Rect::from_min_size(
                        rect.min,
                        Vec2::new((rect.width() - 6.0).max(40.0), 20.0),
                    );
                    ui.painter()
                        .rect_filled(chip, CornerRadius::same(4), status_color(&row.status));
                    ui.painter().text(
                        chip.min + Vec2::new(8.0, 10.0),
                        Align2::LEFT_CENTER,
                        &row.status,
                        FontId::proportional(12.0),
                        palette.chip_text,
                    );
                    status_clicked = response.clicked();
                    ui.add_sized(
                        Vec2::new(assignee_w, 24.0),
                        egui::TextEdit::singleline(&mut row.assignee)
                            .frame(egui::Frame::NONE)
                            .text_color(palette.text),
                    );
                    ui.add_sized(
                        Vec2::new(due_w, 24.0),
                        egui::TextEdit::singleline(&mut row.due)
                            .frame(egui::Frame::NONE)
                            .text_color(palette.text),
                    );
                });
                status_clicked
            })
            .inner;
        if clicked {
            commands.push(Command::CycleStatus(row_id));
        }
    }
    ui.add_space(6.0);
    if ui.add(egui::Button::new("+ New row").frame(false)).clicked() {
        commands.push(Command::AddRow);
    }
}

fn render_board(ui: &mut egui::Ui, page: &mut Page, width: f32, palette: &Palette) {
    let statuses = ["Not started", "In progress", "Done"];
    let column_width = (width - 24.0) / 3.0;
    ui.columns(3, |columns| {
        for (index, status) in statuses.iter().enumerate() {
            let col = &mut columns[index];
            col.set_width(column_width);
            col.label(
                egui::RichText::new(*status)
                    .size(12.0)
                    .strong()
                    .color(palette.muted),
            );
            col.add_space(6.0);
            for row in page.rows.iter().filter(|r| &r.status == status) {
                egui::Frame::NONE
                    .fill(palette.callout)
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(Margin::same(10))
                    .show(col, |ui| {
                        ui.set_width(column_width - 30.0);
                        ui.label(egui::RichText::new(&row.name).color(palette.text));
                        ui.label(
                            egui::RichText::new(format!("{}  {}", row.assignee, row.due))
                                .size(11.0)
                                .color(palette.muted),
                        );
                    });
                col.add_space(6.0);
            }
            col.add_space(6.0);
        }
    });
}
