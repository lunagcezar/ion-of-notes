//! Page-icon customization: the glyph catalog and its floating picker (SRP).
//!
//! Clicking the large book icon opens a Notion-style picker with
//! Emoji / Icons / Upload tabs, a filter field, a Remove action and a grid of
//! glyphs. The chosen icon is stored on the `Page` and rendered everywhere by
//! `icons::draw_page_icon` (header, sidebar, breadcrumbs and tab strip), so the
//! customization is replicated rather than duplicated.

use egui::{Align2, Color32, CornerRadius, FontId, Margin, Pos2, Sense, Stroke, Vec2};

use super::app::{Command, IconTab, ViewState};
use super::palette::Palette;

/// Emoji choices for the page icon picker (rendered via system emoji font when
/// available, with symbol fallbacks).
pub(super) const EMOJI: &[&str] = &[
    "😀", "😃", "😄", "😁", "😊", "🙂", "😉", "😍", "🤔", "😎", "🥳", "🤖",
    "📄", "📘", "📚", "📓", "📝", "📌", "📎", "🗂", "📁", "📂", "🗃", "🗒",
    "✅", "☑", "⭐", "🌟", "🔥", "💡", "🎯", "🚀", "🛠", "⚙", "🔒", "🔓",
    "🏠", "🏢", "🌍", "🌐", "💬", "📣", "📅", "⏰", "📈", "📊", "🧩", "🎨",
    "🧪", "🔬", "📦", "🧭", "🗺", "❤️", "💙", "💚", "💛", "🟣", "🔵", "🟢",
];

/// Monochrome icon/symbol glyphs for the "Icons" tab.
pub(super) const ICONS: &[&str] = &[
    "★", "☆", "☀", "☁", "☂", "☕", "☎", "✉", "✎", "✏", "✂", "✦", "✔", "✖", "⚑", "⚙",
    "←", "→", "↑", "↓", "↔", "↕", "⟲", "⟳", "■", "□", "▲", "▼", "◀", "▶", "◆", "◇",
    "●", "○", "◎", "◐", "◑", "▢", "▣", "▤", "▥", "⊞", "⊟", "∞", "§", "¶", "•",
    "▪", "▫", "◊", "⌂", "⌛", "♫", "♪", "❖", "➤", "✚", "⚓", "⚡", "❄", "✿",
];

fn catalog(tab: IconTab) -> &'static [&'static str] {
    match tab {
        IconTab::Emoji => EMOJI,
        IconTab::Icons => ICONS,
        IconTab::Upload => &[],
    }
}

/// One selectable glyph in the grid.
fn glyph_button(
    ui: &mut egui::Ui,
    glyph: &str,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(34.0, 30.0), Sense::click());
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(5), palette.hover);
    }
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        glyph,
        FontId::proportional(19.0),
        palette.text,
    );
    if response.clicked() {
        commands.push(Command::SetPageIcon(glyph.to_string()));
    }
}

