// ---- Library view ---------------------------------------------------------

//! Library table: tabs, page rows and the empty state.

use crate::model::{Page, Workspace};
use egui::{Align2, Color32, CornerRadius, FontId, Rect, Sense, Stroke, Vec2};

use super::super::commands::Command;
use super::super::icons::draw_page;
use super::super::palette::{Palette, strong_font};
use super::super::view::ViewState;
use super::database::header_cell;

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
                for tab in ["Recents", "Favorites", "Shared", "Private", "More"] {
                    let active = view.library_tab == tab;
                    let btn = ui.add(
                        egui::Button::new(egui::RichText::new(tab).size(13.0).color(if active {
                            palette.text
                        } else {
                            palette.muted
                        }))
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
                ui.painter()
                    .hline(rect.x_range(), rect.max.y, Stroke::new(1.0, palette.border));
                if response.clicked() {
                    commands.push(Command::SelectPage(page.id.clone()));
                }
            }
        });
    });
}
