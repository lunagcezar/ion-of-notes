//! Small shared UI widgets used by the floating windows and menus.
//!
//! These are presentation primitives only: they paint and return a response,
//! never mutate app state. Keeping them here avoids duplicating the same row
//! and popover frames across the dialog modules (DRY, spec-017).

use egui::{Align2, Color32, CornerRadius, FontId, Margin, Pos2, Rect, Sense, Stroke, Vec2};

use super::palette::Palette;

/// A compact window frame shared by the popovers.
pub(super) fn popover_frame(palette: &Palette) -> egui::Frame {
    egui::Frame::NONE
        .fill(palette.content)
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(14))
        .stroke(Stroke::new(1.0, palette.border))
}

/// A small muted section label used by the dialog windows.
pub(super) fn section_label(ui: &mut egui::Ui, text: &str, palette: &Palette) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 24.0), Sense::hover());
    ui.painter().text(
        rect.min + Vec2::new(12.0, 14.0),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(11.0),
        palette.muted,
    );
}

/// A clickable row with a label and an optional shortcut hint.
pub(super) fn menu_button(
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
pub(super) fn toggle_button(
    ui: &mut egui::Ui,
    label: &str,
    on: bool,
    palette: &Palette,
) -> egui::Response {
    let response = menu_button(ui, label, "", palette);
    let rect = response.rect;
    let pill = Rect::from_min_size(rect.max + Vec2::new(-40.0, -21.5), Vec2::new(30.0, 16.0));
    ui.painter().rect_filled(
        pill,
        CornerRadius::same(8),
        if on { palette.accent } else { palette.hover },
    );
    let knob_x = if on {
        pill.max.x - 9.0
    } else {
        pill.min.x + 9.0
    };
    ui.painter()
        .circle_filled(Pos2::new(knob_x, pill.center().y), 6.0, Color32::WHITE);
    response
}
