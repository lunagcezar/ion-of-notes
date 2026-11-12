//! Floating windows: settings, sharing, inbox, page actions, version history,
//! menus, the shortcut sheet and the shared search / new-tab window.
//!
//! The facade re-exports the renderers so `ui::app` keeps one import point.
//! Each child module owns one surface or one cohesive group of surfaces
//! (SRP, spec-017); drawing collects `Command`s and never mutates state.

mod help;
mod menus;
mod onboarding;
mod page_actions;
pub(super) mod search_window;
mod settings;
mod settings_pages;
mod sharing;
mod templates;
mod workspace;

pub(super) use help::render_shortcuts;
pub(super) use menus::{
    render_app_menu, render_compose_menu, render_grid_menu, render_sidebar_menu, render_slash,
    render_workspace_menu,
};
pub(super) use onboarding::render_onboarding;
pub(super) use page_actions::{render_page_actions, render_version_history};
pub(super) use search_window::{render_new_tab, render_search};
pub(super) use settings::render_settings;
pub(super) use sharing::{render_inbox, render_share};
pub(super) use templates::render_templates;
pub(super) use workspace::render_workspace_dialog;
