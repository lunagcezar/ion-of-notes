//! Small clickable icons drawn with simple shapes (SRP: painting primitives).
//! All artwork here is original shape work; no asset is copied from anywhere.

use egui::{Color32, CornerRadius, Pos2, Rect, Sense, Stroke, Vec2};

use super::palette::Palette;

/// Paint a small icon button with the requested interaction sense. Shared by
/// the clickable and the draggable variants (DRY).
fn icon_button_with_sense(
    ui: &mut egui::Ui,
    size: f32,
    palette: &Palette,
    sense: Sense,
    draw: impl FnOnce(&egui::Painter, Rect, Color32),
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), sense);
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(4), palette.hover);
    }
    let color = if response.hovered() {
        palette.text
    } else {
        palette.muted
    };
    draw(ui.painter(), rect, color);
    response
}

/// A small clickable icon drawn with simple shapes (no copied assets).
pub(super) fn icon_button(
    ui: &mut egui::Ui,
    size: f32,
    palette: &Palette,
    draw: impl FnOnce(&egui::Painter, Rect, Color32),
) -> egui::Response {
    icon_button_with_sense(ui, size, palette, Sense::click(), draw)
}

#[allow(dead_code)]
/// A small icon that is both clickable and draggable (the block reorder grip).
pub(super) fn drag_icon_button(
    ui: &mut egui::Ui,
    size: f32,
    palette: &Palette,
    draw: impl FnOnce(&egui::Painter, Rect, Color32),
) -> egui::Response {
    icon_button_with_sense(ui, size, palette, Sense::click_and_drag(), draw)
}

/// A glyph-only clickable area, used by the tab strip and breadcrumb bar.
pub(super) fn glyph_button(
    ui: &mut egui::Ui,
    size: Vec2,
    palette: &Palette,
    draw: impl FnOnce(&egui::Painter, Rect, Color32),
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(4), palette.hover);
    }
    let color = if response.hovered() {
        palette.text
    } else {
        palette.muted
    };
    draw(ui.painter(), rect, color);
    response
}

pub(super) fn draw_plus(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let r = 5.0;
    let stroke = Stroke::new(1.4, color);
    painter.line(vec![c + Vec2::new(-r, 0.0), c + Vec2::new(r, 0.0)], stroke);
    painter.line(vec![c + Vec2::new(0.0, -r), c + Vec2::new(0.0, r)], stroke);
}

pub(super) fn draw_dots(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    for dx in [-3.5, 3.5] {
        for dy in [-3.5, 0.0, 3.5] {
            painter.circle_filled(c + Vec2::new(dx, dy), 1.1, color);
        }
    }
}

pub(super) fn draw_ellipsis(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    for dx in [-5.0, 0.0, 5.0] {
        painter.circle_filled(c + Vec2::new(dx, 0.0), 1.4, color);
    }
}

pub(super) fn draw_chevron(painter: &egui::Painter, rect: Rect, open: bool, color: Color32) {
    let c = rect.center();
    let stroke = Stroke::new(1.3, color);
    if open {
        painter.line(
            vec![
                c + Vec2::new(-3.0, -1.5),
                c + Vec2::new(0.0, 2.0),
                c + Vec2::new(3.0, -1.5),
            ],
            stroke,
        );
    } else {
        painter.line(
            vec![
                c + Vec2::new(-1.5, -3.0),
                c + Vec2::new(2.0, 0.0),
                c + Vec2::new(-1.5, 3.0),
            ],
            stroke,
        );
    }
}

pub(super) fn draw_search(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center() + Vec2::new(-1.5, -1.5);
    let stroke = Stroke::new(1.3, color);
    painter.circle_stroke(c, 4.5, stroke);
    painter.line(
        vec![c + Vec2::new(3.2, 3.2), c + Vec2::new(6.5, 6.5)],
        stroke,
    );
}

pub(super) fn draw_inbox(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let stroke = Stroke::new(1.3, color);
    painter.rect_stroke(
        Rect::from_min_size(c + Vec2::new(-6.0, -5.0), Vec2::new(12.0, 10.0)),
        CornerRadius::same(2),
        stroke,
        egui::StrokeKind::Inside,
    );
    painter.line(
        vec![c + Vec2::new(-6.0, 1.0), c + Vec2::new(6.0, 1.0)],
        stroke,
    );
}

