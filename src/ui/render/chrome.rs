//! Custom window chrome: the compact tab strip and window controls.
//!
//! The left container is exactly as wide as the sidebar, so the tab strip and
//! back/forward controls start where the content pane starts. The app menu,
//! sidebar toggle, notifications and compose action live inside that container
//! (spec-059).

use crate::model::Workspace;
use egui::{Align2, CornerRadius, FontId, Pos2, Rect, Sense, Vec2};

use super::super::commands::Command;
use super::super::icons::{
    draw_arrow_left, draw_arrow_right, draw_close, draw_inbox, draw_menu, draw_minus,
    draw_page_icon, draw_panel, draw_pen, draw_plus, draw_square, glyph_button, page_icon_letter,
};
use super::super::palette::{Palette, tab_title};
use super::super::view::ViewState;

/// Height of the compact, Notion-like tab strip (spec-013). The breadcrumb row
/// sits directly below it, so the whole top chrome stays small.
pub(crate) const TITLE_BAR_HEIGHT: f32 = 36.0;

/// The open sidebar width and the compact app-bar width when it is closed.
pub(crate) const SIDEBAR_WIDTH: f32 = 266.0;
pub(crate) const APP_BAR_COMPACT_WIDTH: f32 = 92.0;

/// The animated app-bar container width: it follows the sidebar while open and
/// never drops below the compact width that holds the menu and toggle.
pub fn app_bar_left_width(sidebar_width: f32, animation: f32) -> f32 {
    (sidebar_width * animation.clamp(0.0, 1.0)).max(APP_BAR_COMPACT_WIDTH)
}

pub(crate) fn render_title_bar(
    ui: &mut egui::Ui,
    ws: &Workspace,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
    left_width: f32,
) {
    let h = TITLE_BAR_HEIGHT;
    let left = left_width.max(APP_BAR_COMPACT_WIDTH);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;

        // Sidebar-width container: app menu, toggle, then notifications and
        // compose pinned to its right edge.
        ui.allocate_ui_with_layout(
            Vec2::new(left, h),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_min_width(left);
                ui.add_space(6.0);
                let menu = glyph_button(ui, Vec2::new(26.0, h - 6.0), palette, draw_menu)
                    .on_hover_text("Menu");
                if menu.clicked() {
                    commands.push(Command::OpenAppMenu);
                }
                let panel = glyph_button(ui, Vec2::new(26.0, h - 6.0), palette, draw_panel)
                    .on_hover_text("Show / hide sidebar (Ctrl+\\)");
                if panel.clicked() {
                    commands.push(Command::ToggleSidebar);
                }
                let controls = 26.0 + 26.0 + 6.0;
                let show_extras = left >= 150.0;
                let spacer = if show_extras {
                    (left - 6.0 - 26.0 - 26.0 - controls - 8.0).max(0.0)
                } else {
                    0.0
                };
                ui.add_space(spacer);
                if show_extras {
                    let inbox = glyph_button(ui, Vec2::new(26.0, h - 6.0), palette, draw_inbox)
                        .on_hover_text("Inbox");
                    if view.inbox_unread > 0 {
                        let badge = Rect::from_center_size(
                            Pos2::new(inbox.rect.right() - 5.0, inbox.rect.top() + 5.0),
                            Vec2::splat(12.0),
                        );
                        ui.painter().circle_filled(
                            badge.center(),
                            6.0,
                            egui::Color32::from_rgb(224, 108, 108),
                        );
                        ui.painter().text(
                            badge.center(),
                            Align2::CENTER_CENTER,
                            view.inbox_unread.min(9).to_string(),
                            FontId::proportional(8.5),
                            egui::Color32::WHITE,
                        );
                    }
                    if inbox.clicked() {
                        commands.push(Command::OpenInbox);
                    }
                    let compose = glyph_button(ui, Vec2::new(26.0, h - 6.0), palette, draw_pen)
                        .on_hover_text("New page, chat, meeting note or database");
                    if compose.clicked() {
                        commands.push(Command::ToggleComposeMenu);
                    }
                }
            },
        );

        // Outside the sidebar container: back/forward, the tab strip and "+".
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
            draw_page_icon(
                ui.painter(),
                icon_rect,
                &page_icon_letter(&icon, &label),
                palette,
                false,
            );
            ui.painter().text(
                tab_rect.min + Vec2::new(34.0, (h - 8.0) / 2.0),
                Align2::LEFT_CENTER,
                &label,
                FontId::proportional(13.0),
                palette.text,
            );
            let close_rect = Rect::from_center_size(
                Pos2::new(tab_rect.right() - 14.0, tab_rect.center().y),
                Vec2::splat(18.0),
            );
            let close = ui.interact(
                close_rect,
                ui.scope_id().with(("close_tab", i)),
                Sense::click(),
            );
            if close.hovered() {
                ui.painter()
                    .rect_filled(close_rect, CornerRadius::same(4), palette.selected);
            }
            ui.painter().text(
                close_rect.center(),
                Align2::CENTER_CENTER,
                "×",
                FontId::proportional(13.0),
                if close.hovered() {
                    palette.text
                } else {
                    palette.muted
                },
            );
            if close.clicked() {
                commands.push(Command::CloseTab(i));
            } else if response.clicked() && !active {
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
