//! Sidebar: search box, Library/Settings/Inbox rows, favorites, page tree and
//! the fixed bottom account / invite / customize group.

use crate::i18n;
use crate::model::{SidebarSectionSource, Workspace};
use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, Vec2};

use super::super::commands::{Command, SidebarMenu};
use super::super::icons::{
    draw_chevron, draw_close, draw_dots, draw_grid, draw_help, draw_inbox, draw_library, draw_page,
    draw_page_icon, draw_person, draw_plus, draw_search, draw_settings, draw_sliders,
};
use super::super::palette::Palette;
use super::super::view::ViewState;

/// A row in customize-sidebar mode: drag handle, glyph, label.
fn customize_row(
    ui: &mut egui::Ui,
    width: f32,
    label: &str,
    selected: bool,
    palette: &Palette,
    draw: fn(&egui::Painter, Rect, Color32),
) -> egui::Response {
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
    draw_dots(
        ui.painter(),
        Rect::from_min_size(rect.min + Vec2::new(4.0, 9.0), Vec2::splat(12.0)),
        palette.muted,
    );
    draw(
        ui.painter(),
        Rect::from_min_size(rect.min + Vec2::new(22.0, 7.0), Vec2::splat(16.0)),
        palette.muted,
    );
    ui.painter().text(
        rect.min + Vec2::new(46.0, 15.0),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(13.0),
        palette.text,
    );
    response
}

