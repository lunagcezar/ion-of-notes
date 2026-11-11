//! Page-icon customization: the glyph catalog and its floating picker (SRP).
//!
//! Clicking the large book icon opens a Notion-style picker with
//! Emoji / Icons / Upload tabs, a filter field, a Remove action and a grid of
//! glyphs. The chosen icon is stored on the `Page` and rendered everywhere by
//! `icons::draw_page_icon` (header, sidebar, breadcrumbs and tab strip), so the
//! customization is replicated rather than duplicated.
//!
//! The Emoji tab searches a named emoji catalog and the Icons tab only shows
//! the monochrome shapes bundled in `ui::icons` (spec-057).

use egui::{Align2, Color32, CornerRadius, FontId, Margin, Pos2, Sense, Stroke, Vec2};

use super::app::{Command, IconTab, ViewState};
use super::icon_catalog::{EMOJI, Emoji, ICONS, IconGlyph, catalog_matches};
use super::palette::Palette;

fn filtered_emoji(query: &str) -> Vec<&'static Emoji> {
    EMOJI
        .iter()
        .filter(|entry| catalog_matches(entry.name, entry.keywords, query))
        .collect()
}

fn filtered_icons(query: &str) -> Vec<&'static IconGlyph> {
    ICONS
        .iter()
        .filter(|entry| catalog_matches(entry.name, "", query))
        .collect()
}

/// One selectable emoji in the grid.
fn emoji_button(
    ui: &mut egui::Ui,
    entry: &Emoji,
    selected: bool,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(34.0, 30.0), Sense::click());
    if selected {
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
        commands.push(Command::SetPageIcon(entry.glyph.to_string()));
    }
    response.on_hover_text(entry.name);
}

/// One selectable monochrome icon in the grid.
fn icon_button(
    ui: &mut egui::Ui,
    entry: &IconGlyph,
    selected: bool,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(34.0, 30.0), Sense::click());
    if selected {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(5), palette.selected);
    } else if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(5), palette.hover);
    }
    let color = if response.hovered() || selected {
        palette.text
    } else {
        palette.muted
    };
    (entry.draw)(ui.painter(), rect.shrink(7.0), color);
    if response.clicked() {
        commands.push(Command::SetPageIcon(format!("shape:{}", entry.name)));
    }
    response.on_hover_text(entry.name);
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
            // Tabs + Remove. The active tab uses the normal selected
            // background; there is no separate underline (spec-057).
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
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
                    if ui
                        .add(
                            egui::Button::new(text)
                                .frame(false)
                                .fill(if active {
                                    palette.selected
                                } else {
                                    Color32::TRANSPARENT
                                })
                                .corner_radius(CornerRadius::same(5))
                                .min_size(Vec2::new(48.0, 26.0)),
                        )
                        .clicked()
                    {
                        commands.push(Command::SetIconTab(tab));
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Remove")
                                    .size(12.0)
                                    .color(palette.muted),
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
                let (rect, _) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width() - 34.0, 30.0),
                    Sense::hover(),
                );
                ui.painter()
                    .rect_filled(rect, CornerRadius::same(6), palette.hover);
                let mut query = view.icon_picker_query.clone();
                let hint = if view.icon_picker_tab == IconTab::Emoji {
                    "Filter emoji…"
                } else {
                    "Filter icons…"
                };
                let field = ui.put(
                    rect,
                    egui::TextEdit::singleline(&mut query)
                        .frame(egui::Frame::NONE)
                        .font(FontId::proportional(13.0))
                        .hint_text(hint)
                        .text_color(palette.text),
                );
                if field.changed() {
                    commands.push(Command::SetIconQuery(query));
                }
                if ui
                    .add_sized(
                        Vec2::new(30.0, 30.0),
                        egui::Button::new(egui::RichText::new("⤨").size(15.0).color(palette.muted))
                            .frame(false),
                    )
                    .clicked()
                {
                    let pick = match view.icon_picker_tab {
                        IconTab::Emoji => {
                            let list = filtered_emoji(&view.icon_picker_query);
                            list.get((view.icon_picker_query.len() + 3) % list.len().max(1))
                                .map(|entry| entry.glyph.to_string())
                        }
                        IconTab::Icons => {
                            let list = filtered_icons(&view.icon_picker_query);
                            list.get((view.icon_picker_query.len() + 3) % list.len().max(1))
                                .map(|entry| format!("shape:{}", entry.name))
                        }
                        IconTab::Upload => None,
                    };
                    if let Some(icon) = pick {
                        commands.push(Command::SetPageIcon(icon));
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
                    let mut reference: String = ctx.data(|d| d.get_temp(id).unwrap_or_default());
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
                                egui::RichText::new("Use image")
                                    .size(12.0)
                                    .color(Color32::WHITE),
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
                IconTab::Emoji | IconTab::Icons => {
                    let query = view.icon_picker_query.trim();
                    if view.icon_picker_tab == IconTab::Emoji {
                        let entries = filtered_emoji(query);
                        if entries.is_empty() {
                            ui.label(
                                egui::RichText::new(format!("No emoji match “{query}”"))
                                    .size(12.0)
                                    .color(palette.muted),
                            );
                        } else {
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    egui::Grid::new("icon_picker_emoji_grid")
                                        .num_columns(9)
                                        .spacing(Vec2::splat(2.0))
                                        .show(ui, |ui| {
                                            for (index, entry) in entries.iter().enumerate() {
                                                let selected = view.page_icon == entry.glyph;
                                                emoji_button(
                                                    ui, entry, selected, palette, commands,
                                                );
                                                if (index + 1) % 9 == 0 {
                                                    ui.end_row();
                                                }
                                            }
                                        });
                                });
                        }
                    } else {
                        let entries = filtered_icons(query);
                        if entries.is_empty() {
                            ui.label(
                                egui::RichText::new(format!("No icons match “{query}”"))
                                    .size(12.0)
                                    .color(palette.muted),
                            );
                        } else {
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    egui::Grid::new("icon_picker_shape_grid")
                                        .num_columns(9)
                                        .spacing(Vec2::splat(2.0))
                                        .show(ui, |ui| {
                                            for (index, entry) in entries.iter().enumerate() {
                                                let selected = view.page_icon
                                                    == format!("shape:{}", entry.name);
                                                icon_button(ui, entry, selected, palette, commands);
                                                if (index + 1) % 9 == 0 {
                                                    ui.end_row();
                                                }
                                            }
                                        });
                                });
                        }
                    }
                }
            }
        });
    if let Some(inner) = resp
        && ctx.input(|i| i.pointer.any_click())
        && ctx.input(|i| {
            i.pointer
                .interact_pos()
                .is_some_and(|p| !inner.response.rect.contains(p))
        })
    {
        commands.push(Command::CloseIconPicker);
    }
}
