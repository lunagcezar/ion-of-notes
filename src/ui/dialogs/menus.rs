// ---- App / workspace / grid menus -----------------------------------------

//! App, workspace, grid and slash menus.

use egui::{Align2, Color32, CornerRadius, FontId, Margin, Pos2, Rect, Sense, Stroke, Vec2};

use super::super::commands::{Command, SidebarMenu};
use super::super::palette::Palette;
use super::super::view::ViewState;
use super::super::widgets::{menu_button, popover_frame};
use crate::model::{BlockKind, Workspace};

/// One app-menu section: title plus label/hint/command rows.
type AppMenuSection = (&'static str, Vec<(&'static str, &'static str, Command)>);

pub(crate) fn render_app_menu(ctx: &egui::Context, palette: &Palette, commands: &mut Vec<Command>) {
    let screen = ctx.viewport_rect();
    let sections: Vec<AppMenuSection> = vec![
        (
            "File",
            vec![
                ("New Tab", "Ctrl+T", Command::OpenNewTab),
                ("New Window", "Ctrl+Shift+N", Command::NoOp),
                (
                    "Reopen Last Closed Tab",
                    "Ctrl+Shift+T",
                    Command::ReopenClosedTab,
                ),
                ("Close Tab", "Ctrl+W", Command::CloseTab(usize::MAX)),
                ("---", "", Command::NoOp),
                ("Print…", "", Command::NoOp),
                ("Restart to Apply Update", "", Command::NoOp),
                ("---", "", Command::NoOp),
                ("Exit Without Saving Tabs", "Ctrl+Shift+Q", Command::NoOp),
                ("Exit", "", Command::WindowClose),
            ],
        ),
        (
            "Edit",
            vec![
                ("Undo", "Ctrl+Z", Command::Undo),
                ("Redo", "Ctrl+Shift+Z", Command::Redo),
                ("---", "", Command::NoOp),
                ("Copy link", "Ctrl+L", Command::CopyLink),
                ("Copy page name", "", Command::CopyPageName),
            ],
        ),
        (
            "View",
            vec![
                ("Toggle sidebar", "Ctrl+\\", Command::ToggleSidebar),
                ("Zoom in", "Ctrl++", Command::ZoomIn),
                ("Zoom out", "Ctrl+-", Command::ZoomOut),
                ("Toggle theme", "Ctrl+Shift+L", Command::ToggleTheme),
                ("Full screen", "F11", Command::ToggleFullscreen),
            ],
        ),
        (
            "History",
            vec![
                ("Back", "Ctrl+[", Command::GoBack),
                ("Forward", "Ctrl+]", Command::GoForward),
                ("Version history", "", Command::OpenVersionHistory),
            ],
        ),
        (
            "Window",
            vec![
                ("Minimize", "", Command::WindowMinimize),
                ("Maximize", "", Command::WindowMaximize),
                ("Next tab", "Ctrl+Tab", Command::NextTab),
                ("Previous tab", "Ctrl+Shift+Tab", Command::PreviousTab),
            ],
        ),
        (
            "Help",
            vec![
                ("Keyboard shortcuts", "F1", Command::OpenShortcuts),
                ("Settings", "Ctrl+,", Command::OpenSettings),
            ],
        ),
    ];
    let active_id = egui::Id::unique("app_menu_active");
    let mut active = ctx
        .data(|data| data.get_temp::<usize>(active_id))
        .unwrap_or(0)
        .min(sections.len().saturating_sub(1));
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        commands.push(Command::CloseAppMenu);
        return;
    }
    let resp = egui::Window::new("app_menu")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_pos(Pos2::new(screen.left() + 6.0, screen.top() + 38.0))
        .default_width(430.0)
        .frame(popover_frame(palette))
        .show(ctx, |ui| {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(150.0);
                    for (index, (title, _)) in sections.iter().enumerate() {
                        let response = menu_button(ui, title, "›", palette);
                        if response.hovered() || response.clicked() {
                            active = index;
                            ctx.data_mut(|data| data.insert_temp(active_id, index));
                        }
                    }
                });
                ui.separator();
                ui.vertical(|ui| {
                    ui.set_width(250.0);
                    for (label, hint, cmd) in &sections[active].1 {
                        if *label == "---" {
                            ui.separator();
                            continue;
                        }
                        if menu_button(ui, label, hint, palette).clicked() {
                            commands.push(cmd.clone());
                            commands.push(Command::CloseAppMenu);
                        }
                    }
                });
            });
        });
    if let Some(inner) = resp
        && ctx.input(|i| i.pointer.any_click())
        && ctx.input(|i| {
            i.pointer
                .interact_pos()
                .is_some_and(|p| !inner.response.rect.contains(p))
        })
    {
        commands.push(Command::CloseAppMenu);
    }
}

