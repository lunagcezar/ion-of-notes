//! Pure rendering of the window chrome, sidebar, page, blocks, Library and
//! inline database.
//!
//! The facade keeps the public import point used by `ui::app`; each child
//! module owns one surface (SRP, spec-017). Render functions collect
//! `Command`s and never mutate app state directly.

mod chrome;
mod content;
mod database;
mod library;
mod sidebar;

pub use chrome::app_bar_left_width;
pub(crate) use chrome::{SIDEBAR_WIDTH, TITLE_BAR_HEIGHT, render_title_bar};
pub use content::index_cursors;
pub(crate) use content::render_content;
pub(crate) use sidebar::render_sidebar;