pub(super) fn draw_page(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let stroke = Stroke::new(1.3, color);
    painter.rect_stroke(
        Rect::from_min_size(c + Vec2::new(-5.0, -6.0), Vec2::new(10.0, 12.0)),
        CornerRadius::same(1),
        stroke,
        egui::StrokeKind::Inside,
    );
    painter.line(
        vec![c + Vec2::new(-2.5, -2.0), c + Vec2::new(2.5, -2.0)],
        stroke,
    );
    painter.line(
        vec![c + Vec2::new(-2.5, 1.5), c + Vec2::new(2.5, 1.5)],
        stroke,
    );
}

pub(super) fn draw_star(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let stroke = Stroke::new(1.3, color);
    painter.line(
        vec![
            c + Vec2::new(0.0, -6.0),
            c + Vec2::new(1.8, -1.8),
            c + Vec2::new(6.0, -1.8),
            c + Vec2::new(2.6, 1.2),
            c + Vec2::new(3.8, 6.0),
            c + Vec2::new(0.0, 3.0),
            c + Vec2::new(-3.8, 6.0),
            c + Vec2::new(-2.6, 1.2),
            c + Vec2::new(-6.0, -1.8),
            c + Vec2::new(-1.8, -1.8),
            c + Vec2::new(0.0, -6.0),
        ],
        stroke,
    );
}

pub(super) fn draw_settings(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let stroke = Stroke::new(1.3, color);
    painter.circle_stroke(c, 5.0, stroke);
    painter.circle_filled(c, 1.6, color);
}

// ---- Chrome, sidebar and page artwork -------------------------------------

/// Sidebar toggle: a window split by a vertical divider.
pub(super) fn draw_panel(painter: &egui::Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.2, color);
    let box_rect = rect.shrink(4.0);
    painter.rect_stroke(
        box_rect,
        CornerRadius::same(2),
        stroke,
        egui::StrokeKind::Inside,
    );
    painter.vline(
        box_rect.min.x + box_rect.width() * 0.38,
        box_rect.y_range(),
        stroke,
    );
}

/// Quick-note pencil.
pub(super) fn draw_note(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let stroke = Stroke::new(1.3, color);
    painter.line(
        vec![
            c + Vec2::new(-5.0, 5.0),
            c + Vec2::new(3.0, -3.0),
            c + Vec2::new(5.0, -5.0),
        ],
        stroke,
    );
    painter.line(
        vec![c + Vec2::new(3.0, -3.0), c + Vec2::new(5.0, -1.0)],
        stroke,
    );
}

/// A small person silhouette used by the account row.
pub(super) fn draw_person(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let stroke = Stroke::new(1.2, color);
    painter.circle_stroke(c + Vec2::new(0.0, -3.0), 3.0, stroke);
    painter.line(
        vec![
            c + Vec2::new(-4.5, 5.5),
            c + Vec2::new(-4.5, 2.0),
            c + Vec2::new(0.0, 0.5),
            c + Vec2::new(4.5, 2.0),
            c + Vec2::new(4.5, 5.5),
        ],
        stroke,
    );
}

/// A small padlock used by the sharing panel's general-access row.
pub(super) fn draw_lock(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let stroke = Stroke::new(1.2, color);
    painter.rect_stroke(
        Rect::from_center_size(c + Vec2::new(0.0, 2.0), Vec2::new(11.0, 9.0)),
        CornerRadius::same(2),
        stroke,
        egui::StrokeKind::Inside,
    );
    painter.line(
        vec![
            c + Vec2::new(-3.5, -1.0),
            c + Vec2::new(-3.5, -4.0),
            c + Vec2::new(0.0, -5.5),
            c + Vec2::new(3.5, -4.0),
            c + Vec2::new(3.5, -1.0),
        ],
        stroke,
    );
}

pub(super) fn draw_arrow_left(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let s = Stroke::new(1.4, color);
    painter.line(
        vec![
            c + Vec2::new(2.5, -5.0),
            c + Vec2::new(-2.5, 0.0),
            c + Vec2::new(2.5, 5.0),
        ],
        s,
    );
}

pub(super) fn draw_arrow_right(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let s = Stroke::new(1.4, color);
    painter.line(
        vec![
            c + Vec2::new(-2.5, -5.0),
            c + Vec2::new(2.5, 0.0),
            c + Vec2::new(-2.5, 5.0),
        ],
        s,
    );
}

pub(super) fn draw_minus(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    painter.line(
        vec![c + Vec2::new(-5.0, 0.0), c + Vec2::new(5.0, 0.0)],
        Stroke::new(1.2, color),
    );
}

pub(super) fn draw_square(painter: &egui::Painter, rect: Rect, color: Color32) {
    painter.rect_stroke(
        Rect::from_min_size(rect.center() + Vec2::new(-5.0, -5.0), Vec2::splat(10.0)),
        CornerRadius::ZERO,
        Stroke::new(1.2, color),
        egui::StrokeKind::Inside,
    );
}

