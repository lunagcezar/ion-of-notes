//! Custom window chrome: the compact tab strip and window controls.
//!
//! The left container is exactly as wide as the sidebar, so the tab strip and
//! back/forward controls start where the content pane starts. The app menu,
//! sidebar toggle, notifications and compose action live inside that container
//! (spec-059).

use crate::model::Workspace;
use egui::{Align2, CornerRadius, FontId, Pos2, Rect, Sense, Vec2};

use crate::ui::constants::{
    BADGE_OFFSET, BADGE_RADIUS, BADGE_SIZE, FONT_MICRO, FONT_ROW, ICON_RADIUS, ICON_SIZE,
    SHOW_EXTRAS_MIN_WIDTH, SPACE_2, SPACE_6, SPACE_8, SPACE_12, TAB_CLOSE_INSET, TAB_CLOSE_SIZE,
    TAB_GAP, TAB_HEIGHT, TAB_ICON_INSET, TAB_ICON_Y, TAB_LABEL_INSET, TAB_RADIUS, TAB_WIDTH,
    TITLE_BUTTON_SIZE, WINDOW_CONTROL_WIDTH,
};

use super::super::commands::{Command, SidebarCmd};
use super::super::icons::{
    draw_arrow_left, draw_arrow_right, draw_close, draw_inbox, draw_menu, draw_minus,
    draw_page_icon, draw_panel, draw_pen, draw_plus, draw_square, glyph_button, page_icon_letter,
};
use super::super::palette::{Palette, tab_title};
use super::super::view::ViewState;

pub(crate) use crate::ui::constants::{APP_BAR_COMPACT_WIDTH, SIDEBAR_WIDTH, TITLE_BAR_HEIGHT};

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
        ui.spacing_mut().item_spacing.x = SPACE_2;

        // Sidebar-width container: app menu, toggle, then notifications and
        // compose pinned to its right edge.
        ui.allocate_ui_with_layout(
            Vec2::new(left, h),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_min_width(left);
                ui.add_space(SPACE_6);
                let menu = glyph_button(
                    ui,
                    Vec2::new(TITLE_BUTTON_SIZE, h - SPACE_6),
                    palette,
                    draw_menu,
                )
                .on_hover_text("Menu");
                if menu.clicked() {
                    commands.push(Command::OpenAppMenu);
                }
                let panel = glyph_button(
                    ui,
                    Vec2::new(TITLE_BUTTON_SIZE, h - SPACE_6),
                    palette,
                    draw_panel,
                )
                .on_hover_text("Show / hide sidebar (Ctrl+\\)");
                if panel.clicked() {
                    commands.push(Command::Sidebar(SidebarCmd::Toggle));
                }
                let controls = TITLE_BUTTON_SIZE + TITLE_BUTTON_SIZE + SPACE_6;
                let show_extras = left >= SHOW_EXTRAS_MIN_WIDTH;
                let spacer = if show_extras {
                    (left - SPACE_6 - TITLE_BUTTON_SIZE - TITLE_BUTTON_SIZE - controls - SPACE_8)
                        .max(0.0)
                } else {
                    0.0
                };
                ui.add_space(spacer);
                if show_extras {
                    let inbox = glyph_button(
                        ui,
                        Vec2::new(TITLE_BUTTON_SIZE, h - SPACE_6),
                        palette,
                        draw_inbox,
                    )
                    .on_hover_text("Inbox");
                    if view.inbox_unread > 0 {
                        let badge = Rect::from_center_size(
                            Pos2::new(
                                inbox.rect.right() - BADGE_OFFSET,
                                inbox.rect.top() + BADGE_OFFSET,
                            ),
                            Vec2::splat(BADGE_SIZE),
                        );
                        ui.painter().circle_filled(
                            badge.center(),
                            BADGE_RADIUS,
                            egui::Color32::from_rgb(224, 108, 108),
                        );
                        ui.painter().text(
                            badge.center(),
                            Align2::CENTER_CENTER,
                            view.inbox_unread.min(9).to_string(),
                            FontId::proportional(FONT_MICRO),
                            egui::Color32::WHITE,
                        );
                    }
                    if inbox.clicked() {
                        commands.push(Command::OpenInbox);
                    }
                    let compose = glyph_button(
                        ui,
                        Vec2::new(TITLE_BUTTON_SIZE, h - SPACE_6),
                        palette,
                        draw_pen,
                    )
                    .on_hover_text("New page, chat, meeting note or database");
                    if compose.clicked() {
                        commands.push(Command::ToggleComposeMenu);
                    }
                }
            },
        );

        // Outside the sidebar container: back/forward, the tab strip and "+".
        if glyph_button(
            ui,
            Vec2::new(TITLE_BUTTON_SIZE, h),
            palette,
            draw_arrow_left,
        )
        .clicked()
        {
            commands.push(Command::GoBack);
        }
        if glyph_button(
            ui,
            Vec2::new(TITLE_BUTTON_SIZE, h),
            palette,
            draw_arrow_right,
        )
        .clicked()
        {
            commands.push(Command::GoForward);
        }
        ui.add_space(SPACE_12);

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
                ui.allocate_exact_size(Vec2::new(TAB_WIDTH, TAB_HEIGHT), Sense::click());
            if active || response.hovered() {
                ui.painter()
                    .rect_filled(tab_rect, CornerRadius::same(TAB_RADIUS), palette.hover);
            }
            let icon_rect = Rect::from_min_size(
                tab_rect.min + Vec2::new(TAB_ICON_INSET, TAB_ICON_Y),
                Vec2::splat(ICON_SIZE),
            );
            draw_page_icon(
                ui.painter(),
                icon_rect,
                &page_icon_letter(&icon, &label),
                palette,
                false,
            );
            ui.painter().text(
                tab_rect.min + Vec2::new(TAB_LABEL_INSET, TAB_HEIGHT / 2.0),
                Align2::LEFT_CENTER,
                &label,
                FontId::proportional(FONT_ROW),
                palette.text,
            );
            let close_rect = Rect::from_center_size(
                Pos2::new(tab_rect.right() - TAB_CLOSE_INSET, tab_rect.center().y),
                Vec2::splat(TAB_CLOSE_SIZE),
            );
            let close = ui.interact(
                close_rect,
                ui.scope_id().with(("close_tab", i)),
                Sense::click(),
            );
            if close.hovered() {
                ui.painter().rect_filled(
                    close_rect,
                    CornerRadius::same(ICON_RADIUS),
                    palette.selected,
                );
            }
            ui.painter().text(
                close_rect.center(),
                Align2::CENTER_CENTER,
                "×",
                FontId::proportional(FONT_ROW),
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
            ui.add_space(TAB_GAP);
        }
        // The "+" opens the "Open in new tab…" switcher, not a blank page.
        if glyph_button(ui, Vec2::new(TITLE_BUTTON_SIZE, h), palette, draw_plus)
            .on_hover_text("Open in new tab")
            .clicked()
        {
            commands.push(Command::OpenNewTab);
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_space(SPACE_6);
            if glyph_button(ui, Vec2::new(WINDOW_CONTROL_WIDTH, h), palette, draw_close).clicked() {
                commands.push(Command::WindowClose);
            }
            if glyph_button(ui, Vec2::new(WINDOW_CONTROL_WIDTH, h), palette, draw_square).clicked()
            {
                commands.push(Command::WindowMaximize);
            }
            if glyph_button(ui, Vec2::new(WINDOW_CONTROL_WIDTH, h), palette, draw_minus).clicked() {
                commands.push(Command::WindowMinimize);
            }
        });
    });
}
