//! Desktop window built with eframe/egui.
//!
//! The window is split by responsibility (SRP): `palette` owns colors, fonts
//! and theming, `constants` owns layout metrics, `icons` owns painting
//! primitives, `app` owns state and commands, `render` draws the chrome,
//! sidebar, page and blocks, `dialogs` draws floating windows, `switcher` owns
//! the shared search / new-tab window, and `page_icon` owns page-icon
//! customization. Every state change is collected as a `Command` and applied
//! through the `Client`, so the UI never mutates server state directly.

mod app;
mod apply;
mod block_ops;
mod commands;
mod constants;
mod crdt;
mod dialogs;
mod edit_queue;
mod format;
mod icon_catalog;
mod icons;
mod input;
mod invite;
mod library_filter;
mod page_icon;
mod palette;
mod remote_worker;
mod render;
mod search_cache;
mod settings_search;
mod switcher;
mod view;
mod widgets;

pub use app::NotionApp;
pub use block_ops::{block_outside_viewport, drop_index, estimate_block_height, split_index};
pub use commands::frame_may_mutate;
pub use crdt::CrdtSession;
pub use dialogs::search_window::page_matches as search_page_matches;
pub use edit_queue::{CommitPlan, PendingEdits, execute_plan, plan_for};
pub use format::{format_number, format_timestamp};
pub use icon_catalog::{EMOJI, ICONS, catalog_matches};
pub use icons::{glyph_visual_offset, page_icon_letter};
pub use invite::{is_valid_email, member_name_from_email, parse_invite_emails};
pub use library_filter::pages_for_view;
pub use palette::{
    Palette, block_menu_options, install_fonts, status_color, strong_font, tab_title,
};
pub use render::app_bar_left_width;
pub use render::index_cursors;
pub use search_cache::SearchCache;
pub use settings_search::{SettingsRow, rows_for, section_matches, settings_matches, visible_rows};
