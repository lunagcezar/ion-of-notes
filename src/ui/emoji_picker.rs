//! Shared searchable emoji grid, used by the page-icon picker and inline
//! reactions (DRY, spec-158).

use egui::{Align2, CornerRadius, FontId, Sense, Vec2};

use super::constants::{ICON_PICKER_COLUMNS, ICON_PICKER_ROW_HEIGHT};
use super::icon_catalog::{EMOJI, catalog_matches_normalized};
use super::palette::Palette;

/// Paint a scrollable emoji grid and return the glyph the user clicked.
pub(super) fn emoji_grid(
    ui: &mut egui::Ui,
    query: &str,
    selected: Option<&str>,
    palette: &Palette,
    max_height: f32,
) -> Option<String> {
    let normalized = query.trim().to_lowercase();
    let indices: Vec<usize> = EMOJI
        .iter()
        .enumerate()
        .filter(|(_, entry)| catalog_matches_normalized(entry.name, entry.keywords, &normalized))
        .map(|(index, _)| index)
        .collect();
    if indices.is_empty() {
        return None;
    }
    let rows = indices.len().div_ceil(ICON_PICKER_COLUMNS);
    let mut picked = None;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .max_height(max_height)
        .show_rows(ui, ICON_PICKER_ROW_HEIGHT, rows, |ui, range| {
            for row in range {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::splat(2.0);
                    for column in 0..ICON_PICKER_COLUMNS {
                        let slot = row * ICON_PICKER_COLUMNS + column;
                        let Some(&index) = indices.get(slot) else {
                            break;
                        };
                        let entry = &EMOJI[index];
                        let is_selected = selected == Some(entry.glyph);
                        let (rect, response) =
                            ui.allocate_exact_size(Vec2::new(34.0, 30.0), Sense::click());
                        if is_selected {
                            ui.painter()
                                .rect_filled(rect, CornerRadius::same(5), palette.selected);
                        } else if response.hovered() {
                            ui.painter()
                                .rect_filled(rect, CornerRadius::same(5), palette.hover);
                        }
                        ui.painter().text(
                            rect.center(),
                            Align2::CENTER_CENTER,
                            entry.glyph,
                            FontId::proportional(19.0),
                            palette.text,
                        );
                        if response.clicked() {
                            picked = Some(entry.glyph.to_string());
                        }
                        response.on_hover_text(entry.name);
                    }
                });
            }
        });
    picked
}
