//! Inline table database: table view, status chips and board view.

use crate::model::Page;
use crate::ui::constants as metrics;
use egui::{Align2, Color32, CornerRadius, FontId, Margin, Rect, Sense, Vec2};

use super::super::commands::Command;
use super::super::format::format_number;
use super::super::palette::{Palette, status_color};
use super::super::view::ViewState;

#[allow(clippy::too_many_arguments)]
pub(super) fn render_database(
    ui: &mut egui::Ui,
    page: &mut Page,
    view: &ViewState,
    width: f32,
    gutter: f32,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    ui.horizontal(|ui| {
        ui.add_space(gutter);
        let table = ui.add(
            egui::Button::new(
                egui::RichText::new("Table").color(if view.settings.table_view {
                    palette.text
                } else {
                    palette.muted
                }),
            )
            .frame(false),
        );
        if table.clicked() {
            commands.push(Command::SetTableView(true));
        }
        let board = ui.add(
            egui::Button::new(
                egui::RichText::new("Board").color(if view.settings.table_view {
                    palette.muted
                } else {
                    palette.text
                }),
            )
            .frame(false),
        );
        if board.clicked() {
            commands.push(Command::SetTableView(false));
        }
        ui.add_space(metrics::SPACE_12);
        ui.label(
            egui::RichText::new(format!(
                "{} rows",
                format_number(page.rows.len() as f64, view.settings.number_format)
            ))
            .size(metrics::FONT_CAPTION)
            .color(palette.muted),
        );
    });
    ui.add_space(metrics::SPACE_8);
    ui.separator();

    if view.settings.table_view {
        render_table(ui, page, width, gutter, palette, commands);
    } else {
        render_board(ui, page, width, palette);
    }
}

pub(super) fn header_cell(ui: &mut egui::Ui, width: f32, text: &str, palette: &Palette) {
    ui.add_sized(
        Vec2::new(width, 22.0),
        egui::Label::new(
            egui::RichText::new(text)
                .size(metrics::FONT_CAPTION)
                .strong()
                .color(palette.muted),
        ),
    );
}

fn render_table(
    ui: &mut egui::Ui,
    page: &mut Page,
    width: f32,
    gutter: f32,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let name_w = (width * 0.40).max(170.0);
    let status_w = (width * 0.22).max(110.0);
    let assignee_w = (width * 0.16).max(80.0);
    let due_w = (width * 0.16).max(80.0);
    let viewport = ui.clip_rect();
    let interactive = ui.memory(|memory| memory.focused().is_some())
        || ui.input(|input| input.pointer.any_down());

    ui.horizontal(|ui| {
        ui.add_space(gutter);
        header_cell(ui, name_w, "Name", palette);
        header_cell(ui, status_w, "Status", palette);
        header_cell(ui, assignee_w, "Assignee", palette);
        header_cell(ui, due_w, "Due", palette);
    });
    ui.separator();

    for (index, row) in page.rows.iter_mut().enumerate() {
        let row_id = row.id.clone();
        let row_height = 28.0;
        let top = ui.cursor().top();
        if !interactive
            && crate::ui::block_outside_viewport(
                top,
                row_height,
                (viewport.top(), viewport.bottom()),
                400.0,
            )
        {
            ui.add_space(row_height);
            continue;
        }
        let clicked = egui::Frame::NONE
            .fill(if index % 2 == 1 {
                palette.callout
            } else {
                Color32::TRANSPARENT
            })
            .corner_radius(CornerRadius::same(metrics::ICON_RADIUS))
            .inner_margin(Margin::symmetric(4, 2))
            .show(ui, |ui| {
                let mut status_clicked = false;
                ui.horizontal(|ui| {
                    ui.add_space(gutter);
                    ui.add_sized(
                        Vec2::new(name_w, 24.0),
                        egui::TextEdit::singleline(&mut row.name)
                            .frame(egui::Frame::NONE)
                            .text_color(palette.text),
                    );
                    let (rect, response) =
                        ui.allocate_exact_size(Vec2::new(status_w - 12.0, 22.0), Sense::click());
                    let chip = Rect::from_min_size(
                        rect.min,
                        Vec2::new((rect.width() - 6.0).max(40.0), 20.0),
                    );
                    ui.painter().rect_filled(
                        chip,
                        CornerRadius::same(metrics::ICON_RADIUS),
                        status_color(&row.status),
                    );
                    ui.painter().text(
                        chip.min + Vec2::new(8.0, 10.0),
                        Align2::LEFT_CENTER,
                        &row.status,
                        FontId::proportional(metrics::FONT_CAPTION),
                        palette.chip_text,
                    );
                    status_clicked = response.clicked();
                    ui.add_sized(
                        Vec2::new(assignee_w, 24.0),
                        egui::TextEdit::singleline(&mut row.assignee)
                            .frame(egui::Frame::NONE)
                            .text_color(palette.text),
                    );
                    ui.add_sized(
                        Vec2::new(due_w, 24.0),
                        egui::TextEdit::singleline(&mut row.due)
                            .frame(egui::Frame::NONE)
                            .text_color(palette.text),
                    );
                });
                status_clicked
            })
            .inner;
        if clicked {
            commands.push(Command::CycleStatus(row_id));
        }
    }
    ui.add_space(metrics::SPACE_6);
    if ui
        .add(egui::Button::new("+ New row").frame(false))
        .clicked()
    {
        commands.push(Command::AddRow);
    }
}

fn render_board(ui: &mut egui::Ui, page: &mut Page, width: f32, palette: &Palette) {
    let statuses = ["Not started", "In progress", "Done"];
    let column_width = (width - 24.0) / 3.0;
    let groups: Vec<Vec<&crate::model::TableRow>> = statuses
        .iter()
        .map(|status| {
            page.rows
                .iter()
                .filter(|row| &row.status == status)
                .collect()
        })
        .collect();
    ui.columns(3, |columns| {
        for (index, status) in statuses.iter().enumerate() {
            let col = &mut columns[index];
            col.set_width(column_width);
            col.label(
                egui::RichText::new(*status)
                    .size(metrics::FONT_CAPTION)
                    .strong()
                    .color(palette.muted),
            );
            col.add_space(6.0);
            for row in &groups[index] {
                egui::Frame::NONE
                    .fill(palette.callout)
                    .corner_radius(CornerRadius::same(metrics::ROW_RADIUS))
                    .inner_margin(Margin::same(10))
                    .show(col, |ui| {
                        ui.set_width(column_width - 30.0);
                        ui.label(egui::RichText::new(&row.name).color(palette.text));
                        ui.label(
                            egui::RichText::new(format!("{}  {}", row.assignee, row.due))
                                .size(metrics::FONT_MICRO)
                                .color(palette.muted),
                        );
                    });
                col.add_space(6.0);
            }
            col.add_space(6.0);
        }
    });
}
