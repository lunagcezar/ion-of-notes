// ---- App / workspace / grid menus -----------------------------------------

//! App, workspace, grid and slash menus.

use egui::{Align2, Color32, CornerRadius, FontId, Margin, Pos2, Sense, Stroke, Vec2};

use super::super::commands::{Command, SidebarMenu};
use super::super::palette::Palette;
use super::super::view::ViewState;
use super::super::widgets::{menu_button, popover_frame};
use crate::model::{BlockKind, Workspace};

#[allow(clippy::too_many_arguments)]
fn popup_menu(
    ctx: &egui::Context,
    id: &str,
    pos: Pos2,
    width: f32,
    palette: &Palette,
    items: &[(&str, &str, Command)],
    commands: &mut Vec<Command>,
    close: Command,
) {
    let resp = egui::Window::new(id)
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_pos(pos)
        .default_width(width)
        .frame(
            egui::Frame::NONE
                .fill(palette.content)
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::same(6))
                .stroke(Stroke::new(1.0, palette.border)),
        )
        .show(ctx, |ui| {
            for (label, hint, cmd) in items {
                if *label == "---" {
                    ui.separator();
                    continue;
                }
                let r = menu_button(ui, label, hint, palette);
                if r.clicked() {
                    commands.push(cmd.clone());
                    commands.push(close.clone());
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
        commands.push(close);
    }
}

pub(crate) fn render_app_menu(ctx: &egui::Context, palette: &Palette, commands: &mut Vec<Command>) {
    let screen = ctx.viewport_rect();
    let items = vec![
        ("File", "", Command::NoOp),
        ("  New page", "Ctrl+N", Command::NewPage(None)),
        ("  New tab", "Ctrl+T", Command::OpenNewTab),
        ("  Close tab", "Ctrl+W", Command::CloseTab(usize::MAX)),
        ("---", "", Command::NoOp),
        ("Edit", "", Command::NoOp),
        ("  Undo", "Ctrl+Z", Command::Undo),
        ("  Redo", "Ctrl+Shift+Z", Command::Redo),
        ("  Copy link", "Ctrl+L", Command::CopyLink),
        ("---", "", Command::NoOp),
        ("View", "", Command::NoOp),
        ("  Toggle sidebar", "Ctrl+\\", Command::ToggleSidebar),
        ("  Zoom in", "Ctrl++", Command::ZoomIn),
        ("  Zoom out", "Ctrl+-", Command::ZoomOut),
        ("  Full screen", "F11", Command::ToggleFullscreen),
        ("---", "", Command::NoOp),
        ("History", "", Command::NoOp),
        ("  Back", "Ctrl+[", Command::GoBack),
        ("  Forward", "Ctrl+]", Command::GoForward),
        ("---", "", Command::NoOp),
        ("Window", "", Command::NoOp),
        ("  Minimize", "", Command::WindowMinimize),
        ("  Next tab", "Ctrl+Tab", Command::NextTab),
        ("---", "", Command::NoOp),
        ("Help", "", Command::NoOp),
        ("  Keyboard shortcuts", "F1", Command::OpenShortcuts),
        ("  Settings", "Ctrl+,", Command::OpenSettings),
    ];
    popup_menu(
        ctx,
        "app_menu",
        Pos2::new(screen.left() + 38.0, screen.top() + 42.0),
        260.0,
        palette,
        &items,
        commands,
        Command::CloseAppMenu,
    );
}

pub(crate) fn render_workspace_menu(
    ctx: &egui::Context,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let screen = ctx.viewport_rect();
    let resp = egui::Window::new("workspace_menu")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_pos(Pos2::new(screen.left() + 12.0, screen.bottom() - 336.0))
        .default_width(250.0)
        .frame(
            egui::Frame::NONE
                .fill(palette.content)
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::same(6))
                .stroke(Stroke::new(1.0, palette.border)),
        )
        .show(ctx, |ui| {
            // Account header.
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
                        egui::RichText::new("Local workspace")
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
            if menu_button(ui, "Local workspace", "✓", palette).clicked() {
                commands.push(Command::CloseWorkspaceMenu);
            }
            if menu_button(ui, "Create new workspace", "", palette).clicked() {
                commands.push(Command::OpenOnboarding);
                commands.push(Command::CloseWorkspaceMenu);
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

/// The Add shortcut / Add section popover used by customize-sidebar mode.
pub(crate) fn render_sidebar_menu(
    ctx: &egui::Context,
    menu: SidebarMenu,
    ws: &Workspace,
    _view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let screen = ctx.viewport_rect();
    let y = match menu {
        SidebarMenu::Shortcut => screen.top() + 300.0,
        SidebarMenu::Section => screen.top() + 470.0,
    };
    let resp = egui::Window::new("sidebar_menu")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_pos(Pos2::new(screen.left() + 12.0, y))
        .default_width(250.0)
        .frame(popover_frame(palette))
        .show(ctx, |ui| {
            let title = match menu {
                SidebarMenu::Shortcut => "Add shortcut",
                SidebarMenu::Section => "Add section",
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
                    for page in ws.pages.iter().take(10) {
                        let hint = if page.favorite { "✓" } else { "" };
                        if menu_button(ui, &page.title, hint, palette).clicked() {
                            commands.push(Command::ToggleFavorite(page.id.clone()));
                            commands.push(Command::CloseSidebarMenu);
                        }
                    }
                    ui.separator();
                    if menu_button(ui, "New page", "", palette).clicked() {
                        commands.push(Command::NewPage(None));
                        commands.push(Command::CloseSidebarMenu);
                    }
                }
                SidebarMenu::Section => {
                    for label in ["Favorites", "Private", "Shared"] {
                        if menu_button(ui, label, "", palette).clicked() {
                            commands.push(Command::CloseSidebarMenu);
                        }
                    }
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("Sections group shortcuts in the sidebar.")
                            .size(10.5)
                            .color(palette.muted),
                    );
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
    let screen = ctx.viewport_rect();
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
    popup_menu(
        ctx,
        "grid_menu",
        Pos2::new(screen.left() + 220.0, screen.bottom() - 220.0),
        180.0,
        palette,
        &items,
        commands,
        Command::CloseGridMenu,
    );
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