pub(super) fn draw_close(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let s = Stroke::new(1.2, color);
    painter.line(vec![c + Vec2::new(-5.0, -5.0), c + Vec2::new(5.0, 5.0)], s);
    painter.line(vec![c + Vec2::new(5.0, -5.0), c + Vec2::new(-5.0, 5.0)], s);
}

/// Library: an open book drawn from two upright panels.
pub(super) fn draw_library(painter: &egui::Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.3, color);
    let c = rect.center();
    painter.line(
        vec![
            c + Vec2::new(-6.0, -5.0),
            c + Vec2::new(-1.0, -6.0),
            c + Vec2::new(0.0, 6.0),
            c + Vec2::new(-6.0, 5.0),
            c + Vec2::new(-6.0, -5.0),
        ],
        stroke,
    );
    painter.line(
        vec![
            c + Vec2::new(6.0, -5.0),
            c + Vec2::new(1.0, -6.0),
            c + Vec2::new(0.0, 6.0),
            c + Vec2::new(6.0, 5.0),
            c + Vec2::new(6.0, -5.0),
        ],
        stroke,
    );
}

/// "Customize sidebar": two sliders.
pub(super) fn draw_sliders(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let s = Stroke::new(1.2, color);
    for (dy, dx) in [(-3.0, 2.5), (3.0, -2.5)] {
        painter.line(vec![c + Vec2::new(-6.0, dy), c + Vec2::new(6.0, dy)], s);
        painter.circle_filled(c + Vec2::new(dx, dy), 2.0, color);
    }
}

pub(super) fn draw_grid(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    for dx in [-4.0, 2.0] {
        for dy in [-4.0, 2.0] {
            painter.rect_filled(
                Rect::from_min_size(c + Vec2::new(dx, dy), Vec2::splat(2.5)),
                CornerRadius::ZERO,
                color,
            );
        }
    }
}

pub(super) fn draw_help(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    painter.circle_stroke(c, 6.0, Stroke::new(1.2, color));
    painter.text(
        c,
        egui::Align2::CENTER_CENTER,
        "?",
        egui::FontId::proportional(9.0),
        color,
    );
}

pub(super) fn draw_link(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let s = Stroke::new(1.3, color);
    painter.circle_stroke(c + Vec2::new(-3.0, -3.0), 3.0, s);
    painter.circle_stroke(c + Vec2::new(3.0, 3.0), 3.0, s);
    painter.line(vec![c + Vec2::new(-1.0, -1.0), c + Vec2::new(1.0, 1.0)], s);
}

pub(super) fn draw_share(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let s = Stroke::new(1.3, color);
    painter.rect_stroke(
        Rect::from_min_size(c + Vec2::new(-6.0, -2.0), Vec2::new(9.0, 9.0)),
        CornerRadius::same(1),
        s,
        egui::StrokeKind::Inside,
    );
    painter.line(vec![c + Vec2::new(0.0, 4.0), c + Vec2::new(6.0, -3.0)], s);
    painter.line(
        vec![
            c + Vec2::new(2.0, -3.0),
            c + Vec2::new(6.0, -3.0),
            c + Vec2::new(6.0, 1.0),
        ],
        s,
    );
}

/// Three short lines: the small page-outline control at the right edge.
pub(super) fn draw_lines(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let s = Stroke::new(1.3, color);
    for (dy, half) in [(-5.0, 6.0), (0.0, 5.0), (5.0, 4.0)] {
        painter.line(vec![c + Vec2::new(-half, dy), c + Vec2::new(half, dy)], s);
    }
}

/// App menu (hamburger) icon.
pub(super) fn draw_menu(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let s = Stroke::new(1.4, color);
    for dy in [-4.5, 0.0, 4.5] {
        painter.line(vec![c + Vec2::new(-6.0, dy), c + Vec2::new(6.0, dy)], s);
    }
}

/// A stylized pen nib for the floating help button (original shape work).
pub(super) fn draw_pen(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let s = Stroke::new(1.6, color);
    painter.line(
        vec![
            c + Vec2::new(-5.0, 6.0),
            c + Vec2::new(-1.0, -6.0),
            c + Vec2::new(4.0, -3.0),
            c + Vec2::new(-5.0, 6.0),
        ],
        s,
    );
    painter.line(vec![c + Vec2::new(-1.0, -6.0), c + Vec2::new(0.0, 2.0)], s);
    painter.circle_filled(c + Vec2::new(-5.0, 6.0), 1.6, color);
}

