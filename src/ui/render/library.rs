// ---- Library view ---------------------------------------------------------

//! Library table: tabs, page rows and the empty state.

use crate::i18n;
use crate::model::Workspace;
use egui::{Align2, Color32, CornerRadius, FontId, Rect, Sense, Stroke, Vec2};

use super::super::commands::Command;
use super::super::format::format_timestamp;
use super::super::icons::draw_page;
use super::super::library_filter::pages_for_view;
use super::super::palette::{Palette, strong_font};
use super::super::view::ViewState;
use super::database::header_cell;

fn tab_label(tab: &str) -> String {
    match tab {
        "Recents" => i18n::t("recents"),
        "Favorites" => i18n::t("favorites"),
        "Shared" => i18n::t("shared"),
        "Private" => i18n::t("private"),
        "AI Meeting Notes" => i18n::t("ai_meeting_notes"),
        "Agents" => i18n::t("agents"),
        "Templates" => i18n::t("templates"),
        "Archive" => i18n::t("archive_view"),
        "Trash" => i18n::t("trash"),
        _ => tab.to_string(),
    }
}

pub(super) fn render_library(
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
                for tab in ["Recents", "Favorites", "Shared", "Private"] {
                    let active = view.library_tab == tab;
                    let btn = ui.add(
                        egui::Button::new(
                            egui::RichText::new(tab_label(tab))
                                .size(13.0)
                                .color(if active { palette.text } else { palette.muted }),
                        )
                        .fill(if active {
                            palette.selected
                        } else {
                            Color32::TRANSPARENT
                        })
                        .corner_radius(CornerRadius::same(6)),
                    );
                    if btn.clicked() {
                        commands.push(Command::SetLibraryTab(tab.to_string()));
                    }
                }
                ui.menu_button(
                    egui::RichText::new(i18n::t("more"))
                        .size(13.0)
                        .color(palette.muted),
                    |ui| {
                        for tab in [
                            "AI Meeting Notes",
                            "Agents",
                            "Templates",
                            "Archive",
                            "Trash",
                        ] {
                            if ui.button(tab_label(tab)).clicked() {
                                if tab == "Templates" {
                                    commands.push(Command::OpenTemplates);
                                } else {
                                    commands.push(Command::SetLibraryTab(tab.to_string()));
                                }
                                ui.close();
                            }
                        }
                    },
                );
            });
            ui.add_space(18.0);

            let name_w = (full - 192.0) * 0.42;
            let created_w = (full - 192.0) * 0.24;
            let source_w = (full - 192.0) * 0.20;
            ui.horizontal(|ui| {
                header_cell(ui, name_w, &i18n::t("page_name"), palette);
                header_cell(ui, created_w, &i18n::t("created_by"), palette);
                header_cell(ui, source_w, &i18n::t("source"), palette);
                header_cell(ui, 120.0, &i18n::t("last_edited_time"), palette);
            });
            ui.separator();

            let pages = pages_for_view(&ws.pages, &view.library_tab, &view.library_shared);
            if pages.is_empty() {
                ui.add_space(20.0);
                ui.label(
                    egui::RichText::new(i18n::t("no_pages"))
                        .size(13.0)
                        .color(palette.muted),
                );
            }
            for page in pages {
                let (rect, response) =
                    ui.allocate_exact_size(Vec2::new(full - 192.0, 34.0), Sense::click());
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
                    &view.library_owner,
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
                    format_timestamp(&page.updated_at, view.time_format),
                    FontId::proportional(12.0),
                    palette.muted,
                );
                ui.painter()
                    .hline(rect.x_range(), rect.max.y, Stroke::new(1.0, palette.border));
                if response.clicked() {
                    commands.push(Command::SelectPage(page.id.clone()));
                }
            }
        });
    });
}
