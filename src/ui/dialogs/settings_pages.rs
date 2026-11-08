//! Settings pages shown in the right-hand pane of the settings window.

use crate::model::Workspace;
use egui::{Align2, Color32, CornerRadius, FontId, Margin, Pos2, Rect, Sense, Stroke, Vec2};

use super::super::commands::Command;
use super::super::palette::{Palette, strong_font};
use super::super::view::ViewState;
use super::settings::display_row;

pub(super) fn render_preferences(
    ui: &mut egui::Ui,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    ui.label(
        egui::RichText::new("Appearance")
            .font(strong_font(17.0))
            .color(palette.text),
    );
    ui.add_space(12.0);
    ui.separator();
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Theme").color(palette.text));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let label = if view.dark { "Dark" } else { "Light" };
            if ui.add(egui::Button::new(label)).clicked() {
                commands.push(Command::ToggleTheme);
            }
        });
    });
    ui.add_space(6.0);
    ui.label(
        egui::RichText::new("Choose a theme for this device (Ctrl+Shift+L)")
            .size(12.0)
            .color(palette.muted),
    );
    ui.add_space(10.0);
    display_row(
        ui,
        "High contrast  Beta",
        "Increase contrast for improved visibility",
        "Use system setting ⌄",
        palette,
    );

    ui.add_space(26.0);
    ui.label(
        egui::RichText::new("Input options")
            .font(strong_font(17.0))
            .color(palette.text),
    );
    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    display_row(
        ui,
        "Use Enter to add a new line",
        "Applies to chat and comments. Press Ctrl+Enter to send.",
        "Off",
        palette,
    );

    ui.add_space(26.0);
    ui.label(
        egui::RichText::new("Language & time")
            .font(strong_font(17.0))
            .color(palette.text),
    );
    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    let (lang_rect, lang_resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 46.0), Sense::click());
    if lang_resp.hovered() {
        ui.painter()
            .rect_filled(lang_rect, CornerRadius::same(6), palette.hover);
    }
    ui.painter().text(
        lang_rect.min + Vec2::new(0.0, 14.0),
        Align2::LEFT_CENTER,
        rust_i18n::t!("language"),
        FontId::proportional(12.5),
        palette.text,
    );
    ui.painter().text(
        lang_rect.min + Vec2::new(0.0, 32.0),
        Align2::LEFT_CENTER,
        rust_i18n::t!("language_description"),
        FontId::proportional(11.0),
        palette.muted,
    );
    let lang_label = crate::i18n::label_for(&view.language);
    let chip = Rect::from_min_size(
        Pos2::new(lang_rect.max.x - 170.0, lang_rect.min.y + 9.0),
        Vec2::new(170.0, 26.0),
    );
    ui.painter().rect_stroke(
        chip,
        CornerRadius::same(6),
        Stroke::new(1.0, palette.border),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        chip.center(),
        Align2::CENTER_CENTER,
        format!("{lang_label} ⌄"),
        FontId::proportional(12.0),
        palette.text,
    );
    if lang_resp.clicked() {
        let codes: Vec<&str> = crate::i18n::SUPPORTED.iter().map(|(c, _)| *c).collect();
        let idx = codes.iter().position(|c| *c == view.language).unwrap_or(0);
        let next = codes[(idx + 1) % codes.len()];
        commands.push(Command::SetLanguage(next.to_string()));
    }
    display_row(
        ui,
        "Number format",
        "Choose how numbers and currencies are formatted",
        "1.000.000,00 ⌄",
        palette,
    );
    display_row(
        ui,
        "Time format",
        "Choose 12-hour or 24-hour time display",
        "24-hour",
        palette,
    );

    ui.add_space(26.0);
    ui.label(
        egui::RichText::new("Workspace & sync")
            .font(strong_font(17.0))
            .color(palette.text),
    );
    ui.add_space(12.0);
    ui.separator();
    ui.add_space(12.0);
    ui.label(
        egui::RichText::new("Self-hosted server  ·  connected")
            .size(12.0)
            .color(palette.accent),
    );
    ui.add_space(6.0);
    ui.label(
        egui::RichText::new(
            "Documents, members and permissions live on your server and sync to every signed-in device.",
        )
        .size(12.0)
        .color(palette.muted),
    );
    ui.add_space(14.0);
    for (label, value) in [
        ("Accounts", "1 owner · 2 guests"),
        ("Sharing", "Invite by email · full / edit / view"),
        ("Sync", "Versioned changes across devices"),
        ("Real-time", "Live co-editing over WebSocket"),
        ("Export", "Markdown tree · People CSV"),
    ] {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(label)
                    .size(12.0)
                    .strong()
                    .color(palette.text),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new(value).size(11.0).color(palette.muted));
            });
        });
        ui.add_space(6.0);
    }

    ui.add_space(18.0);
    ui.separator();
    ui.add_space(12.0);
    ui.label(
        egui::RichText::new(
            "Open source under the MIT license. Run your own server; your data stays yours.",
        )
        .size(12.0)
        .color(palette.muted),
    );
    ui.add_space(12.0);
    if ui
        .add(egui::Button::new("Restore demo workspace"))
        .clicked()
    {
        commands.push(Command::ResetWorkspace);
    }
}

