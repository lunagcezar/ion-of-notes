//! Settings pages shown in the right-hand pane of the settings window.
//!
//! Each page renders the rows selected by `visible_rows`, so a settings query
//! filters the rows inside the open section as well as the navigation rail
//! (spec-016 A9). One module per page keeps the seven surfaces independent
//! (spec-049).

mod ai;
mod offline;
mod people;
mod preferences;
mod server;
mod workspace;

pub(super) use ai::render_ai_settings;
pub(super) use offline::render_offline_settings;
pub(super) use people::render_people_settings;
pub(super) use preferences::render_preferences;
pub(super) use server::render_server_settings;
pub(super) use workspace::render_workspace_settings;

pub(super) use super::super::widgets::{info_row, section_heading};
pub(super) use super::settings::display_row;
