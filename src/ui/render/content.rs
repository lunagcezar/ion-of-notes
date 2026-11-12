//! Content area: breadcrumbs, page header, editable blocks and the floating
//! controls.
//!
//! `render_content` is the single entry point; page, block and remote-cursor
//! rendering live in child modules (spec-049).

mod blocks;
mod cursors;
mod page;

pub use cursors::index_cursors;
pub(crate) use page::render_content;
