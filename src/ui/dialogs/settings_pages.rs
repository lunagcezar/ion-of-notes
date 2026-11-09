//! Settings pages shown in the right-hand pane of the settings window.
//!
//! Each page renders the rows selected by `visible_rows`, so a settings query
//! filters the rows inside the open section as well as the navigation rail
//! (spec-016 A9).

use crate::i18n;
use crate::model::Workspace;
use egui::{Align2, Color32, CornerRadius, FontId, Margin, Pos2, Rect, Sense, Stroke, Vec2};

use super::super::commands::Command;
use super::super::palette::{Palette, strong_font};
use super::super::settings_search::SettingsRow;
use super::super::view::ViewState;
use super::super::widgets::toggle_button;
use super::settings::display_row;

/// A section heading inside a settings page. Rows sharing a section are
/// grouped under one heading, so filtering never leaves an empty heading.
fn section_heading(ui: &mut egui::Ui, title: &str, palette: &Palette) {
    ui.add_space(20.0);
    ui.label(
        egui::RichText::new(title)
            .font(strong_font(17.0))
            .color(palette.text),
    );
    ui.add_space(10.0);
    ui.separator();
    ui.add_space(10.0);
}

/// A compact label/value row used by the workspace summary.
fn info_row(ui: &mut egui::Ui, label: &str, value: &str, palette: &Palette) {
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

pub(super) fn render_preferences(
    ui: &mut egui::Ui,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
    rows: &[&SettingsRow],
) {
    let mut section = "";
    for row in rows {
        if row.section != section {
            section = row.section;
            section_heading(ui, section, palette);
        }
        match row.id {
            "theme" => {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(row.title).color(palette.text));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let label = if view.dark { "Dark" } else { "Light" };
                        if ui.add(egui::Button::new(label)).clicked() {
                            commands.push(Command::ToggleTheme);
                        }
                    });
                });
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(row.description)
                        .size(12.0)
                        .color(palette.muted),
                );
                ui.add_space(10.0);
            }
            "high_contrast" => display_row(
                ui,
                row.title,
                row.description,
                "Use system setting ⌄",
                palette,
            ),
            "enter_new_line" => display_row(ui, row.title, row.description, "Off", palette),
            "language" => {
                let (lang_rect, lang_resp) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), 46.0), Sense::click());
                if lang_resp.hovered() {
                    ui.painter()
                        .rect_filled(lang_rect, CornerRadius::same(6), palette.hover);
                }
                ui.painter().text(
                    lang_rect.min + Vec2::new(0.0, 14.0),
                    Align2::LEFT_CENTER,
                    i18n::t("language"),
                    FontId::proportional(12.5),
                    palette.text,
                );
                ui.painter().text(
                    lang_rect.min + Vec2::new(0.0, 32.0),
                    Align2::LEFT_CENTER,
                    i18n::t("language_description"),
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
            }
            "number_format" => {
                display_row(ui, row.title, row.description, "1.000.000,00 ⌄", palette)
            }
            "time_format" => display_row(ui, row.title, row.description, "24-hour", palette),
            "server" => {
                ui.label(
                    egui::RichText::new("Self-hosted server  ·  connected")
                        .size(12.0)
                        .color(palette.accent),
                );
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(row.description)
                        .size(12.0)
                        .color(palette.muted),
                );
                ui.add_space(10.0);
            }
            "accounts" => info_row(ui, row.title, "1 owner · 2 guests", palette),
            "sharing" => info_row(
                ui,
                row.title,
                "Invite by email · full / edit / view",
                palette,
            ),
            "sync" => info_row(ui, row.title, "Versioned changes across devices", palette),
            "realtime" => info_row(ui, row.title, "Live co-editing over WebSocket", palette),
            "export" => info_row(ui, row.title, "Markdown tree · People CSV", palette),
            "license" => {
                ui.label(
                    egui::RichText::new(
                        "Open source under the MIT license. Run your own server; your data stays yours.",
                    )
                    .size(12.0)
                    .color(palette.muted),
                );
                ui.add_space(12.0);
            }
            "restore_demo" => {
                if ui
                    .add(egui::Button::new("Restore demo workspace"))
                    .clicked()
                {
                    commands.push(Command::ResetWorkspace);
                }
                ui.add_space(10.0);
            }
            _ => {}
        }
    }
}