/// The large open-book page icon. Original artwork: two pale pages on a blue
/// cover with faint text rules, echoing the pictured emoji without copying it.
pub(super) fn draw_book(painter: &egui::Painter, rect: Rect) {
    let w = rect.width();
    let h = rect.height();
    let blue = Color32::from_rgb(46, 116, 190);
    let pale = Color32::from_rgb(238, 243, 249);
    let rule = Color32::from_rgb(163, 182, 203);

    let cover = Rect::from_min_size(
        rect.min + Vec2::new(w * 0.07, h * 0.14),
        Vec2::new(w * 0.86, h * 0.72),
    );
    painter.rect_filled(cover, CornerRadius::same(7), blue);

    let left = Rect::from_min_size(
        rect.min + Vec2::new(w * 0.10, h * 0.19),
        Vec2::new(w * 0.38, h * 0.60),
    );
    let right = Rect::from_min_size(
        rect.min + Vec2::new(w * 0.52, h * 0.19),
        Vec2::new(w * 0.38, h * 0.60),
    );
    painter.rect_filled(left, CornerRadius::same(3), pale);
    painter.rect_filled(right, CornerRadius::same(3), pale);
    painter.vline(rect.center().x, left.y_range(), Stroke::new(1.0, blue));

    let s = Stroke::new(1.0, rule);
    for i in 0..4 {
        let y = left.min.y + 8.0 + i as f32 * (h * 0.11);
        painter.line(
            vec![
                Pos2::new(left.min.x + 5.0, y),
                Pos2::new(left.max.x - 4.0, y),
            ],
            s,
        );
        painter.line(
            vec![
                Pos2::new(right.min.x + 4.0, y),
                Pos2::new(right.max.x - 5.0, y),
            ],
            s,
        );
    }
}

/// A small colored dot used as a neutral stand-in for inline emoji.
pub(super) fn draw_bullet_dot(painter: &egui::Painter, rect: Rect, color: Color32) {
    painter.circle_filled(rect.center(), 3.0, color);
}

/// Optical lift applied to text glyphs so their ink, not the font box, sits in
/// the center of the icon well.
pub fn glyph_visual_offset(font_size: f32) -> f32 {
    font_size * 0.08
}

/// The icon to draw for a page: the explicit icon when present, otherwise the
/// uppercase first letter of the title (spec-065).
pub fn page_icon_letter(icon: &str, title: &str) -> String {
    if !icon.trim().is_empty() {
        return icon.to_string();
    }
    title
        .trim()
        .chars()
        .next()
        .map(|ch| ch.to_uppercase().collect::<String>())
        .unwrap_or_else(|| "?".to_string())
}

/// Draw the icon that identifies a page (spec-013).
///
/// An empty icon (or the literal `"book"`) is the default: the original
/// open-book artwork for the large page header, or the small page glyph in
/// lists. A custom icon — an emoji/symbol chosen in the picker or an uploaded
/// image reference — is drawn as a centered glyph on a soft rounded well, so
/// the same choice is recognizable in the header, sidebar, breadcrumbs and
/// tab strip (DRY: one renderer for every place a page icon appears).
pub(super) fn draw_page_icon(
    painter: &egui::Painter,
    rect: Rect,
    icon: &str,
    palette: &Palette,
    large: bool,
) {
    if icon.is_empty() || icon == "book" {
        draw_book(painter, rect.shrink(if large { 0.0 } else { 1.0 }));
        return;
    }
    if large {
        painter.rect_filled(rect, CornerRadius::same(12), palette.icon_bg);
    }
    if let Some(name) = icon.strip_prefix("shape:") {
        super::icon_catalog::draw_shape(
            painter,
            rect.shrink(rect.height() * 0.18),
            name,
            palette.text,
        );
        return;
    }
    let label = if let Some(reference) = icon.strip_prefix("path:") {
        // An uploaded image is represented by a small picture placeholder;
        // the reference itself is kept on the page for persistence.
        let _ = reference;
        "▧"
    } else {
        icon
    };
    // Emoji and symbol fonts sit slightly below the geometric center; center
    // the laid-out galley and apply a small optical lift so the ink, not the
    // font box, is centered in the icon well.
    let size = rect.height() * if large { 0.60 } else { 0.72 };
    let galley = painter.layout_no_wrap(
        label.to_string(),
        egui::FontId::proportional(size),
        palette.text,
    );
    let pos = rect.center() - galley.size() / 2.0 - Vec2::new(0.0, glyph_visual_offset(size));
    painter.galley(pos, galley, palette.text);
}
