//! Sidebar: search box, Library/Settings/Inbox rows, favorites, page tree and
//! the fixed bottom account / invite / customize group.

use crate::i18n;
use crate::model::{SidebarSectionSource, Workspace};
use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, Vec2};

use crate::ui::constants::{
    FONT_CAPTION, FONT_ROW, ICON_SIZE, ROW_HEIGHT, ROW_ICON_INSET, ROW_LABEL_INSET, ROW_RADIUS,
    SECTION_GAP, SECTION_LABEL_HEIGHT, SIDEBAR_ACCOUNT_ACTIONS_WIDTH, SIDEBAR_ACCOUNT_HEIGHT,
    SIDEBAR_BOTTOM_RESERVE, SIDEBAR_CONTENT_PADDING, SIDEBAR_CUSTOMIZE_HEIGHT, SIDEBAR_DONE_HEIGHT,
    SIDEBAR_INVITE_HEIGHT, SIDEBAR_MIN_SCROLL_HEIGHT, SIDEBAR_SEARCH_HEIGHT,
    SIDEBAR_SHORTCUT_HEIGHT, SPACE_4, SPACE_6, SPACE_8, SPACE_10, SPACE_12, SPACE_14, SPACE_18,
    SPACE_20, SPACE_22, SPACE_24, SPACE_34, SPACE_38, SPACE_40, SPACE_42, SPACE_44, SPACE_46,
    SPACE_64, STROKE_EMPHASIS, STROKE_HAIRLINE,
};

use super::super::commands::{Command, SidebarMenu};
use super::super::icons::{
    draw_chevron, draw_close, draw_dots, draw_grid, draw_help, draw_inbox, draw_library, draw_page,
    draw_page_icon, draw_person, draw_plus, draw_search, draw_settings, draw_sliders,
    page_icon_letter,
};
use super::super::palette::Palette;
use super::super::view::ViewState;

/// One draggable item in customize-sidebar mode.
#[derive(Clone)]
enum SidebarDrag {
    Section(String),
    Shortcut { section: String, shortcut: String },
}

/// One drop candidate recorded while the customize list is laid out.
#[derive(Clone)]
struct SidebarDropTarget {
    rect: Rect,
    kind: SidebarDropKind,
}

#[derive(Clone)]
enum SidebarDropKind {
    Section {
        id: String,
        index: usize,
    },
    Shortcut {
        section: String,
        shortcut: String,
        index: usize,
    },
}

/// The `nav:` targets that can live inside a custom sidebar section. They are
/// not pages, so they carry their own glyph and action (spec-062).
#[derive(Clone, Copy)]
enum NavShortcut {
    Library,
    Settings,
    Inbox,
    Home,
}

impl NavShortcut {
    fn from_id(id: &str) -> Option<Self> {
        match id {
            "library" => Some(Self::Library),
            "settings" => Some(Self::Settings),
            "inbox" => Some(Self::Inbox),
            "home" => Some(Self::Home),
            _ => None,
        }
    }

    fn label(self) -> String {
        match self {
            Self::Library => i18n::t("library"),
            Self::Settings => i18n::t("settings"),
            Self::Inbox => i18n::t("inbox"),
            Self::Home => "Home".to_string(),
        }
    }

    fn draw(self) -> fn(&egui::Painter, Rect, Color32) {
        match self {
            Self::Library => draw_library,
            Self::Settings => draw_settings,
            Self::Inbox => draw_inbox,
            Self::Home => draw_page,
        }
    }

    fn selected(self, view: &ViewState) -> bool {
        match self {
            Self::Library => view.library_open,
            Self::Settings => view.settings.open,
            Self::Inbox => view.inbox_open,
            Self::Home => false,
        }
    }

    fn command(self, ws: &Workspace) -> Option<Command> {
        match self {
            Self::Library => Some(Command::OpenLibrary),
            Self::Settings => Some(Command::OpenSettings),
            Self::Inbox => Some(Command::OpenInbox),
            Self::Home => ws
                .pages
                .iter()
                .find(|page| page.title == "Home")
                .or_else(|| ws.pages.first())
                .map(|page| Command::SelectPage(page.id.clone())),
        }
    }
}

