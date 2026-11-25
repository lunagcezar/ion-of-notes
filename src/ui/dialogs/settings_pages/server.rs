//! Settings page: server.

use super::super::super::commands::Command;
use super::super::super::palette::{Palette, strong_font};
use super::super::super::view::ServerView;
use crate::ui::constants as metrics;
pub(in crate::ui::dialogs) fn render_server_settings(
    ui: &mut egui::Ui,
    server: &ServerView,
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
    let mut url = server.url.clone();
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
    let mut email = server.email.clone();
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
    let mut password = server.password.clone();
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

    if !server.connected {
        let mut register = server.register;
        if ui
            .checkbox(&mut register, "Create an account for an invited email")
            .changed()
        {
            commands.push(Command::SetServerRegister(register));
        }
        if server.register {
            let mut token = server.invite_token.clone();
            if ui
                .add(
                    egui::TextEdit::singleline(&mut token)
                        .desired_width(360.0)
                        .hint_text("Invitation token (inv_...)")
                        .text_color(palette.text),
                )
                .changed()
            {
                commands.push(Command::SetServerInviteToken(token));
            }
        }
    }

    if server.connected {
        ui.add_space(metrics::SECTION_GAP);
        ui.label(
            egui::RichText::new("Change password")
                .size(metrics::FONT_CAPTION)
                .color(palette.text),
        );
        let mut current = server.current_password.clone();
        if ui
            .add(
                egui::TextEdit::singleline(&mut current)
                    .desired_width(360.0)
                    .password(true)
                    .hint_text("Current password")
                    .text_color(palette.text),
            )
            .changed()
        {
            commands.push(Command::SetServerCurrentPassword(current));
        }
        let mut new_password = server.new_password.clone();
        if ui
            .add(
                egui::TextEdit::singleline(&mut new_password)
                    .desired_width(360.0)
                    .password(true)
                    .hint_text("New password (8+ characters)")
                    .text_color(palette.text),
            )
            .changed()
        {
            commands.push(Command::SetServerNewPassword(new_password));
        }
        let mut confirm = server.confirm_password.clone();
        if ui
            .add(
                egui::TextEdit::singleline(&mut confirm)
                    .desired_width(360.0)
                    .password(true)
                    .hint_text("Repeat the new password")
                    .text_color(palette.text),
            )
            .changed()
        {
            commands.push(Command::SetServerConfirmPassword(confirm));
        }
        if ui.button("Change password").clicked() {
            commands.push(Command::ChangeServerPassword);
        }

        ui.add_space(metrics::SECTION_GAP);
        ui.label(
            egui::RichText::new("Reset a member password (owner)")
                .size(metrics::FONT_CAPTION)
                .color(palette.text),
        );
        let mut reset_email = server.reset_email.clone();
        if ui
            .add(
                egui::TextEdit::singleline(&mut reset_email)
                    .desired_width(360.0)
                    .hint_text("member@workspace.local")
                    .text_color(palette.text),
            )
            .changed()
        {
            commands.push(Command::SetServerResetEmail(reset_email));
        }
        let mut reset_password = server.reset_password.clone();
        if ui
            .add(
                egui::TextEdit::singleline(&mut reset_password)
                    .desired_width(360.0)
                    .password(true)
                    .hint_text("New password (8+ characters)")
                    .text_color(palette.text),
            )
            .changed()
        {
            commands.push(Command::SetServerResetPassword(reset_password));
        }
        if ui.button("Reset member password").clicked() {
            commands.push(Command::ResetServerPassword);
        }
    }

    if server.connected {
        ui.label(
            egui::RichText::new("Connected")
                .size(metrics::FONT_CAPTION)
                .color(palette.accent),
        );
        if ui.button("Disconnect and use local workspace").clicked() {
            commands.push(Command::DisconnectServer);
        }
    } else {
        let label = if server.busy {
            "Connecting..."
        } else {
            "Connect"
        };
        if ui
            .add_enabled(!server.busy, egui::Button::new(label))
            .clicked()
        {
            commands.push(Command::ConnectServer);
        }
    }

    if let Some(status) = &server.status {
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