/// The compose control in the app bar: new page/chat/meeting note/database.
pub(crate) fn render_compose_menu(
    ctx: &egui::Context,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let screen = ctx.viewport_rect();
    let items: &[(&str, &str, Command)] = &[
        ("New page", "Ctrl+N", Command::NewPage(None)),
        ("New chat", "soon", Command::NoOp),
        ("New meeting note", "soon", Command::NoOp),
        ("New database", "soon", Command::NoOp),
    ];
    let resp = egui::Window::new("compose_menu")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_pos(Pos2::new(screen.left() + 150.0, screen.top() + 42.0))
        .default_width(230.0)
        .frame(popover_frame(palette))
        .show(ctx, |ui| {
            for (label, hint, cmd) in items {
                if menu_button(ui, label, hint, palette).clicked() {
                    commands.push(cmd.clone());
                    commands.push(Command::CloseComposeMenu);
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
        commands.push(Command::CloseComposeMenu);
    }
}
pub(crate) fn render_workspace_menu(
    ctx: &egui::Context,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let screen = ctx.viewport_rect();
    let anchor = ctx
        .data(|data| data.get_temp::<Rect>(egui::Id::unique("sidebar_account_rect")))
        .unwrap_or_else(|| {
            Rect::from_min_size(
                Pos2::new(screen.left() + 10.0, screen.bottom() - 46.0),
                Vec2::new(246.0, 40.0),
            )
        });
    let resp = egui::Window::new("workspace_menu")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .pivot(Align2::LEFT_BOTTOM)
        .fixed_pos(Pos2::new(anchor.left(), anchor.top()))
        .default_width(250.0)
        .frame(
            egui::Frame::NONE
                .fill(palette.content)
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::same(6))
                .stroke(Stroke::new(1.0, palette.border)),
        )
        .show(ctx, |ui| {
            let active_name = view
                .workspaces
                .iter()
                .find(|workspace| workspace.active)
                .map(|workspace| workspace.name.clone())
                .unwrap_or_else(|| "Workspace".to_string());
            ui.horizontal(|ui| {
                let (avatar, _) = ui.allocate_exact_size(Vec2::splat(30.0), Sense::hover());
                ui.painter()
                    .circle_filled(avatar.center(), 13.0, palette.accent);
                ui.painter().text(
                    avatar.center(),
                    Align2::CENTER_CENTER,
                    "N",
                    FontId::proportional(12.0),
                    Color32::WHITE,
                );
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(active_name)
                            .size(13.0)
                            .color(palette.text),
                    );
                    ui.label(
                        egui::RichText::new("owner@workspace.local")
                            .size(10.5)
                            .color(palette.muted),
                    );
                });
            });
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(2.0);
            for (label, page, cmd) in [
                ("Workspace settings", Some("General"), Command::OpenSettings),
                ("Invite members", None, Command::OpenShare),
                ("Add account", Some("People"), Command::OpenSettings),
            ] {
                if menu_button(ui, label, "", palette).clicked() {
                    if let Some(page) = page {
                        commands.push(Command::SetSettingsPage(page.to_string()));
                    }
                    commands.push(cmd);
                    commands.push(Command::CloseWorkspaceMenu);
                }
            }
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(2.0);
            ui.label(
                egui::RichText::new("Workspaces")
                    .size(10.5)
                    .color(palette.muted),
            );
            for workspace in &view.workspaces {
                let hint = if workspace.active { "✓" } else { "" };
                if menu_button(ui, &workspace.name, hint, palette).clicked() {
                    commands.push(Command::SwitchWorkspace(workspace.id.clone()));
                }
            }
            if menu_button(ui, "Create new workspace", "", palette).clicked() {
                commands.push(Command::OpenWorkspaceCreate);
            }
            if menu_button(ui, "Rename workspace", "", palette).clicked() {
                commands.push(Command::OpenWorkspaceRename);
            }
            if view.workspaces.len() > 1
                && menu_button(ui, "Delete workspace", "", palette).clicked()
            {
                commands.push(Command::DeleteWorkspace(view.active_workspace.clone()));
            }
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(2.0);
            if menu_button(ui, "Log out", "", palette).clicked() {
                commands.push(Command::DisconnectServer);
                commands.push(Command::CloseWorkspaceMenu);
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
        commands.push(Command::CloseWorkspaceMenu);
    }
}
/// The Add shortcut / Add section / row-settings popover used by customize
/// sidebar mode (spec-062).
pub(crate) fn render_sidebar_menu(
    ctx: &egui::Context,
    menu: &SidebarMenu,
    ws: &Workspace,
    _view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let screen = ctx.viewport_rect();
    let y = match menu {
        SidebarMenu::Shortcut => screen.top() + 300.0,
        SidebarMenu::Section => screen.top() + 420.0,
        SidebarMenu::RowSettings { .. } => screen.top() + 360.0,
    };
    let query_id = egui::Id::unique(("sidebar_menu_query", menu));
    let resp = egui::Window::new("sidebar_menu")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_pos(Pos2::new(screen.left() + 12.0, y))
        .default_width(260.0)
        .frame(popover_frame(palette))
        .show(ctx, |ui| {
            let title = match menu {
                SidebarMenu::Shortcut => "Add shortcut",
                SidebarMenu::Section => "Add section",
                SidebarMenu::RowSettings { .. } => "Row settings",
            };
            ui.label(
                egui::RichText::new(title)
                    .size(12.0)
                    .strong()
                    .color(palette.text),
            );
            ui.add_space(4.0);
            match menu {
                SidebarMenu::Shortcut => {
                    let mut query: String =
                        ctx.data(|data| data.get_temp(query_id).unwrap_or_default());
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut query)
                                .desired_width(f32::INFINITY)
                                .hint_text("Search shortcuts...")
                                .text_color(palette.text),
                        )
                        .changed()
                    {
                        ctx.data_mut(|data| data.insert_temp(query_id, query.clone()));
                    }
                    let query = query.trim().to_lowercase();
                    let matches =
                        |label: &str| query.is_empty() || label.to_lowercase().contains(&query);
                    if matches("New page")
                        && menu_button(ui, "New page", "Ctrl+N", palette).clicked()
                    {
                        commands.push(Command::NewPage(None));
                        commands.push(Command::CloseSidebarMenu);
                    }
                    for label in ["New chat", "New meeting note", "New database", "New agent"] {
                        if matches(label) {
                            menu_button(ui, label, "soon", palette);
                        }
                    }
                    ui.separator();
                    for (label, id) in [
                        ("Settings", "nav:settings"),
                        ("Library", "nav:library"),
                        ("Inbox", "nav:inbox"),
                    ] {
                        if matches(label) && menu_button(ui, label, "", palette).clicked() {
                            commands.push(Command::AddSidebarShortcut(
                                id.to_string(),
                                label.to_string(),
                            ));
                            commands.push(Command::CloseSidebarMenu);
                        }
                    }
                    ui.separator();
                    ui.label(egui::RichText::new("Pages").size(10.5).color(palette.muted));
                    for page in ws.pages.iter().filter(|page| matches(&page.title)) {
                        if menu_button(ui, &page.title, "", palette).clicked() {
                            commands.push(Command::AddSidebarShortcut(
                                page.id.clone(),
                                page.title.clone(),
                            ));
                            commands.push(Command::CloseSidebarMenu);
                        }
                    }
                }
                SidebarMenu::Section => {
                    let mut query: String =
                        ctx.data(|data| data.get_temp(query_id).unwrap_or_default());
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut query)
                                .desired_width(f32::INFINITY)
                                .hint_text("Search sections...")
                                .text_color(palette.text),
                        )
                        .changed()
                    {
                        ctx.data_mut(|data| data.insert_temp(query_id, query.clone()));
                    }
                    let query = query.trim().to_lowercase();
                    let matches =
                        |label: &str| query.is_empty() || label.to_lowercase().contains(&query);
                    let items: &[(&str, crate::model::SidebarSectionSource)] = &[
                        ("Favorites", crate::model::SidebarSectionSource::Favorites),
                        ("Private", crate::model::SidebarSectionSource::Private),
                        ("Blank section", crate::model::SidebarSectionSource::Custom),
                        ("Recents", crate::model::SidebarSectionSource::Custom),
                        ("Shared", crate::model::SidebarSectionSource::Custom),
                        ("Pinned", crate::model::SidebarSectionSource::Custom),
                        ("Chats", crate::model::SidebarSectionSource::Custom),
                        ("Agents", crate::model::SidebarSectionSource::Custom),
                        (
                            "Upcoming events",
                            crate::model::SidebarSectionSource::Custom,
                        ),
                        (
                            "AI Meeting Notes",
                            crate::model::SidebarSectionSource::Custom,
                        ),
                        ("Developer", crate::model::SidebarSectionSource::Custom),
                    ];
                    for (label, source) in items {
                        if matches(label) && menu_button(ui, label, "", palette).clicked() {
                            commands
                                .push(Command::AddSidebarSection((*label).to_string(), *source));
                            commands.push(Command::CloseSidebarMenu);
                        }
                    }
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("A section is a sidebar tab with its own shortcuts.")
                            .size(10.5)
                            .color(palette.muted),
                    );
                }
                SidebarMenu::RowSettings { section, shortcut } => {
                    let rename_id = egui::Id::unique(("sidebar_rename", section, shortcut));
                    let current = if let Some(shortcut_id) = shortcut {
                        ws.settings
                            .sidebar_layout
                            .sections
                            .iter()
                            .find(|item| &item.id == section)
                            .and_then(|item| {
                                item.shortcuts
                                    .iter()
                                    .find(|item| &item.id == shortcut_id)
                                    .map(|item| item.label.clone())
                            })
                            .unwrap_or_default()
                    } else {
                        ws.settings
                            .sidebar_layout
                            .sections
                            .iter()
                            .find(|item| &item.id == section)
                            .map(|item| item.title.clone())
                            .unwrap_or_default()
                    };
                    let mut rename: String =
                        ctx.data(|data| data.get_temp(rename_id).unwrap_or(current));
                    ui.label(egui::RichText::new("Name").size(10.5).color(palette.muted));
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut rename)
                                .desired_width(f32::INFINITY)
                                .text_color(palette.text),
                        )
                        .changed()
                    {
                        ctx.data_mut(|data| data.insert_temp(rename_id, rename.clone()));
                    }
                    if ui.button("Apply name").clicked() {
                        if let Some(shortcut_id) = shortcut {
                            commands.push(Command::RenameSidebarShortcut(
                                section.clone(),
                                shortcut_id.clone(),
                                rename.clone(),
                            ));
                        } else {
                            commands.push(Command::RenameSidebarSection(
                                section.clone(),
                                rename.clone(),
                            ));
                        }
                        commands.push(Command::CloseSidebarMenu);
                    }
                    ui.separator();
                    let hide_label = if shortcut.is_some() {
                        "Hide / show shortcut"
                    } else {
                        "Hide / show section"
                    };
                    if menu_button(ui, hide_label, "", palette).clicked() {
                        if let Some(shortcut_id) = shortcut {
                            commands.push(Command::ToggleSidebarShortcutHidden(
                                section.clone(),
                                shortcut_id.clone(),
                            ));
                        } else if let Some(nav) = section.strip_prefix("nav:") {
                            commands.push(Command::ToggleSidebarNav(nav.to_string()));
                        } else {
                            commands.push(Command::ToggleSidebarSectionHidden(section.clone()));
                        }
                        commands.push(Command::CloseSidebarMenu);
                    }
                    if let Some(shortcut_id) = shortcut
                        && menu_button(ui, "Remove shortcut", "", palette).clicked()
                    {
                        commands.push(Command::RemoveSidebarShortcut(
                            section.clone(),
                            shortcut_id.clone(),
                        ));
                        commands.push(Command::CloseSidebarMenu);
                    }
                    if shortcut.is_none()
                        && !section.starts_with("nav:")
                        && menu_button(ui, "Remove section", "", palette).clicked()
                    {
                        commands.push(Command::RemoveSidebarSection(section.clone()));
                        commands.push(Command::CloseSidebarMenu);
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
        commands.push(Command::CloseSidebarMenu);
    }
}