pub(super) fn render_workspace_settings(
    ui: &mut egui::Ui,
    ws: &mut Workspace,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
    rows: &[&SettingsRow],
) {
    let mut section = "";
    for row in rows {
        if row.section != section {
            section = row.section;
            section_heading(ui, section, palette);
        }
        match row.id {
            "workspace_name" => {
                ui.label(
                    egui::RichText::new(row.title)
                        .size(12.0)
                        .color(palette.text),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut ws.name)
                        .desired_width(320.0)
                        .text_color(palette.text),
                );
                ui.add_space(8.0);
            }
            "icon" => {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(row.title)
                            .size(12.0)
                            .color(palette.text),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut ws.settings.icon)
                            .desired_width(48.0)
                            .text_color(palette.text),
                    );
                });
                ui.add_space(8.0);
            }
            "landing_page" => {
                ui.label(
                    egui::RichText::new(row.title)
                        .size(12.0)
                        .color(palette.text),
                );
                let pages: Vec<(String, String)> = ws
                    .pages
                    .iter()
                    .map(|page| (page.id.clone(), page.title.clone()))
                    .collect();
                let first_page = i18n::t("first_page");
                let selected = ws
                    .settings
                    .landing_page
                    .as_deref()
                    .and_then(|id| {
                        pages
                            .iter()
                            .find(|(page_id, _)| page_id == id)
                            .map(|(_, title)| title.as_str())
                    })
                    .unwrap_or(&first_page);
                egui::ComboBox::from_id_salt("landing_page")
                    .selected_text(selected)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut ws.settings.landing_page, None, &first_page);
                        for (id, title) in &pages {
                            ui.selectable_value(
                                &mut ws.settings.landing_page,
                                Some(id.clone()),
                                title,
                            );
                        }
                    });
                ui.add_space(8.0);
            }
            "allowed_domains" => {
                let input_id = egui::Id::unique("allowed_domain_input");
                let mut domain: String = ui
                    .ctx()
                    .data(|data| data.get_temp(input_id).unwrap_or_default());
                ui.horizontal(|ui| {
                    let edit = ui.add(
                        egui::TextEdit::singleline(&mut domain)
                            .desired_width(260.0)
                            .hint_text("Type an email domain...")
                            .text_color(palette.text),
                    );
                    if edit.changed() {
                        ui.ctx()
                            .data_mut(|data| data.insert_temp(input_id, domain.clone()));
                    }
                    if ui.button(i18n::t("add_domain")).clicked() {
                        let value = domain.trim();
                        if !value.is_empty()
                            && !ws.settings.allowed_domains.iter().any(|item| item == value)
                        {
                            ws.settings.allowed_domains.push(value.to_string());
                        }
                        ui.ctx()
                            .data_mut(|data| data.remove_temp::<String>(input_id));
                    }
                });
                let mut remove = None;
                for (index, value) in ws.settings.allowed_domains.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(value).size(12.0).color(palette.text));
                        if ui.button(i18n::t("remove")).clicked() {
                            remove = Some(index);
                        }
                    });
                }
                if let Some(index) = remove {
                    ws.settings.allowed_domains.remove(index);
                }
                ui.add_space(12.0);
            }
            "get_started" => {
                if ui.button(i18n::t("get_started")).clicked() {
                    commands.push(Command::OpenOnboarding);
                }
                ui.add_space(12.0);
            }
            "members_access" => {
                if ui
                    .add(egui::Button::new(i18n::t("invite_members")))
                    .clicked()
                {
                    commands.push(Command::OpenShare);
                }
                ui.add_space(12.0);
            }
            "data_export" => {
                ui.label(
                    egui::RichText::new(i18n::t("export_includes"))
                        .size(12.0)
                        .color(palette.muted),
                );
                ui.horizontal(|ui| {
                    if ui.button(i18n::t("export_workspace")).clicked() {
                        commands.push(Command::ExportWorkspace);
                    }
                    if ui.button(i18n::t("export_members")).clicked() {
                        commands.push(Command::ExportMembers);
                    }
                });
                if let Some(status) = &view.settings_status {
                    ui.label(egui::RichText::new(status).size(11.0).color(palette.accent));
                }
                ui.add_space(10.0);
            }
            _ => {}
        }
    }
}

