//! Small shared UI widgets used by the floating windows and menus.
//!
//! These are presentation primitives only: they paint and return a response,
//! never mutate app state. Keeping them here avoids duplicating the same row
//! and popover frames across the dialog modules (DRY, spec-017).

use egui::{Align2, Color32, CornerRadius, FontId, Margin, Pos2, Rect, Sense, Stroke, Vec2};

use super::palette::{Palette, strong_font};
use crate::ui::constants::{
    DIALOG_RADIUS, FONT_CAPTION, FONT_SUBTITLE, MENU_ROW_HEIGHT, MENU_ROW_RADIUS, MENU_TEXT_SIZE,
    POPOVER_PADDING, POPOVER_RADIUS, SECTION_LABEL_HEIGHT, SPACE_6, SPACE_8, SPACE_10, SPACE_12,
    SPACE_14, SPACE_16, SPACE_20, SPACE_22, SPACE_30, SPACE_40, STROKE_HAIRLINE,
};

/// A section heading inside a settings page. Rows sharing a section are
/// grouped under one heading, so filtering never leaves an empty heading.
pub(super) fn section_heading(ui: &mut egui::Ui, title: &str, palette: &Palette) {
    ui.add_space(SPACE_20);
    ui.label(
        egui::RichText::new(title)
            .font(strong_font(FONT_SUBTITLE))
            .color(palette.text),
    );
    ui.add_space(SPACE_10);
    ui.separator();
    ui.add_space(SPACE_10);
}

/// A compact label/value row used by the workspace summary.
pub(super) fn info_row(ui: &mut egui::Ui, label: &str, value: &str, palette: &Palette) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(label)
                .size(FONT_CAPTION)
                .strong()
                .color(palette.text),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(value)
                    .size(FONT_CAPTION)
                    .color(palette.muted),
            );
        });
    });
    ui.add_space(SPACE_6);
}

/// A compact window frame shared by the popovers.
pub(super) fn popover_frame(palette: &Palette) -> egui::Frame {
    egui::Frame::NONE
        .fill(palette.content)
        .corner_radius(CornerRadius::same(DIALOG_RADIUS))
        .inner_margin(Margin::same(SPACE_14 as i8))
        .stroke(Stroke::new(STROKE_HAIRLINE, palette.border))
}

/// A compact window frame shared by the context popovers.
pub(super) fn popover_frame_compact(palette: &Palette) -> egui::Frame {
    egui::Frame::NONE
        .fill(palette.content)
        .corner_radius(CornerRadius::same(POPOVER_RADIUS))
        .inner_margin(Margin::same(POPOVER_PADDING as i8))
        .stroke(Stroke::new(STROKE_HAIRLINE, palette.border))
}

/// A small muted section label used by the dialog windows.
pub(super) fn section_label(ui: &mut egui::Ui, text: &str, palette: &Palette) {
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), SECTION_LABEL_HEIGHT),
        Sense::hover(),
    );
    ui.painter().text(
        rect.min + Vec2::new(SPACE_12, SPACE_14),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(FONT_CAPTION),
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
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), MENU_ROW_HEIGHT),
        Sense::click(),
    );
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(MENU_ROW_RADIUS), palette.hover);
    }
    ui.painter().text(
        rect.min + Vec2::new(SPACE_10, MENU_ROW_HEIGHT / 2.0),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(MENU_TEXT_SIZE),
        palette.text,
    );
    if !hint.is_empty() {
        ui.painter().text(
            rect.max + Vec2::new(-SPACE_8, -MENU_ROW_HEIGHT / 2.0),
            Align2::RIGHT_CENTER,
            hint,
            FontId::proportional(FONT_CAPTION),
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
    let pill = Rect::from_min_size(
        rect.max + Vec2::new(-SPACE_40, -SPACE_22),
        Vec2::new(SPACE_30, SPACE_16),
    );
    ui.painter().rect_filled(
        pill,
        CornerRadius::same(POPOVER_RADIUS),
        if on { palette.accent } else { palette.hover },
    );
    let knob_x = if on {
        pill.max.x - SPACE_10
    } else {
        pill.min.x + SPACE_10
    };
    ui.painter()
        .circle_filled(Pos2::new(knob_x, pill.center().y), SPACE_6, Color32::WHITE);
    response
}