pub(crate) fn render_grid_menu(
    ctx: &egui::Context,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let items = vec![
        ("Library", "", Command::OpenLibrary),
        (
            "Skills",
            "",
            Command::SetSettingsPage("AI assistant".into()),
        ),
        ("Trash", "", Command::OpenLibrary),
        ("---", "", Command::NoOp),
        ("Settings", "", Command::OpenSettings),
    ];
    let screen = ctx.viewport_rect();
    let anchor = ctx
        .data(|data| data.get_temp::<Rect>(egui::Id::unique("sidebar_grid_rect")))
        .unwrap_or_else(|| {
            Rect::from_min_size(
                Pos2::new(screen.left() + 234.0, screen.bottom() - 46.0),
                Vec2::new(18.0, 40.0),
            )
        });
    let resp = egui::Window::new("grid_menu")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .pivot(Align2::LEFT_BOTTOM)
        .fixed_pos(Pos2::new(anchor.left(), anchor.top()))
        .default_width(180.0)
        .frame(
            egui::Frame::NONE
                .fill(palette.content)
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::same(6))
                .stroke(Stroke::new(1.0, palette.border)),
        )
        .show(ctx, |ui| {
            for (label, hint, cmd) in &items {
                if *label == "---" {
                    ui.separator();
                    continue;
                }
                if menu_button(ui, label, hint, palette).clicked() {
                    commands.push(cmd.clone());
                    commands.push(Command::CloseGridMenu);
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
        commands.push(Command::CloseGridMenu);
    }
}

/// One entry of the block menu: a label and the kind it turns the block into.
/// Entries without a kind are display-only placeholders for features that are
/// deliberately out of scope (media, embeds, databases).
fn slash_item(ui: &mut egui::Ui, label: &str, hint: &str, palette: &Palette) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 28.0), Sense::click());
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(5), palette.hover);
    }
    ui.painter().text(
        rect.min + Vec2::new(8.0, 14.0),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(13.0),
        palette.text,
    );
    if !hint.is_empty() {
        ui.painter().text(
            rect.max + Vec2::new(-8.0, -14.0),
            Align2::RIGHT_CENTER,
            hint,
            FontId::proportional(11.0),
            palette.muted,
        );
    }
    response
}