pub(super) fn render_workspace_settings(
    ui: &mut egui::Ui,
    ws: &Workspace,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    ui.label(
        egui::RichText::new("Workspace")
            .font(strong_font(17.0))
            .color(palette.text),
    );
    ui.add_space(12.0);
    ui.separator();
    ui.add_space(12.0);
    display_row(
        ui,
        "Workspace name",
        "Shown in the sidebar and exports",
        &ws.name,
        palette,
    );
    display_row(
        ui,
        "Workspace icon",
        "Customize the workspace identity",
        "Default",
        palette,
    );
    ui.add_space(18.0);
    ui.label(
        egui::RichText::new("Members & access")
            .font(strong_font(17.0))
            .color(palette.text),
    );
    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);
    display_row(
        ui,
        "Members",
        "Manage who can access this workspace",
        "Manage",
        palette,
    );
    if ui.add(egui::Button::new("Invite members")).clicked() {
        commands.push(Command::OpenShare);
    }
    ui.add_space(18.0);
    ui.label(
        egui::RichText::new("Data & export")
            .font(strong_font(17.0))
            .color(palette.text),
    );
    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new("Export includes pages, version history metadata, and People.")
            .size(12.0)
            .color(palette.muted),
    );
    let _ = (view, commands);
}

pub(super) fn render_sidebar_settings(
    ui: &mut egui::Ui,
    _view: &ViewState,
    palette: &Palette,
    _commands: &mut Vec<Command>,
) {
    ui.label(
        egui::RichText::new("Customize sidebar")
            .font(strong_font(17.0))
            .color(palette.text),
    );
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new("Choose what appears in the sidebar. Drag-and-drop ordering is planned; toggles are available here.")
            .size(12.0)
            .color(palette.muted),
    );
    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    for item in [
        "Search",
        "Library",
        "Settings",
        "Inbox",
        "Favorites",
        "Private pages",
        "Customize sidebar",
    ] {
        display_row(ui, item, "Show in sidebar", "Visible", palette);
    }
}

pub(super) fn render_offline_settings(
    ui: &mut egui::Ui,
    ws: &Workspace,
    view: &ViewState,
    palette: &Palette,
    _commands: &mut Vec<Command>,
) {
    ui.label(
        egui::RichText::new("Offline pages")
            .font(strong_font(17.0))
            .color(palette.text),
    );
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(
            "Pages marked \"Available offline\" are kept for offline access and listed here.",
        )
        .size(12.0)
        .color(palette.muted),
    );
    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    let mut found = false;
    for page in ws.pages.iter().filter(|p| p.available_offline) {
        found = true;
        ui.label(egui::RichText::new(format!("• {}", page.title)).color(palette.text));
    }
    if !found {
        ui.label(
            egui::RichText::new("No offline pages yet. Use Page actions → Available offline.")
                .size(12.0)
                .color(palette.muted),
        );
    }
    let _ = view;
}