/// The closest drop candidate for the item being dragged.
fn nearest_drop_target<'a>(
    targets: &'a [SidebarDropTarget],
    pos: Pos2,
    drag: &SidebarDrag,
) -> Option<&'a SidebarDropTarget> {
    targets
        .iter()
        .filter(|target| {
            matches!(
                (drag, &target.kind),
                (SidebarDrag::Section(_), SidebarDropKind::Section { .. })
                    | (SidebarDrag::Shortcut { .. }, _)
            )
        })
        .min_by(|a, b| {
            let da = rect_distance(a.rect, pos);
            let db = rect_distance(b.rect, pos);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
        .filter(|target| rect_distance(target.rect, pos) <= SPACE_64)
}

fn rect_distance(rect: Rect, pos: Pos2) -> f32 {
    let clamped = Pos2::new(
        pos.x.clamp(rect.left(), rect.right()),
        pos.y.clamp(rect.top(), rect.bottom()),
    );
    clamped.distance(pos)
}

fn record_drop_target(ui: &egui::Ui, id: egui::Id, rect: Rect, kind: SidebarDropKind) {
    ui.ctx().data_mut(|data| {
        let mut targets = data
            .get_temp::<Vec<SidebarDropTarget>>(id)
            .unwrap_or_default();
        targets.push(SidebarDropTarget { rect, kind });
        data.insert_temp(id, targets);
    });
}

/// A row in customize-sidebar mode: drag handle, glyph, label.
fn customize_row(
    ui: &mut egui::Ui,
    width: f32,
    label: &str,
    selected: bool,
    palette: &Palette,
    draw: fn(&egui::Painter, Rect, Color32),
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, ROW_HEIGHT), Sense::click());
    let fill = if selected {
        palette.selected
    } else if response.hovered() {
        palette.hover
    } else {
        Color32::TRANSPARENT
    };
    if fill != Color32::TRANSPARENT {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(ROW_RADIUS), fill);
    }
    draw_dots(
        ui.painter(),
        Rect::from_min_size(
            rect.min + Vec2::new(SPACE_4, SPACE_10),
            Vec2::splat(SPACE_12),
        ),
        palette.muted,
    );
    draw(
        ui.painter(),
        Rect::from_min_size(
            rect.min + Vec2::new(SPACE_22, SPACE_8),
            Vec2::splat(ICON_SIZE),
        ),
        palette.muted,
    );
    ui.painter().text(
        rect.min + Vec2::new(SPACE_46, ROW_HEIGHT / 2.0),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(FONT_ROW),
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
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(width, ROW_HEIGHT), Sense::click_and_drag());
    let fill = if selected {
        palette.selected
    } else if response.hovered() {
        palette.hover
    } else {
        Color32::TRANSPARENT
    };
    if fill != Color32::TRANSPARENT {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(ROW_RADIUS), fill);
    }
    draw_dots(
        ui.painter(),
        Rect::from_min_size(
            rect.min + Vec2::new(SPACE_4, SPACE_10),
            Vec2::splat(SPACE_12),
        ),
        palette.muted,
    );
    draw(
        ui.painter(),
        Rect::from_min_size(
            rect.min + Vec2::new(SPACE_22, SPACE_8),
            Vec2::splat(ICON_SIZE),
        ),
        palette.muted,
    );
    ui.painter().text(
        rect.min + Vec2::new(SPACE_46, ROW_HEIGHT / 2.0),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(FONT_ROW),
        palette.text,
    );
    let gear = Rect::from_center_size(
        Pos2::new(rect.right() - SPACE_40, rect.center().y),
        Vec2::splat(SPACE_18),
    );
    let exclude = Rect::from_center_size(
        Pos2::new(rect.right() - SPACE_18, rect.center().y),
        Vec2::splat(SPACE_18),
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
                commands.push(Command::RemoveSidebarShortcut(
                    section.to_string(),
                    shortcut.to_string(),
                ));
            }
        } else if let Some(nav) = section.strip_prefix("nav:") {
            commands.push(Command::ToggleSidebarNav(nav.to_string()));
        } else {
            commands.push(Command::RemoveSidebarSection(section.to_string()));
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
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, ROW_HEIGHT), Sense::click());
    let fill = if selected {
        palette.selected
    } else if response.hovered() {
        palette.hover
    } else {
        Color32::TRANSPARENT
    };
    if fill != Color32::TRANSPARENT {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(ROW_RADIUS), fill);
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
    let icon_rect = Rect::from_min_size(
        rect.min + Vec2::new(ROW_ICON_INSET, SPACE_8),
        Vec2::splat(ICON_SIZE),
    );
    draw(ui.painter(), icon_rect, palette.muted);
    ui.painter().text(
        rect.min + Vec2::new(ROW_LABEL_INSET, ROW_HEIGHT / 2.0),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(FONT_ROW),
        palette.text,
    );
    response
}

