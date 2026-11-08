// ---- App / workspace / grid menus -----------------------------------------

//! App, workspace, grid and slash menus.

use egui::{Align2, CornerRadius, FontId, Margin, Pos2, Sense, Stroke, Vec2};

use super::super::commands::Command;
use super::super::palette::Palette;
use super::super::view::ViewState;
use super::super::widgets::{menu_button, popover_frame};

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
        .fixed_pos(Pos2::new(screen.left() + 12.0, screen.bottom() - 250.0))
        .default_width(240.0)
        .frame(
            egui::Frame::NONE
                .fill(palette.content)
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::same(6))
                .stroke(Stroke::new(1.0, palette.border)),
        )
        .show(ctx, |ui| {
            let items: &[(&str, Command, Option<&str>)] = &[
                ("Workspace settings", Command::OpenSettings, Some("General")),
                ("Invite members", Command::OpenShare, None),
                ("Add account", Command::OpenSettings, Some("People")),
                ("---", Command::NoOp, None),
                ("Switch workspace", Command::OpenLibrary, None),
                (
                    "Create new workspace",
                    Command::OpenSettings,
                    Some("General"),
                ),
            ];
            for (label, cmd, page) in items {
                if *label == "---" {
                    ui.separator();
                    continue;
                }
                let r = menu_button(ui, label, "", palette);
                if r.clicked() {
                    if let Some(p) = page {
                        commands.push(Command::SetSettingsPage((*p).to_string()));
                    }
                    commands.push(cmd.clone());
                    commands.push(Command::CloseWorkspaceMenu);
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
        commands.push(Command::CloseWorkspaceMenu);
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
#[allow(clippy::too_many_lines)]
pub(crate) fn render_slash(
    ctx: &egui::Context,
    block: &str,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    use crate::model::BlockKind;
    egui::Window::new("slash_menu")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_pos(view.slash_pos)
        .default_width(260.0)
        .frame(popover_frame(palette))
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(330.0)
                .show(ui, |ui| {
                    let apply = |commands: &mut Vec<Command>, kind: BlockKind| {
                        commands.push(Command::SetBlockKind(block.to_string(), kind));
                    };
                    slash_group(ui, "Suggested", palette);
                    for label in ["AI Meeting Notes", "HTML · Embeds", "AI Image"] {
                        slash_item(ui, label, "", palette);
                    }
                    slash_group(ui, "Basic blocks", palette);
                    for (label, hint, kind) in [
                        ("Text", "", BlockKind::Paragraph),
                        ("Heading 1", "#", BlockKind::Heading1),
                        ("Heading 2", "##", BlockKind::Heading2),
                        ("Heading 3", "###", BlockKind::Heading3),
                        ("Bulleted list", "-", BlockKind::Bulleted),
                        ("Numbered list", "1.", BlockKind::Numbered),
                        ("To-do list", "[]", BlockKind::Todo),
                        ("Quote", "\"", BlockKind::Quote),
                        ("Callout", "", BlockKind::Callout),
                        ("Code", "", BlockKind::Code),
                        ("Divider", "---", BlockKind::Divider),
                    ] {
                        if slash_item(ui, label, hint, palette).clicked() {
                            apply(commands, kind);
                        }
                    }
                    slash_group(ui, "Media", palette);
                    for label in ["Image", "Video", "Audio", "File", "Web bookmark"] {
                        slash_item(ui, label, "", palette);
                    }
                    slash_group(ui, "Database", palette);
                    for label in ["Table view", "Board view", "Gallery view", "List view"] {
                        slash_item(ui, label, "", palette);
                    }
                    slash_group(ui, "Advanced blocks", palette);
                    for label in [
                        "Table of contents",
                        "Block equation",
                        "Button",
                        "Breadcrumb",
                        "Tabs",
                        "Synced block",
                    ] {
                        slash_item(ui, label, "", palette);
                    }
                    slash_group(ui, "Inline", palette);
                    for label in [
                        "Mention a person",
                        "Mention a page",
                        "Date or reminder",
                        "Emoji",
                    ] {
                        slash_item(ui, label, "", palette);
                    }
                    slash_group(ui, "Embeds", palette);
                    for label in ["Embed", "Google Drive", "Tweet"] {
                        slash_item(ui, label, "", palette);
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
                    ui.label(
                        egui::RichText::new("Close menu  esc")
                            .size(10.5)
                            .color(palette.muted),
                    );
                });
            });
        });
}