/// A customize-mode row with the settings and exclude actions on hover.
#[allow(clippy::too_many_arguments)]
fn customize_row_actions(
    ui: &mut egui::Ui,
    width: f32,
    label: &str,
    selected: bool,
    palette: &Palette,
    draw: fn(&egui::Painter, Rect, Color32),
    section: &str,
    shortcut: Option<&str>,
    commands: &mut Vec<Command>,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 30.0), Sense::click_and_drag());
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
    draw_dots(
        ui.painter(),
        Rect::from_min_size(rect.min + Vec2::new(4.0, 9.0), Vec2::splat(12.0)),
        palette.muted,
    );
    draw(
        ui.painter(),
        Rect::from_min_size(rect.min + Vec2::new(22.0, 7.0), Vec2::splat(16.0)),
        palette.muted,
    );
    ui.painter().text(
        rect.min + Vec2::new(46.0, 15.0),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(13.0),
        palette.text,
    );
    if response.hovered() {
        let gear = Rect::from_center_size(
            Pos2::new(rect.right() - 40.0, rect.center().y),
            Vec2::splat(18.0),
        );
        let exclude = Rect::from_center_size(
            Pos2::new(rect.right() - 18.0, rect.center().y),
            Vec2::splat(18.0),
        );
        let gear_resp = ui.interact(
            gear,
            ui.scope_id().with(("sidebar_gear", section, shortcut)),
            Sense::click(),
        );
        let exclude_resp = ui.interact(
            exclude,
            ui.scope_id().with(("sidebar_exclude", section, shortcut)),
            Sense::click(),
        );
        draw_settings(ui.painter(), gear, palette.muted);
        draw_close(ui.painter(), exclude, palette.muted);
        if gear_resp.clicked() {
            commands.push(Command::OpenSidebarMenu(SidebarMenu::RowSettings {
                section: section.to_string(),
                shortcut: shortcut.map(str::to_string),
            }));
        }
        if exclude_resp.clicked() {
            if let Some(shortcut) = shortcut {
                if section == "favorites" {
                    commands.push(Command::ToggleFavorite(shortcut.to_string()));
                } else {
                    commands.push(Command::ToggleSidebarShortcutHidden(
                        section.to_string(),
                        shortcut.to_string(),
                    ));
                }
            } else if let Some(nav) = section.strip_prefix("nav:") {
                commands.push(Command::ToggleSidebarNav(nav.to_string()));
            } else {
                commands.push(Command::ToggleSidebarSectionHidden(section.to_string()));
            }
        }
    }
    response
}

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
    let width = ui.available_width();
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
            if view.sidebar_customize {
                // Edit mode: every row has a handle, settings and exclude
                // actions; sections can be reordered and extended (spec-062).
                let home_id = ws
                    .pages
                    .iter()
                    .find(|page| page.title == "Home")
                    .or_else(|| ws.pages.first())
                    .map(|page| page.id.clone());
                let home = customize_row_actions(
                    ui,
                    width,
                    "Home",
                    home_id.as_deref() == Some(view.current_page.as_str()) && !view.library_open,
                    palette,
                    draw_page,
                    "nav:home",
                    None,
                    commands,
                );
                let plus_rect =
                    Rect::from_min_size(home.rect.max - Vec2::new(30.0, 24.0), Vec2::splat(18.0));
                let plus = ui.interact(
                    plus_rect,
                    ui.scope_id().with("customize_home_plus"),
                    Sense::click(),
                );
                if plus.hovered() {
                    ui.painter()
                        .rect_filled(plus_rect, CornerRadius::same(4), palette.hover);
                }
                draw_plus(ui.painter(), plus_rect, palette.muted);
                if plus.clicked() {
                    commands.push(Command::NewPage(None));
                } else if home.clicked()
                    && let Some(id) = home_id
                {
                    commands.push(Command::SelectPage(id));
                }
                if customize_row_actions(
                    ui,
                    width,
                    &i18n::t("library"),
                    view.library_open,
                    palette,
                    draw_library,
                    "nav:library",
                    None,
                    commands,
                )
                .clicked()
                {
                    commands.push(Command::OpenLibrary);
                }
                if customize_row_actions(
                    ui,
                    width,
                    &i18n::t("settings"),
                    view.settings_open,
                    palette,
                    draw_settings,
                    "nav:settings",
                    None,
                    commands,
                )
                .clicked()
                {
                    commands.push(Command::OpenSettings);
                }
                if customize_row_actions(
                    ui,
                    width,
                    &i18n::t("inbox"),
                    view.inbox_open,
                    palette,
                    draw_inbox,
                    "nav:inbox",
                    None,
                    commands,
                )
                .clicked()
                {
                    commands.push(Command::OpenInbox);
                }
                if let Some(page) = ws.page(&view.current_page)
                    && page.title != "Home"
                {
                    customize_row_actions(
                        ui,
                        width,
                        &page.title,
                        !view.library_open,
                        palette,
                        draw_page,
                        "nav:page",
                        None,
                        commands,
                    );
                }
                if customize_row(ui, width, "Add shortcut", false, palette, draw_plus).clicked() {
                    commands.push(Command::OpenSidebarMenu(SidebarMenu::Shortcut));
                }

                ui.add_space(18.0);
                let drag_id = egui::Id::unique("sidebar_drag_section");
                let drop_id = egui::Id::unique("sidebar_drop_section");
                let shortcut_drag_id = egui::Id::unique("sidebar_drag_shortcut");
                let shortcut_target_id = egui::Id::unique("sidebar_shortcut_target");
                for (index, section) in view.sidebar_layout.sections.iter().enumerate() {
                    let title = if section.hidden {
                        format!("{} (hidden)", section.title)
                    } else {
                        section.title.clone()
                    };
                    let header = customize_row_actions(
                        ui,
                        width,
                        &title,
                        false,
                        palette,
                        draw_page,
                        &section.id,
                        None,
                        commands,
                    );
                    if header.drag_started() {
                        ui.ctx().data_mut(|data| {
                            data.insert_temp(drag_id, section.id.clone());
                        });
                    }
                    if header.hovered() && ui.input(|i| i.pointer.primary_down()) {
                        ui.ctx().data_mut(|data| {
                            data.insert_temp(drop_id, index);
                        });
                        if let Some((_, _)) = ui
                            .ctx()
                            .data(|data| data.get_temp::<(String, String)>(shortcut_drag_id))
                        {
                            ui.ctx().data_mut(|data| {
                                data.insert_temp(shortcut_target_id, section.id.clone());
                            });
                        }
                    }
                    if section.source == crate::model::SidebarSectionSource::Favorites {
                        for page in ws.favorites() {
                            if customize_row_actions(
                                ui,
                                width,
                                &page.title,
                                view.current_page == page.id && !view.library_open,
                                palette,
                                draw_page,
                                "favorites",
                                Some(&page.id),
                                commands,
                            )
                            .clicked()
                            {
                                commands.push(Command::SelectPage(page.id.clone()));
                            }
                        }
                    } else if section.source == crate::model::SidebarSectionSource::Custom {
                        for shortcut in &section.shortcuts {
                            let label = ws
                                .page(&shortcut.page_id)
                                .map(|page| page.title.clone())
                                .unwrap_or_else(|| shortcut.label.clone());
                            let row = customize_row_actions(
                                ui,
                                width,
                                &label,
                                !shortcut.hidden,
                                palette,
                                draw_page,
                                &section.id,
                                Some(&shortcut.id),
                                commands,
                            );
                            if row.drag_started() {
                                ui.ctx().data_mut(|data| {
                                    data.insert_temp(
                                        shortcut_drag_id,
                                        (section.id.clone(), shortcut.id.clone()),
                                    );
                                });
                            }
                            if row.hovered() && ui.input(|i| i.pointer.primary_down()) {
                                ui.ctx().data_mut(|data| {
                                    data.insert_temp(shortcut_target_id, section.id.clone());
                                });
                            }
                            if row.clicked()
                                && let Some(page) = ws.page(&shortcut.page_id)
                            {
                                commands.push(Command::SelectPage(page.id.clone()));
                            }
                        }
                    }
                }
                if customize_row(ui, width, "Add section", false, palette, draw_plus).clicked() {
                    commands.push(Command::OpenSidebarMenu(SidebarMenu::Section));
                }
                if let Some(id) = ui.ctx().data(|data| data.get_temp::<String>(drag_id))
                    && !ui.input(|i| i.pointer.primary_down())
                {
                    if let Some(target) = ui.ctx().data(|data| data.get_temp::<usize>(drop_id)) {
                        commands.push(Command::MoveSidebarSection(id, target));
                    }
                    ui.ctx().data_mut(|data| {
                        data.remove::<String>(drag_id);
                        data.remove::<usize>(drop_id);
                    });
                }
                if let Some((from, shortcut_id)) = ui
                    .ctx()
                    .data(|data| data.get_temp::<(String, String)>(shortcut_drag_id))
                    && !ui.input(|i| i.pointer.primary_down())
                {
                    if let Some(target) = ui
                        .ctx()
                        .data(|data| data.get_temp::<String>(shortcut_target_id))
                        && target != from
                    {
                        commands.push(Command::MoveSidebarShortcut(from, shortcut_id, target));
                    }
                    ui.ctx().data_mut(|data| {
                        data.remove::<(String, String)>(shortcut_drag_id);
                        data.remove::<String>(shortcut_target_id);
                    });
                }
            } else {
                if ws.settings.sidebar_library
                    && nav_row(
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
                if ws.settings.sidebar_settings
                    && nav_row(
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
                if ws.settings.sidebar_inbox
                    && nav_row(ui, width, &i18n::t("inbox"), false, palette, draw_inbox).clicked()
                {
                    commands.push(Command::OpenInbox);
                }

                for section in view.sidebar_layout.visible_sections() {
                    match section.source {
                        SidebarSectionSource::Favorites if ws.settings.sidebar_favorites => {
                            ui.add_space(18.0);
                            section_label(ui, &i18n::t("favorites"), palette);
                            let favorites = ws.favorites();
                            if favorites.is_empty() {
                                section_label(ui, &i18n::t("no_favorites"), palette);
                            } else {
                                for page in favorites {
                                    let selected =
                                        view.current_page == page.id && !view.library_open;
                                    let (rect, response) =
                                        nav_row_rect(ui, width, selected, palette);
                                    draw_page_icon(
                                        ui.painter(),
                                        Rect::from_min_size(
                                            rect.min + Vec2::new(10.0, 7.0),
                                            Vec2::splat(16.0),
                                        ),
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
                        }
                        SidebarSectionSource::Private if ws.settings.sidebar_private => {
                            ui.add_space(18.0);
                            section_label(ui, &i18n::t("private"), palette);
                            render_tree(ui, ws, None, 0, width, view, palette, commands);
                            if nav_row(ui, width, &i18n::t("new_page"), false, palette, draw_plus)
                                .clicked()
                            {
                                commands.push(Command::NewPage(None));
                            }
                        }
                        SidebarSectionSource::Custom => {
                            ui.add_space(18.0);
                            let title = if section.title.trim().is_empty() {
                                "Shortcuts"
                            } else {
                                &section.title
                            };
                            section_label(ui, title, palette);
                            for shortcut in
                                section.shortcuts.iter().filter(|shortcut| !shortcut.hidden)
                            {
                                let (rect, response) =
                                    ui.allocate_exact_size(Vec2::new(width, 28.0), Sense::click());
                                if response.hovered() {
                                    ui.painter().rect_filled(
                                        rect,
                                        CornerRadius::same(6),
                                        palette.hover,
                                    );
                                }
                                if let Some(page) = ws.page(&shortcut.page_id) {
                                    draw_page_icon(
                                        ui.painter(),
                                        Rect::from_min_size(
                                            rect.min + Vec2::new(10.0, 6.0),
                                            Vec2::splat(16.0),
                                        ),
                                        &page.icon,
                                        palette,
                                        false,
                                    );
                                    ui.painter().text(
                                        rect.min + Vec2::new(36.0, 14.0),
                                        Align2::LEFT_CENTER,
                                        &page.title,
                                        FontId::proportional(13.0),
                                        palette.text,
                                    );
                                    if response.clicked() {
                                        commands.push(Command::SelectPage(page.id.clone()));
                                    }
                                } else {
                                    ui.painter().text(
                                        rect.min + Vec2::new(36.0, 14.0),
                                        Align2::LEFT_CENTER,
                                        &shortcut.label,
                                        FontId::proportional(13.0),
                                        palette.text,
                                    );
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        });

    // Bottom group: anchored to the bottom of the sidebar.
    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
        // Account row: the workspace switcher button is its own hit target;
        // Help and the four-point control sit outside its hover area.
        let (row, _) = ui.allocate_exact_size(Vec2::new(width, 40.0), Sense::hover());
        let account_rect = Rect::from_min_max(row.min, Pos2::new(row.right() - 52.0, row.bottom()));
        let help_rect = Rect::from_center_size(
            Pos2::new(row.right() - 38.0, row.center().y),
            Vec2::splat(22.0),
        );
        let grid_rect = Rect::from_center_size(
            Pos2::new(row.right() - 14.0, row.center().y),
            Vec2::splat(22.0),
        );
        let account = ui.interact(
            account_rect,
            ui.scope_id().with("workspace_switcher"),
            Sense::click(),
        );
        let help = ui.interact(
            help_rect,
            ui.scope_id().with("sidebar_help"),
            Sense::click(),
        );
        let grid = ui.interact(
            grid_rect,
            ui.scope_id().with("sidebar_grid"),
            Sense::click(),
        );
        if account.hovered() || view.workspace_menu_open {
            ui.painter()
                .rect_filled(account_rect, CornerRadius::same(6), palette.hover);
        }
        if help.hovered() {
            ui.painter()
                .rect_filled(help_rect, CornerRadius::same(4), palette.hover);
        }
        if grid.hovered() {
            ui.painter()
                .rect_filled(grid_rect, CornerRadius::same(4), palette.hover);
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
            Rect::from_min_size(
                Pos2::new(account_rect.right() - 24.0, row.min.y + 12.0),
                Vec2::splat(16.0),
            ),
            view.workspace_menu_open,
            palette.muted,
        );
        draw_help(
            ui.painter(),
            Rect::from_center_size(help_rect.center(), Vec2::splat(16.0)),
            if help.hovered() {
                palette.text
            } else {
                palette.muted
            },
        );
        draw_grid(
            ui.painter(),
            Rect::from_center_size(grid_rect.center(), Vec2::splat(16.0)),
            if grid.hovered() {
                palette.text
            } else {
                palette.muted
            },
        );
        if account.clicked() {
            commands.push(Command::ToggleWorkspaceMenu);
        }
        if help.clicked() {
            commands.push(Command::OpenShortcuts);
        }
        if grid.clicked() {
            commands.push(Command::ToggleGridMenu);
        }

        // Invite-members card (same footprint as the pictured sidebar card).
        // Members, sharing and collaboration live on the self-hosted server.
        if ws.settings.sidebar_invite {
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
        }

        // Customize sidebar toggles the in-place edit mode; the Done button
        // sits above the invite card, as in reference/customize-sidebar.png.
        if view.sidebar_customize {
            let (done, done_resp) = ui.allocate_exact_size(Vec2::new(width, 32.0), Sense::click());
            ui.painter()
                .rect_filled(done, CornerRadius::same(6), palette.accent);
            ui.painter().text(
                done.center(),
                Align2::CENTER_CENTER,
                "Done",
                FontId::proportional(13.0),
                Color32::WHITE,
            );
            if done_resp.clicked() {
                commands.push(Command::SetSidebarCustomize(false));
            }
            ui.add_space(8.0);
        } else if ws.settings.sidebar_customize {
            let (custom, custom_resp) =
                ui.allocate_exact_size(Vec2::new(width, 28.0), Sense::click());
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
                commands.push(Command::SetSidebarCustomize(true));
            }
            ui.add_space(10.0);
        }
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