pub(super) fn render_sidebar_settings(
    ui: &mut egui::Ui,
    ws: &mut Workspace,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
    rows: &[&SettingsRow],
) {
    let mut section = "";
    for row in rows {
        if row.section != section {
            section = row.section;
            section_heading(ui, section, palette);
        }
        match row.id {
            "library" => {
                if toggle_button(
                    ui,
                    &i18n::t("library"),
                    ws.settings.sidebar_library,
                    palette,
                )
                .clicked()
                {
                    ws.settings.sidebar_library = !ws.settings.sidebar_library;
                }
            }
            "settings" => {
                if toggle_button(
                    ui,
                    &i18n::t("settings"),
                    ws.settings.sidebar_settings,
                    palette,
                )
                .clicked()
                {
                    ws.settings.sidebar_settings = !ws.settings.sidebar_settings;
                }
            }
            "inbox" => {
                if toggle_button(ui, &i18n::t("inbox"), ws.settings.sidebar_inbox, palette)
                    .clicked()
                {
                    ws.settings.sidebar_inbox = !ws.settings.sidebar_inbox;
                }
            }
            "favorites" => {
                if toggle_button(
                    ui,
                    &i18n::t("favorites"),
                    ws.settings.sidebar_favorites,
                    palette,
                )
                .clicked()
                {
                    ws.settings.sidebar_favorites = !ws.settings.sidebar_favorites;
                }
            }
            "private" => {
                if toggle_button(
                    ui,
                    &i18n::t("private"),
                    ws.settings.sidebar_private,
                    palette,
                )
                .clicked()
                {
                    ws.settings.sidebar_private = !ws.settings.sidebar_private;
                }
            }
            "invite_card" => {
                if toggle_button(ui, row.title, ws.settings.sidebar_invite, palette).clicked() {
                    ws.settings.sidebar_invite = !ws.settings.sidebar_invite;
                }
            }
            "customize" => {
                if toggle_button(ui, row.title, ws.settings.sidebar_customize, palette).clicked() {
                    ws.settings.sidebar_customize = !ws.settings.sidebar_customize;
                }
            }
            "other_apps" => {
                if toggle_button(ui, row.title, ws.settings.show_other_apps, palette).clicked() {
                    ws.settings.show_other_apps = !ws.settings.show_other_apps;
                }
            }
            "export" => {
                ui.horizontal(|ui| {
                    if ui.button(i18n::t("export_workspace")).clicked() {
                        commands.push(Command::ExportWorkspace);
                    }
                    if ui.button(i18n::t("export_members")).clicked() {
                        commands.push(Command::ExportMembers);
                    }
                });
                if let Some(status) = &view.settings_status {
                    ui.label(egui::RichText::new(status).size(11.0).color(palette.accent));
                }
                ui.add_space(10.0);
            }
            "analytics" => {
                if toggle_button(ui, row.title, ws.settings.analytics, palette).clicked() {
                    ws.settings.analytics = !ws.settings.analytics;
                }
                ui.add_space(10.0);
            }
            _ => {}
        }
    }
}

pub(super) fn render_offline_settings(
    ui: &mut egui::Ui,
    ws: &mut Workspace,
    palette: &Palette,
    rows: &[&SettingsRow],
) {
    let mut section = "";
    for row in rows {
        if row.section != section {
            section = row.section;
            section_heading(ui, section, palette);
        }
        match row.id {
            "auto_download" => {
                if toggle_button(ui, row.title, ws.settings.offline_auto_download, palette)
                    .clicked()
                {
                    ws.settings.offline_auto_download = !ws.settings.offline_auto_download;
                }
                ui.add_space(14.0);
            }
            "offline_pages" => {
                let search_id = egui::Id::unique("offline_search");
                let mut query: String = ui
                    .ctx()
                    .data(|data| data.get_temp(search_id).unwrap_or_default());
                let edit = ui.add(
                    egui::TextEdit::singleline(&mut query)
                        .desired_width(280.0)
                        .hint_text(i18n::t("search_offline"))
                        .text_color(palette.text),
                );
                if edit.changed() {
                    ui.ctx()
                        .data_mut(|data| data.insert_temp(search_id, query.clone()));
                }
                let query = query.trim().to_lowercase();
                let mut found = false;
                for page in ws.pages.iter().filter(|page| {
                    page.available_offline && page.title.to_lowercase().contains(&query)
                }) {
                    found = true;
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&page.title).color(palette.text));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(&page.updated_at)
                                    .size(11.0)
                                    .color(palette.muted),
                            );
                        });
                    });
                }
                if !found {
                    ui.label(
                        egui::RichText::new(i18n::t("no_offline"))
                            .size(12.0)
                            .color(palette.muted),
                    );
                }
            }
            _ => {}
        }
    }
}

