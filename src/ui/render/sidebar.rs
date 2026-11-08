//! Sidebar: search box, Library/Settings/Inbox rows, favorites, page tree and
//! the fixed bottom account / invite / customize group.

use crate::i18n;
use crate::model::Workspace;
use egui::{Align2, Color32, CornerRadius, FontId, Rect, Sense, Stroke, Vec2};

use super::super::commands::Command;
use super::super::icons::{
    draw_chevron, draw_grid, draw_help, draw_inbox, draw_library, draw_page_icon, draw_person,
    draw_plus, draw_search, draw_settings, draw_sliders,
};
use super::super::palette::Palette;
use super::super::view::ViewState;

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
        ui.painter().rect_filled(rect, CornerRadius::same(6), fill);
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

// ---- Sidebar --------------------------------------------------------------

pub(crate) fn render_sidebar(
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
        i18n::t("search_or_ask"),
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
            if nav_row(
                ui,
                width,
                &i18n::t("library"),
                view.library_open,
                palette,
                draw_library,
            )
            .clicked()
            {
                commands.push(Command::OpenLibrary);
            }
            if nav_row(
                ui,
                width,
                &i18n::t("settings"),
                false,
                palette,
                draw_settings,
            )
            .clicked()
            {
                commands.push(Command::OpenSettings);
            }
            if nav_row(ui, width, &i18n::t("inbox"), false, palette, draw_inbox).clicked() {
                commands.push(Command::OpenInbox);
            }

            ui.add_space(18.0);
            section_label(ui, &i18n::t("favorites"), palette);
            let favorites = ws.favorites();
            if favorites.is_empty() {
                section_label(ui, &i18n::t("no_favorites"), palette);
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
            section_label(ui, &i18n::t("private"), palette);
            render_tree(ui, ws, None, 0, width, view, palette, commands);
            if nav_row(ui, width, &i18n::t("new_page"), false, palette, draw_plus).clicked() {
                commands.push(Command::NewPage(None));
            }
        });

    // Bottom group: anchored to the bottom of the sidebar.
    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
        // Account row + workspace menu + grid menu.
        let (row, row_resp) = ui.allocate_exact_size(Vec2::new(width, 40.0), Sense::click());
        if row_resp.hovered() {
            ui.painter()
                .rect_filled(row, CornerRadius::same(6), palette.hover);
        }
        ui.painter()
            .circle_filled(row.min + Vec2::new(22.0, 20.0), 13.0, palette.accent);
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
            i18n::t("local_workspace"),
            FontId::proportional(13.0),
            palette.text,
        );
        draw_chevron(
            ui.painter(),
            Rect::from_min_size(row.min + Vec2::new(164.0, 12.0), Vec2::splat(16.0)),
            view.workspace_menu_open,
            palette.muted,
        );
        let help_rect =
            Rect::from_min_size(row.min + Vec2::new(width - 46.0, 12.0), Vec2::splat(16.0));
        let grid_rect =
            Rect::from_min_size(row.min + Vec2::new(width - 26.0, 12.0), Vec2::splat(16.0));
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
            i18n::t("invite_members"),
            FontId::proportional(13.0),
            palette.accent,
        );
        ui.painter().text(
            card.min + Vec2::new(12.0, 42.0),
            Align2::LEFT_CENTER,
            i18n::t("collaborate_team"),
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
            ui.painter()
                .rect_filled(custom, CornerRadius::same(6), palette.hover);
        }
        draw_sliders(
            ui.painter(),
            Rect::from_min_size(custom.min + Vec2::new(10.0, 6.0), Vec2::splat(16.0)),
            palette.muted,
        );
        ui.painter().text(
            custom.min + Vec2::new(36.0, 14.0),
            Align2::LEFT_CENTER,
            i18n::t("customize_sidebar"),
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
        if page.archived {
            continue;
        }
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
            ui.painter().rect_filled(rect, CornerRadius::same(6), fill);
        }
        let glyph_rect = Rect::from_min_size(rect.min + Vec2::new(indent, 6.0), Vec2::splat(16.0));
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
