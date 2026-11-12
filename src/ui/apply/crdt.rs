//! Command family: crdt.

use crate::server::realtime::Awareness;

use super::super::app::NotionApp;
use super::super::commands::Command;
use super::set_block_text;
impl NotionApp {
    pub(super) fn apply_crdt(&mut self, command: Command) {
        match command {
            Command::CrdtTextChanged {
                page_id,
                block_id,
                text,
            } => {
                if page_id == self.collaboration.page
                    && let Ok(Some(update)) = self.collaboration.crdt.local_text(&block_id, &text)
                    && self
                        .client
                        .apply_crdt_update(&page_id, &update, "m-owner")
                        .is_ok()
                    && let Some(page) = self.last_committed.page_mut(&page_id)
                {
                    set_block_text(&mut page.blocks, &block_id, &text);
                }
            }
            Command::CrdtCursor {
                page_id,
                block_id,
                index,
                selection_end,
            } => {
                let state = (block_id.clone(), index, selection_end);
                if self.collaboration.last_awareness.as_ref() != Some(&state) {
                    self.collaboration.last_awareness = Some(state);
                    let awareness = Awareness {
                        session_id: self.collaboration.session.clone(),
                        member_id: "m-owner".to_string(),
                        name: "You".to_string(),
                        color: self.collaboration.color.clone(),
                        page_id,
                        block_id,
                        index,
                        selection_end,
                    };
                    let _ = self.client.publish_awareness(&awareness);
                }
            }
            other => debug_assert!(false, "unexpected crdt command: {other:?}"),
        }
    }
}
