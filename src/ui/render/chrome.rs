//! Custom window chrome: the compact tab strip and window controls.

use crate::model::Workspace;
use egui::{Align2, CornerRadius, FontId, Pos2, Rect, Sense, Vec2};

use super::super::commands::Command;
use super::super::icons::{
    draw_arrow_left, draw_arrow_right, draw_close, draw_menu, draw_minus, draw_page_icon,
    draw_panel, draw_plus, draw_square, glyph_button,
};
use super::super::palette::{Palette, tab_title};
use super::super::view::ViewState;

/// Height of the compact, Notion-like tab strip (spec-013). The breadcrumb row
/// sits directly below it, so the whole top chrome stays small.
pub(crate) const TITLE_BAR_HEIGHT: f32 = 36.0;

pub(crate) fn render_title_bar(
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
        let menu =
            glyph_button(ui, Vec2::new(26.0, h - 6.0), palette, draw_menu).on_hover_text("Menu");
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