pub(super) fn render_people_settings(
    ui: &mut egui::Ui,
    _ws: &Workspace,
    _view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    ui.label(
        egui::RichText::new("People")
            .font(strong_font(17.0))
            .color(palette.text),
    );
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new("Members and guests are managed by your self-hosted workspace server.")
            .size(12.0)
            .color(palette.muted),
    );
    ui.add_space(12.0);
    if ui.add(egui::Button::new("Invite members")).clicked() {
        commands.push(Command::OpenShare);
    }
}

pub(super) fn render_ai_settings(
    ui: &mut egui::Ui,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    ui.label(
        egui::RichText::new("Connection")
            .font(strong_font(17.0))
            .color(palette.text),
    );
    ui.add_space(10.0);
    ui.separator();
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        let dot_color = if view.ai_connected {
            palette.accent
        } else {
            palette.muted
        };
        let (dot, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
        ui.painter().circle_filled(dot.center(), 4.0, dot_color);
        ui.label(
            egui::RichText::new(&view.ai_status)
                .size(12.0)
                .color(if view.ai_connected {
                    palette.text
                } else {
                    palette.muted
                }),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.add(egui::Button::new("Refresh")).clicked() {
                commands.push(Command::RefreshAiStatus);
            }
        });
    });
    ui.add_space(18.0);

    ui.label(
        egui::RichText::new("Service endpoint (host:port on your network)")
            .size(12.0)
            .strong()
            .color(palette.text),
    );
    ui.add_space(6.0);
    let mut endpoint = view.ai_endpoint.clone();
    if ui
        .add(
            egui::TextEdit::singleline(&mut endpoint)
                .desired_width(f32::INFINITY)
                .text_color(palette.text),
        )
        .changed()
    {
        commands.push(Command::SetAiEndpoint(endpoint));
    }
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(
            "Start Ollama (`ollama serve`) and point this at its port, or leave the default loopback. OLLAMA_HOST also works.",
        )
        .size(11.0)
        .color(palette.muted),
    );
    ui.add_space(16.0);

    ui.label(
        egui::RichText::new("Model")
            .size(12.0)
            .strong()
            .color(palette.text),
    );
    ui.add_space(6.0);
    let mut model = view.ai_model.clone();
    if ui
        .add(
            egui::TextEdit::singleline(&mut model)
                .desired_width(f32::INFINITY)
                .text_color(palette.text),
        )
        .changed()
    {
        commands.push(Command::SetAiModel(model));
    }
    ui.add_space(16.0);

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Prefer the ollama CLI when it is in PATH")
                .size(12.0)
                .color(palette.text),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let label = if view.ai_prefer_cli { "On" } else { "Off" };
            if ui.add(egui::Button::new(label)).clicked() {
                commands.push(Command::ToggleAiCli);
            }
        });
    });
    ui.add_space(22.0);

    ui.label(
        egui::RichText::new("Try it")
            .font(strong_font(17.0))
            .color(palette.text),
    );
    ui.add_space(10.0);
    ui.separator();
    ui.add_space(12.0);
    if ui.add(egui::Button::new("Ask a test question")).clicked() {
        commands.push(Command::AskAi(
            "Say hello in one short sentence.".to_string(),
        ));
    }
    ui.add_space(10.0);
    if view.ai_busy {
        ui.label(
            egui::RichText::new("Waiting for Ollama…")
                .size(12.0)
                .color(palette.accent),
        );
    } else if let Some(answer) = &view.ai_answer {
        egui::Frame::NONE
            .fill(palette.callout)
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::same(10))
            .show(ui, |ui| {
                ui.label(egui::RichText::new(answer).size(12.5).color(palette.text));
            });
    } else if let Some(error) = &view.ai_error {
        ui.label(
            egui::RichText::new(error)
                .size(11.5)
                .color(Color32::from_rgb(224, 108, 108)),
        );
    }
    ui.add_space(18.0);
    ui.label(
        egui::RichText::new(
            "Shortcut: Shift+Ctrl+J opens the assistant. Ask anything from Search or ask.",
        )
        .size(11.0)
        .color(palette.muted),
    );
}
