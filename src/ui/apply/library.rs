//! Command family: library.

use super::super::app::NotionApp;
use super::super::commands::Command;
impl NotionApp {
    pub(super) fn apply_library(&mut self, command: Command) {
        match command {
            Command::OpenLibrary => {
                self.close_overlays();
                self.library_open = true;
                self.refresh_library();
            }
            Command::CloseLibrary => self.library_open = false,
            Command::SetLibraryTab(tab) => self.library_tab = tab,
            other => debug_assert!(false, "unexpected library command: {other:?}"),
        }
    }
}
