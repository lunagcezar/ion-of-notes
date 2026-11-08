//! First-run onboarding: “What is this space for?” (spec-027).

use egui::{Align2, CornerRadius, FontId, Sense, Vec2};

use crate::i18n;

use super::super::commands::Command;
use super::super::palette::{Palette, strong_font};
use super::super::view::ViewState;
use super::super::widgets::popover_frame;

fn choice(
    ui: &mut egui::Ui,
    icon: &str,
    title: &str,
    description: &str,
    command: Command,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 96.0), Sense::click());
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(8), palette.hover);
    }
    ui.painter().text(
        rect.min + Vec2::new(18.0, 34.0),
        Align2::LEFT_CENTER,
        icon,
        FontId::proportional(24.0),
        palette.muted,
    );
    ui.painter().text(
        rect.min + Vec2::new(58.0, 30.0),
        Align2::LEFT_CENTER,
        title,
        strong_font(15.0),
        palette.text,
    );
    ui.painter().text(
        rect.min + Vec2::new(58.0, 52.0),
        Align2::LEFT_CENTER,
        description,
        FontId::proportional(12.0),
        palette.muted,
    );
    if response.clicked() {
        commands.push(command);
    }
}

pub(crate) fn render_onboarding(
    ctx: &egui::Context,
    _view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    egui::Window::new("onboarding")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_size(Vec2::new(520.0, 540.0))
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .frame(popover_frame(palette))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(i18n::t("onboarding_title"))
                        .font(strong_font(22.0))
                        .color(palette.text),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("×").size(18.0).color(palette.muted),
                            )
                            .frame(false),
                        )
                        .clicked()
                    {
                        commands.push(Command::CloseOnboarding);
                    }
                });
            });
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(i18n::t("onboarding_subtitle"))
                    .size(12.5)
                    .color(palette.muted),
            );
            ui.add_space(18.0);
            choice(
                ui,
                "🏢",
                &i18n::t("onboarding_work"),
                &i18n::t("onboarding_work_desc"),
                Command::CompleteOnboarding("work".to_string()),
                palette,
                commands,
            );
            ui.add_space(8.0);
            choice(
                ui,
                "🏠",
                &i18n::t("onboarding_personal"),
                &i18n::t("onboarding_personal_desc"),
                Command::CompleteOnboarding("personal".to_string()),
                palette,
                commands,
            );
            ui.add_space(8.0);
            choice(
                ui,
                "🎓",
                &i18n::t("onboarding_school"),
                &i18n::t("onboarding_school_desc"),
                Command::CompleteOnboarding("school".to_string()),
                palette,
                commands,
            );
            ui.add_space(18.0);
            if ui.button(i18n::t("skip_for_now")).clicked() {
                commands.push(Command::CloseOnboarding);
            }
        });
}