/// The floating icon picker, anchored next to the page icon.
pub(super) fn render_icon_picker(
    ctx: &egui::Context,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let screen = ctx.viewport_rect();
    let size = Vec2::new(404.0, 344.0);
    let pos = Pos2::new(screen.left() + 304.0, screen.top() + 236.0);
    let resp = egui::Window::new("page_icon_picker")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_size(size)
        .fixed_pos(pos)
        .frame(
            egui::Frame::NONE
                .fill(palette.content)
                .corner_radius(CornerRadius::same(10))
                .inner_margin(Margin::same(12))
                .stroke(Stroke::new(1.0, palette.border)),
        )
        .show(ctx, |ui| {
            // Tabs + Remove.
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                for (tab, label) in [
                    (IconTab::Emoji, "Emoji"),
                    (IconTab::Icons, "Icons"),
                    (IconTab::Upload, "Upload"),
                ] {
                    let active = view.icon_picker_tab == tab;
                    let text = egui::RichText::new(label).size(13.0).color(if active {
                        palette.text
                    } else {
                        palette.muted
                    });
                    if ui.add(egui::Button::new(text).frame(false)).clicked() {
                        commands.push(Command::SetIconTab(tab));
                    }
                    if active {
                        let r = ui.max_rect();
                        ui.painter().hline(
                            (r.min.x - 12.0)..=(r.min.x + 14.0),
                            r.min.y + 2.0,
                            Stroke::new(2.0, palette.accent),
                        );
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Remove").size(12.0).color(palette.muted),
                            )
                            .frame(false),
                        )
                        .clicked()
                    {
                        commands.push(Command::SetPageIcon(String::new()));
                    }
                });
            });
            ui.add_space(12.0);

            // Filter field + shuffle.
            ui.horizontal(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width() - 34.0, 30.0), Sense::hover());
                ui.painter()
                    .rect_filled(rect, CornerRadius::same(6), palette.hover);
                let mut query = view.icon_picker_query.clone();
                let field = ui.put(
                    rect,
                    egui::TextEdit::singleline(&mut query)
                        .frame(egui::Frame::NONE)
                        .font(FontId::proportional(13.0))
                        .hint_text("Filter…")
                        .text_color(palette.text),
                );
                if field.changed() {
                    commands.push(Command::SetIconQuery(query));
                }
                if ui
                    .add_sized(
                        Vec2::new(30.0, 30.0),
                        egui::Button::new(
                            egui::RichText::new("⤨").size(15.0).color(palette.muted),
                        )
                        .frame(false),
                    )
                    .clicked()
                {
                    let list = catalog(view.icon_picker_tab);
                    if !list.is_empty() {
                        let pick = list[(view.icon_picker_query.len()) % list.len()];
                        commands.push(Command::SetPageIcon(pick.to_string()));
                    }
                }
            });
            ui.add_space(10.0);

            match view.icon_picker_tab {
                IconTab::Upload => {
                    ui.label(
                        egui::RichText::new("Upload an image")
                            .size(12.0)
                            .strong()
                            .color(palette.muted),
                    );
                    ui.add_space(6.0);
                    let id = egui::Id::unique("icon_upload_reference");
                    let mut reference: String =
                        ctx.data(|d| d.get_temp(id).unwrap_or_default());
                    let field = ui.add(
                        egui::TextEdit::singleline(&mut reference)
                            .desired_width(f32::INFINITY)
                            .hint_text("Paste an image URL or file path")
                            .text_color(palette.text),
                    );
                    if field.changed() {
                        ctx.data_mut(|d| d.insert_temp(id, reference.clone()));
                    }
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(
                            "The image becomes the page icon and appears in the sidebar too.",
                        )
                        .size(11.0)
                        .color(palette.muted),
                    );
                    ui.add_space(10.0);
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Use image").size(12.0).color(Color32::WHITE),
                            )
                            .fill(palette.accent)
                            .corner_radius(CornerRadius::same(6)),
                        )
                        .clicked()
                        && !reference.trim().is_empty()
                    {
                        commands.push(Command::SetPageIcon(format!("path:{}", reference.trim())));
                    }
                }
                _ => {
                    ui.label(
                        egui::RichText::new(if view.icon_picker_tab == IconTab::Emoji {
                            "People"
                        } else {
                            "Icons"
                        })
                        .size(11.0)
                        .color(palette.muted),
                    );
                    ui.add_space(6.0);
                    // Typing an arbitrary glyph also becomes a usable icon.
                    if !view.icon_picker_query.trim().is_empty() {
                        let typed = view.icon_picker_query.trim().to_string();
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new(format!("Use “{typed}”"))
                                        .size(12.0)
                                        .color(palette.accent),
                                )
                                .frame(false),
                            )
                            .clicked()
                        {
                            commands.push(Command::SetPageIcon(typed));
                        }
                        ui.add_space(6.0);
                    }
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            egui::Grid::new("icon_picker_grid")
                                .num_columns(9)
                                .spacing(Vec2::splat(2.0))
                                .show(ui, |ui| {
                                    let mut column = 0;
                                    for glyph in catalog(view.icon_picker_tab) {
                                        glyph_button(ui, glyph, palette, commands);
                                        column += 1;
                                        if column % 9 == 0 {
                                            ui.end_row();
                                        }
                                    }
                                });
                        });
                }
            }
        });
    if let Some(inner) = resp
        && ctx.input(|i| i.pointer.any_click())
        && ctx.input(|i| i.pointer.interact_pos().map_or(false, |p| !inner.response.rect.contains(p)))
    {
        commands.push(Command::CloseIconPicker);
    }
}