fn section_label(ui: &mut egui::Ui, text: &str, palette: &Palette) {
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), SECTION_LABEL_HEIGHT),
        Sense::hover(),
    );
    ui.painter().text(
        rect.min + Vec2::new(SPACE_12, SPACE_12),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(FONT_CAPTION),
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
    ui.add_space(SIDEBAR_CONTENT_PADDING);

    // Search box with the Ctrl+K hint.
    let (rect, search_resp) =
        ui.allocate_exact_size(Vec2::new(width, SIDEBAR_SEARCH_HEIGHT), Sense::click());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(ROW_RADIUS), palette.hover);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(ROW_RADIUS),
        Stroke::new(STROKE_HAIRLINE, palette.border),
        egui::StrokeKind::Inside,
    );
    draw_search(
        ui.painter(),
        Rect::from_min_size(
            rect.min + Vec2::new(SPACE_10, SPACE_10),
            Vec2::splat(ICON_SIZE),
        ),
        palette.muted,
    );
    ui.painter().text(
        rect.min + Vec2::new(SPACE_34, SIDEBAR_SEARCH_HEIGHT / 2.0),
        Align2::LEFT_CENTER,
        i18n::t("search_or_ask"),
        FontId::proportional(FONT_ROW),
        palette.muted,
    );
    ui.painter().text(
        rect.max + Vec2::new(-SPACE_10, -SIDEBAR_SEARCH_HEIGHT / 2.0),
        Align2::RIGHT_CENTER,
        "Ctrl+K",
        FontId::proportional(FONT_CAPTION),
        palette.muted,
    );
    if search_resp.clicked() {
        commands.push(Command::OpenSearch);
    }

    ui.add_space(SIDEBAR_CONTENT_PADDING);

    // The page list scrolls when it grows beyond the space left above the
    // fixed bottom group (spec-013: sidebar overflow).
    let bottom_reserve = SIDEBAR_BOTTOM_RESERVE;
    let scroll_height = (ui.available_height() - bottom_reserve).max(SIDEBAR_MIN_SCROLL_HEIGHT);
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .max_height(scroll_height)
        .show(ui, |ui| {
            ui.set_width(width);
            if view.sidebar_customize || view.sidebar_layout.tabs.len() > 1 {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = SPACE_4;
                    for tab in &view.sidebar_layout.tabs {
                        let active = tab.id == view.sidebar_layout.active_tab;
                        if ui
                            .selectable_label(
                                active,
                                egui::RichText::new(&tab.title).size(FONT_CAPTION),
                            )
                            .clicked()
                        {
                            commands.push(Command::SwitchSidebarTab(tab.id.clone()));
                        }
                        if view.sidebar_customize
                            && active
                            && ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("×")
                                            .size(FONT_CAPTION)
                                            .color(palette.muted),
                                    )
                                    .frame(false),
                                )
                                .clicked()
                        {
                            commands.push(Command::RemoveSidebarTab(tab.id.clone()));
                        }
                    }
                    if view.sidebar_customize
                        && ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("+").size(FONT_ROW).color(palette.muted),
                                )
                                .frame(false),
                            )
                            .on_hover_text("Add tab")
                            .clicked()
                    {
                        commands.push(Command::AddSidebarTab);
                    }
                });
                ui.add_space(SPACE_8);
            }
            if view.sidebar_customize {
                render_sidebar_customize(ui, ws, view, palette, commands, width);
            } else {
                render_sidebar_default(ui, ws, view, palette, commands, width);
            }
        });

    // Bottom group: anchored to the bottom of the sidebar.
    render_sidebar_bottom(ui, ws, view, palette, commands, width);
}

