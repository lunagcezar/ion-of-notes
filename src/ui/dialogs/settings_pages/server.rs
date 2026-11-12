//! Settings page: server.

use super::super::super::commands::Command;
use super::super::super::palette::{Palette, strong_font};
use super::super::super::view::ViewState;
use crate::ui::constants as metrics;
pub(in crate::ui::dialogs) fn render_server_settings(
    ui: &mut egui::Ui,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    ui.label(
        egui::RichText::new("Self-hosted server")
            .font(strong_font(metrics::FONT_SUBTITLE))
            .color(palette.text),
    );
    ui.add_space(metrics::SPACE_6);
    ui.label(
        egui::RichText::new(
            "Connect to a running --serve instance. The password is sent only to that server and is not stored.",
        )
        .size(metrics::FONT_CAPTION)
        .color(palette.muted),
    );
    ui.add_space(metrics::SPACE_14);

    ui.label(
        egui::RichText::new("Server URL")
            .size(metrics::FONT_CAPTION)
            .color(palette.text),
    );
    let mut url = view.server_url.clone();
    if ui
        .add(
            egui::TextEdit::singleline(&mut url)
                .desired_width(360.0)
                .hint_text("http://127.0.0.1:8787")
                .text_color(palette.text),
        )
        .changed()
    {
        commands.push(Command::SetServerUrl(url));
    }
    ui.add_space(metrics::SPACE_10);

    ui.label(
        egui::RichText::new("Email")
            .size(metrics::FONT_CAPTION)
            .color(palette.text),
    );
    let mut email = view.server_email.clone();
    if ui
        .add(
            egui::TextEdit::singleline(&mut email)
                .desired_width(360.0)
                .hint_text("owner@workspace.local")
                .text_color(palette.text),
        )
        .changed()
    {
        commands.push(Command::SetServerEmail(email));
    }
    ui.add_space(metrics::SPACE_10);

    ui.label(
        egui::RichText::new("Password")
            .size(metrics::FONT_CAPTION)
            .color(palette.text),
    );
    let mut password = view.server_password.clone();
    if ui
        .add(
            egui::TextEdit::singleline(&mut password)
                .desired_width(360.0)
                .password(true)
                .text_color(palette.text),
        )
        .changed()
    {
        commands.push(Command::SetServerPassword(password));
    }
    ui.add_space(metrics::SPACE_16);

    if view.server_connected {
        ui.label(
            egui::RichText::new("Connected")
                .size(metrics::FONT_CAPTION)
                .color(palette.accent),
        );
        if ui.button("Disconnect and use local workspace").clicked() {
            commands.push(Command::DisconnectServer);
        }
    } else {
        let label = if view.server_busy {
            "Connecting..."
        } else {
            "Connect"
        };
        if ui
            .add_enabled(!view.server_busy, egui::Button::new(label))
            .clicked()
        {
            commands.push(Command::ConnectServer);
        }
    }

    if let Some(status) = &view.server_status {
        ui.add_space(metrics::SPACE_10);
        ui.label(
            egui::RichText::new(status)
                .size(metrics::FONT_CAPTION)
                .color(palette.muted),
        );
    }

    ui.add_space(metrics::SECTION_GAP);
    ui.label(
        egui::RichText::new(
            "Tip: start the server with `--serve 127.0.0.1:8787 --database data/ion-of-notes.sqlite3`.",
        )
        .size(metrics::FONT_MICRO)
        .color(palette.muted),
    );
}