pub(super) fn render_people_settings(
    ui: &mut egui::Ui,
    ws: &mut Workspace,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
    rows: &[&SettingsRow],
) {
    let mut section = "";
    for row in rows {
        if row.section != section {
            section = row.section;
            section_heading(ui, section, palette);
        }
        match row.id {
            "people_directory" => {
                if toggle_button(ui, row.title, ws.settings.people_directory, palette).clicked() {
                    ws.settings.people_directory = !ws.settings.people_directory;
                }
            }
            "recent_activity" => {
                if toggle_button(ui, row.title, ws.settings.recent_activity, palette).clicked() {
                    ws.settings.recent_activity = !ws.settings.recent_activity;
                }
            }
            "hover_cards" => {
                if toggle_button(
                    ui,
                    &i18n::t("hover_cards"),
                    ws.settings.hover_cards,
                    palette,
                )
                .clicked()
                {
                    ws.settings.hover_cards = !ws.settings.hover_cards;
                }
            }
            "members" => {
                if view.share_members.is_empty() {
                    ui.label(
                        egui::RichText::new(i18n::t("no_members"))
                            .size(12.0)
                            .color(palette.muted),
                    );
                }
                for member in view.share_members.iter() {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&member.name).color(palette.text));
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new(&member.email)
                                .size(11.0)
                                .color(palette.muted),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(member.role.label())
                                    .size(11.0)
                                    .color(palette.muted),
                            );
                        });
                    });
                    ui.add_space(4.0);
                }
                if ui
                    .add(egui::Button::new(i18n::t("invite_members")))
                    .clicked()
                {
                    commands.push(Command::OpenShare);
                }
                ui.add_space(10.0);
            }
            "danger_zone" => {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(
                            "Permanently delete this workspace, including all pages and files.",
                        )
                        .size(12.0)
                        .color(palette.muted),
                    );
                    if ui
                        .add(egui::Button::new(
                            egui::RichText::new(i18n::t("delete_workspace"))
                                .color(Color32::from_rgb(224, 108, 108)),
                        ))
                        .clicked()
                    {
                        commands.push(Command::SetSettingsStatus(i18n::t("delete_disabled")));
                    }
                });
                if let Some(status) = &view.settings_status {
                    ui.label(egui::RichText::new(status).size(11.0).color(palette.accent));
                }
                ui.add_space(12.0);
            }
            "workspace_id" => {
                ui.label(
                    egui::RichText::new(i18n::t("workspace_id"))
                        .size(12.0)
                        .color(palette.text),
                );
                ui.label(
                    egui::RichText::new(&ws.settings.id)
                        .size(11.0)
                        .color(palette.muted),
                );
                ui.add_space(10.0);
            }
            _ => {}
        }
    }
}

pub(super) fn render_ai_settings(
    ui: &mut egui::Ui,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
    rows: &[&SettingsRow],
) {
    let mut section = "";
    for row in rows {
        if row.section != section {
            section = row.section;
            section_heading(ui, section, palette);
        }
        match row.id {
            "connection" => {
                ui.horizontal(|ui| {
                    let dot_color = if view.ai_connected {
                        palette.accent
                    } else {
                        palette.muted
                    };
                    let (dot, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                    ui.painter().circle_filled(dot.center(), 4.0, dot_color);
                    ui.label(egui::RichText::new(&view.ai_status).size(12.0).color(
                        if view.ai_connected {
                            palette.text
                        } else {
                            palette.muted
                        },
                    ));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new("Refresh")).clicked() {
                            commands.push(Command::RefreshAiStatus);
                        }
                    });
                });
                ui.add_space(18.0);
            }
            "endpoint" => {
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
            }
            "model" => {
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
            }
            "prefer_cli" => {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(row.title)
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
            }
            "try_it" => {
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
            }
            "shortcut" => {
                ui.label(
                    egui::RichText::new(
                        "Shortcut: Shift+Ctrl+J opens the assistant. Ask anything from Search or ask.",
                    )
                    .size(11.0)
                    .color(palette.muted),
                );
                ui.add_space(10.0);
            }
            _ => {}
        }
    }
}