#[allow(clippy::too_many_arguments)]
fn render_sidebar_customize(
    ui: &mut egui::Ui,
    ws: &Workspace,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
    width: f32,
) {
    // Edit mode: every row has a handle, settings and exclude
    // actions; sections and shortcuts can be reordered by dragging
    // (spec-062).
    let drag_id = egui::Id::unique("sidebar_drag");
    let targets_id = egui::Id::unique("sidebar_drop_targets");
    ui.ctx().data_mut(|data| {
        data.insert_temp(targets_id, Vec::<SidebarDropTarget>::new());
    });
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
    if home.clicked()
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
        view.settings.open,
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

    ui.add_space(SECTION_GAP);
    for (index, section) in view
        .sidebar_layout
        .sections
        .iter()
        .filter(|section| section.tab == view.sidebar_layout.active_tab)
        .enumerate()
    {
        let section_index = view
            .sidebar_layout
            .sections
            .iter()
            .position(|item| item.id == section.id)
            .unwrap_or(index);
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
                data.insert_temp(drag_id, SidebarDrag::Section(section.id.clone()));
            });
        }
        record_drop_target(
            ui,
            targets_id,
            header.rect,
            SidebarDropKind::Section {
                id: section.id.clone(),
                index: section_index,
            },
        );
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
            for (shortcut_index, shortcut) in section.shortcuts.iter().enumerate() {
                let nav = shortcut
                    .page_id
                    .strip_prefix("nav:")
                    .and_then(NavShortcut::from_id);
                let (label, draw): (String, fn(&egui::Painter, Rect, Color32)) =
                    if let Some(nav) = nav {
                        (nav.label(), nav.draw())
                    } else {
                        let label = ws
                            .page(&shortcut.page_id)
                            .map(|page| page.title.clone())
                            .unwrap_or_else(|| shortcut.label.clone());
                        (label, draw_page)
                    };
                let row = customize_row_actions(
                    ui,
                    width,
                    &label,
                    !shortcut.hidden,
                    palette,
                    draw,
                    &section.id,
                    Some(&shortcut.id),
                    commands,
                );
                if row.drag_started() {
                    ui.ctx().data_mut(|data| {
                        data.insert_temp(
                            drag_id,
                            SidebarDrag::Shortcut {
                                section: section.id.clone(),
                                shortcut: shortcut.id.clone(),
                            },
                        );
                    });
                }
                record_drop_target(
                    ui,
                    targets_id,
                    row.rect,
                    SidebarDropKind::Shortcut {
                        section: section.id.clone(),
                        shortcut: shortcut.id.clone(),
                        index: shortcut_index,
                    },
                );
                if row.clicked() {
                    if let Some(nav) = nav {
                        if let Some(command) = nav.command(ws) {
                            commands.push(command);
                        }
                    } else if let Some(page) = ws.page(&shortcut.page_id) {
                        commands.push(Command::SelectPage(page.id.clone()));
                    }
                }
            }
        }
    }
    if customize_row(ui, width, "Add section", false, palette, draw_plus).clicked() {
        commands.push(Command::OpenSidebarMenu(SidebarMenu::Section));
    }

    let drag = ui.ctx().data(|data| data.get_temp::<SidebarDrag>(drag_id));
    if let Some(drag) = drag {
        let pointer = ui.ctx().input(|input| input.pointer.latest_pos());
        let targets = ui.ctx().data(|data| {
            data.get_temp::<Vec<SidebarDropTarget>>(targets_id)
                .unwrap_or_default()
        });
        if ui.ctx().input(|input| input.pointer.any_released()) {
            if let Some(pos) = pointer
                && let Some(target) = nearest_drop_target(&targets, pos, &drag)
            {
                match (&drag, &target.kind) {
                    (SidebarDrag::Section(id), SidebarDropKind::Section { index, .. }) => {
                        commands.push(Command::MoveSidebarSection(id.clone(), *index));
                    }
                    (
                        SidebarDrag::Shortcut { section, shortcut },
                        SidebarDropKind::Section { id, .. },
                    ) => {
                        if section != id {
                            commands.push(Command::MoveSidebarShortcut(
                                section.clone(),
                                shortcut.clone(),
                                id.clone(),
                                None,
                            ));
                        }
                    }
                    (
                        SidebarDrag::Shortcut { section, shortcut },
                        SidebarDropKind::Shortcut {
                            section: to,
                            shortcut: target_shortcut,
                            index,
                        },
                    ) if shortcut != target_shortcut => {
                        commands.push(Command::MoveSidebarShortcut(
                            section.clone(),
                            shortcut.clone(),
                            to.clone(),
                            Some(*index),
                        ));
                    }
                    _ => {}
                }
            }
            ui.ctx()
                .data_mut(|data| data.remove::<SidebarDrag>(drag_id));
        } else if let Some(pos) = pointer
            && let Some(target) = nearest_drop_target(&targets, pos, &drag)
        {
            ui.painter().rect_stroke(
                target.rect,
                CornerRadius::same(ROW_RADIUS),
                Stroke::new(STROKE_EMPHASIS, palette.accent),
                egui::StrokeKind::Inside,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_sidebar_default(
    ui: &mut egui::Ui,
    ws: &Workspace,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
    width: f32,
) {
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

    for section in view.sidebar_layout.visible_sections() {
        match section.source {
            SidebarSectionSource::Favorites if ws.settings.sidebar_favorites => {
                ui.add_space(SECTION_GAP);
                section_label(ui, &i18n::t("favorites"), palette);
                let favorites = ws.favorites();
                if favorites.is_empty() {
                    section_label(ui, &i18n::t("no_favorites"), palette);
                } else {
                    for page in favorites {
                        let selected = view.current_page == page.id && !view.library_open;
                        let (rect, response) = nav_row_rect(ui, width, selected, palette);
                        draw_page_icon(
                            ui.painter(),
                            Rect::from_min_size(
                                rect.min + Vec2::new(ROW_ICON_INSET, SPACE_8),
                                Vec2::splat(ICON_SIZE),
                            ),
                            &page_icon_letter(&page.icon, &page.title),
                            palette,
                            false,
                        );
                        ui.painter().text(
                            rect.min + Vec2::new(ROW_LABEL_INSET, ROW_HEIGHT / 2.0),
                            Align2::LEFT_CENTER,
                            &page.title,
                            FontId::proportional(FONT_ROW),
                            palette.text,
                        );
                        if response.clicked() {
                            commands.push(Command::SelectPage(page.id.clone()));
                        }
                    }
                }
            }
            SidebarSectionSource::Private if ws.settings.sidebar_private => {
                ui.add_space(SECTION_GAP);
                section_label(ui, &i18n::t("private"), palette);
                render_tree(ui, ws, None, 0, width, view, palette, commands);
                if nav_row(ui, width, &i18n::t("new_page"), false, palette, draw_plus).clicked() {
                    commands.push(Command::NewPage(None));
                }
            }
            SidebarSectionSource::Custom => {
                ui.add_space(SECTION_GAP);
                let title = if section.title.trim().is_empty() {
                    "Shortcuts"
                } else {
                    &section.title
                };
                section_label(ui, title, palette);
                for shortcut in section.shortcuts.iter().filter(|shortcut| !shortcut.hidden) {
                    let nav = shortcut
                        .page_id
                        .strip_prefix("nav:")
                        .and_then(NavShortcut::from_id);
                    let (rect, response) = ui.allocate_exact_size(
                        Vec2::new(width, SIDEBAR_SHORTCUT_HEIGHT),
                        Sense::click(),
                    );
                    let selected = nav.is_some_and(|nav| nav.selected(view));
                    if selected {
                        ui.painter().rect_filled(
                            rect,
                            CornerRadius::same(ROW_RADIUS),
                            palette.selected,
                        );
                    } else if response.hovered() {
                        ui.painter().rect_filled(
                            rect,
                            CornerRadius::same(ROW_RADIUS),
                            palette.hover,
                        );
                    }
                    if let Some(nav) = nav {
                        nav.draw()(
                            ui.painter(),
                            Rect::from_min_size(
                                rect.min + Vec2::new(ROW_ICON_INSET, SPACE_6),
                                Vec2::splat(ICON_SIZE),
                            ),
                            palette.muted,
                        );
                        ui.painter().text(
                            rect.min + Vec2::new(ROW_LABEL_INSET, SPACE_14),
                            Align2::LEFT_CENTER,
                            nav.label(),
                            FontId::proportional(FONT_ROW),
                            palette.text,
                        );
                        if response.clicked()
                            && let Some(command) = nav.command(ws)
                        {
                            commands.push(command);
                        }
                    } else if let Some(page) = ws.page(&shortcut.page_id) {
                        draw_page_icon(
                            ui.painter(),
                            Rect::from_min_size(
                                rect.min + Vec2::new(ROW_ICON_INSET, SPACE_6),
                                Vec2::splat(ICON_SIZE),
                            ),
                            &page_icon_letter(&page.icon, &page.title),
                            palette,
                            false,
                        );
                        ui.painter().text(
                            rect.min + Vec2::new(ROW_LABEL_INSET, SPACE_14),
                            Align2::LEFT_CENTER,
                            &page.title,
                            FontId::proportional(FONT_ROW),
                            palette.text,
                        );
                        if response.clicked() {
                            commands.push(Command::SelectPage(page.id.clone()));
                        }
                    } else {
                        ui.painter().text(
                            rect.min + Vec2::new(ROW_LABEL_INSET, SPACE_14),
                            Align2::LEFT_CENTER,
                            &shortcut.label,
                            FontId::proportional(FONT_ROW),
                            palette.text,
                        );
                    }
                }
            }
            _ => {}
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_sidebar_bottom(
    ui: &mut egui::Ui,
    ws: &Workspace,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
    width: f32,
) {
    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
        // Account row: the workspace switcher button is its own hit target;
        // Help and the four-point control sit outside its hover area.
        let (row, _) =
            ui.allocate_exact_size(Vec2::new(width, SIDEBAR_ACCOUNT_HEIGHT), Sense::hover());
        let account_rect = Rect::from_min_max(
            row.min,
            Pos2::new(row.right() - SIDEBAR_ACCOUNT_ACTIONS_WIDTH, row.bottom()),
        );
        let help_rect = Rect::from_center_size(
            Pos2::new(row.right() - SPACE_38, row.center().y),
            Vec2::splat(SPACE_22),
        );
        let grid_rect = Rect::from_center_size(
            Pos2::new(row.right() - SPACE_14, row.center().y),
            Vec2::splat(SPACE_22),
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
        ui.ctx().data_mut(|data| {
            data.insert_temp(egui::Id::unique("sidebar_account_rect"), account_rect);
            data.insert_temp(egui::Id::unique("sidebar_grid_rect"), grid_rect);
        });
        if account.hovered() || view.workspace_menu_open {
            ui.painter()
                .rect_filled(account_rect, CornerRadius::same(ROW_RADIUS), palette.hover);
        }
        if help.hovered() {
            ui.painter()
                .rect_filled(help_rect, CornerRadius::same(4), palette.hover);
        }
        if grid.hovered() {
            ui.painter()
                .rect_filled(grid_rect, CornerRadius::same(4), palette.hover);
        }
        ui.painter().circle_filled(
            row.min + Vec2::new(SPACE_22, SIDEBAR_ACCOUNT_HEIGHT / 2.0),
            SPACE_14,
            palette.accent,
        );
        ui.painter().text(
            row.min + Vec2::new(SPACE_22, SIDEBAR_ACCOUNT_HEIGHT / 2.0),
            Align2::CENTER_CENTER,
            "N",
            FontId::proportional(FONT_CAPTION),
            Color32::WHITE,
        );
        ui.painter().text(
            row.min + Vec2::new(SPACE_44, SIDEBAR_ACCOUNT_HEIGHT / 2.0),
            Align2::LEFT_CENTER,
            i18n::t("local_workspace"),
            FontId::proportional(FONT_ROW),
            palette.text,
        );
        draw_chevron(
            ui.painter(),
            Rect::from_min_size(
                Pos2::new(account_rect.right() - SPACE_24, row.min.y + SPACE_12),
                Vec2::splat(ICON_SIZE),
            ),
            view.workspace_menu_open,
            palette.muted,
        );
        draw_help(
            ui.painter(),
            Rect::from_center_size(help_rect.center(), Vec2::splat(ICON_SIZE)),
            if help.hovered() {
                palette.text
            } else {
                palette.muted
            },
        );
        draw_grid(
            ui.painter(),
            Rect::from_center_size(grid_rect.center(), Vec2::splat(ICON_SIZE)),
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
            let (card, card_resp) =
                ui.allocate_exact_size(Vec2::new(width, SIDEBAR_INVITE_HEIGHT), Sense::click());
            ui.painter()
                .rect_filled(card, CornerRadius::same(8), palette.callout);
            if card_resp.hovered() {
                ui.painter().rect_stroke(
                    card,
                    CornerRadius::same(8),
                    Stroke::new(STROKE_HAIRLINE, palette.accent),
                    egui::StrokeKind::Inside,
                );
            }
            draw_person(
                ui.painter(),
                Rect::from_min_size(
                    card.min + Vec2::new(SPACE_12, SPACE_12),
                    Vec2::splat(ICON_SIZE),
                ),
                palette.muted,
            );
            ui.painter().text(
                card.min + Vec2::new(ROW_LABEL_INSET, SPACE_20),
                Align2::LEFT_CENTER,
                i18n::t("invite_members"),
                FontId::proportional(FONT_ROW),
                palette.accent,
            );
            ui.painter().text(
                card.min + Vec2::new(SPACE_12, SPACE_42),
                Align2::LEFT_CENTER,
                i18n::t("collaborate_team"),
                FontId::proportional(FONT_CAPTION),
                palette.muted,
            );
            if card_resp.clicked() {
                commands.push(Command::OpenShare);
            }
            ui.add_space(SPACE_8);
        }

        // Customize sidebar toggles the in-place edit mode; the Done button
        // sits above the invite card, as in reference/customize-sidebar.png.
        if view.sidebar_customize {
            let (done, done_resp) =
                ui.allocate_exact_size(Vec2::new(width, SIDEBAR_DONE_HEIGHT), Sense::click());
            ui.painter()
                .rect_filled(done, CornerRadius::same(ROW_RADIUS), palette.accent);
            ui.painter().text(
                done.center(),
                Align2::CENTER_CENTER,
                "Done",
                FontId::proportional(FONT_ROW),
                Color32::WHITE,
            );
            if done_resp.clicked() {
                commands.push(Command::SetSidebarCustomize(false));
            }
            ui.add_space(SPACE_8);
        } else if ws.settings.sidebar_customize {
            let (custom, custom_resp) =
                ui.allocate_exact_size(Vec2::new(width, SIDEBAR_CUSTOMIZE_HEIGHT), Sense::click());
            if custom_resp.hovered() {
                ui.painter()
                    .rect_filled(custom, CornerRadius::same(ROW_RADIUS), palette.hover);
            }
            draw_sliders(
                ui.painter(),
                Rect::from_min_size(
                    custom.min + Vec2::new(ROW_ICON_INSET, SPACE_6),
                    Vec2::splat(ICON_SIZE),
                ),
                palette.muted,
            );
            ui.painter().text(
                custom.min + Vec2::new(ROW_LABEL_INSET, SPACE_14),
                Align2::LEFT_CENTER,
                i18n::t("customize_sidebar"),
                FontId::proportional(FONT_CAPTION),
                palette.muted,
            );
            if custom_resp.clicked() {
                commands.push(Command::SetSidebarCustomize(true));
            }
            ui.add_space(SPACE_10);
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
        let indent = ROW_ICON_INSET + depth as f32 * ICON_SIZE;
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(width, SIDEBAR_SHORTCUT_HEIGHT), Sense::click());
        let fill = if view.current_page == page.id && !view.library_open {
            palette.selected
        } else if response.hovered() {
            palette.hover
        } else {
            Color32::TRANSPARENT
        };
        if fill != Color32::TRANSPARENT {
            ui.painter()
                .rect_filled(rect, CornerRadius::same(ROW_RADIUS), fill);
        }
        let glyph_rect = Rect::from_min_size(
            rect.min + Vec2::new(indent, SPACE_6),
            Vec2::splat(ICON_SIZE),
        );
        if has_children {
            draw_chevron(ui.painter(), glyph_rect, open, palette.muted);
        } else {
            draw_page_icon(
                ui.painter(),
                glyph_rect,
                &page_icon_letter(&page.icon, &page.title),
                palette,
                false,
            );
        }
        ui.painter().text(
            rect.min + Vec2::new(indent + SPACE_24, SPACE_14),
            Align2::LEFT_CENTER,
            &page.title,
            FontId::proportional(FONT_ROW),
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
