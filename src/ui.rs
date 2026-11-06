//! Desktop window built with eframe/egui.
//!
//! The window is split by responsibility (SRP): `palette` owns colors, fonts
//! and theming, `icons` owns painting primitives, `app` owns state and
//! commands, `render` draws the chrome, sidebar, page and blocks, and
//! `dialogs` draws floating windows. Every state change is collected as a
//! `Command` and applied through the `Client`, so the UI never mutates server
//! state directly.

mod app;
mod dialogs;
mod icons;
mod palette;
mod render;

pub use app::NotionApp;
pub use palette::{
    block_menu_options, install_fonts, status_color, strong_font, tab_title, Palette,
};
