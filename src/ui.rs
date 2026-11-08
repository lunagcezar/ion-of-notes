//! Desktop window built with eframe/egui.
//!
//! The window is split by responsibility (SRP): `palette` owns colors, fonts
//! and theming, `icons` owns painting primitives, `app` owns state and
//! commands, `render` draws the chrome, sidebar, page and blocks, `dialogs`
//! draws floating windows, `switcher` owns the shared search / new-tab window,
//! and `page_icon` owns page-icon customization. Every state change is
//! collected as a `Command` and applied through the `Client`, so the UI never
//! mutates server state directly.

mod app;
mod apply;
mod commands;
mod dialogs;
mod edit_queue;
mod icons;
mod input;
mod invite;
mod page_icon;
mod palette;
mod render;
mod search_cache;
mod settings_search;
mod switcher;
mod view;
mod widgets;

pub use app::NotionApp;
pub use edit_queue::{CommitPlan, PendingEdits, execute_plan, plan_for};
pub use invite::{is_valid_email, member_name_from_email, parse_invite_emails};
pub use palette::{
    Palette, block_menu_options, install_fonts, status_color, strong_font, tab_title,
};
pub use search_cache::SearchCache;
pub use settings_search::settings_matches;