fn slash_group(ui: &mut egui::Ui, title: &str, palette: &Palette) {
    ui.add_space(6.0);
    ui.label(egui::RichText::new(title).size(10.5).color(palette.muted));
    ui.add_space(2.0);
}

/// The block menu opened by the "+", the six-dot handle or typing "/".
/// It mirrors the categorized menu in the attachments (Suggested, Basic
/// blocks, Media, Database, Advanced blocks, Inline, Embeds).
/// One block-menu entry: label, shortcut hint and the kind it applies.
type SlashEntry = (&'static str, &'static str, Option<BlockKind>);
/// A titled group of block-menu entries.
type SlashGroup = (&'static str, &'static [SlashEntry]);

#[allow(clippy::too_many_lines)]
pub(crate) fn render_slash(
    ctx: &egui::Context,
    block: &str,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        commands.push(Command::CloseSlash);
        return;
    }
    let query = view.slash_query.trim().to_lowercase();
    let matches = |label: &str| query.is_empty() || label.to_lowercase().contains(&query);
    let groups: &[SlashGroup] = &[
        (
            "Suggested",
            &[
                ("AI Meeting Notes", "", None),
                ("HTML · Embeds", "", None),
                ("AI Image", "", None),
            ],
        ),
        (
            "Basic blocks",
            &[
                ("Text", "", Some(BlockKind::Paragraph)),
                ("Heading 1", "#", Some(BlockKind::Heading1)),
                ("Heading 2", "##", Some(BlockKind::Heading2)),
                ("Heading 3", "###", Some(BlockKind::Heading3)),
                ("Bulleted list", "-", Some(BlockKind::Bulleted)),
                ("Numbered list", "1.", Some(BlockKind::Numbered)),
                ("To-do list", "[]", Some(BlockKind::Todo)),
                ("Quote", "\"", Some(BlockKind::Quote)),
                ("Callout", "", Some(BlockKind::Callout)),
                ("Code", "", Some(BlockKind::Code)),
                ("Divider", "---", Some(BlockKind::Divider)),
            ],
        ),
        (
            "Media",
            &[
                ("Image", "", None),
                ("Video", "", None),
                ("Audio", "", None),
                ("File", "", None),
                ("Web bookmark", "", None),
            ],
        ),
        (
            "Database",
            &[
                ("Table view", "", None),
                ("Board view", "", None),
                ("Gallery view", "", None),
                ("List view", "", None),
            ],
        ),
        (
            "Advanced blocks",
            &[
                ("Table of contents", "", None),
                ("Block equation", "", None),
                ("Button", "", None),
                ("Breadcrumb", "", None),
                ("Tabs", "", None),
                ("Synced block", "", None),
            ],
        ),
        (
            "Inline",
            &[
                ("Mention a person", "", None),
                ("Mention a page", "", None),
                ("Date or reminder", "", None),
                ("Emoji", "", None),
            ],
        ),
        (
            "Embeds",
            &[
                ("Embed", "", None),
                ("Google Drive", "", None),
                ("Tweet", "", None),
            ],
        ),
    ];

    egui::Window::new("slash_menu")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_pos(view.slash_pos)
        .default_width(260.0)
        .frame(popover_frame(palette))
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, true])
                .max_height(330.0)
                .show(ui, |ui| {
                    let mut shown = 0;
                    for (title, items) in groups {
                        let visible: Vec<_> = items
                            .iter()
                            .filter(|(label, _, _)| matches(label))
                            .collect();
                        if visible.is_empty() {
                            continue;
                        }
                        slash_group(ui, title, palette);
                        for (label, hint, kind) in visible {
                            shown += 1;
                            if slash_item(ui, label, hint, palette).clicked()
                                && let Some(kind) = kind
                            {
                                commands.push(Command::SetBlockKind(block.to_string(), *kind));
                            }
                        }
                    }
                    if shown == 0 {
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new(format!("No blocks match “{query}”"))
                                .size(12.0)
                                .color(palette.muted),
                        );
                    }
                });
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(4.0);
            if slash_item(ui, "Delete block", "", palette).clicked() {
                commands.push(Command::DeleteBlock(block.to_string()));
                commands.push(Command::CloseSlash);
            }
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Type to filter…")
                        .size(11.0)
                        .color(palette.muted),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Close menu  esc")
                                    .size(10.5)
                                    .color(palette.muted),
                            )
                            .frame(false),
                        )
                        .clicked()
                    {
                        commands.push(Command::CloseSlash);
                    }
                });
            });
        });
}
