//! Command family: crdt.

use crate::server::realtime::Awareness;

use super::super::app::NotionApp;
use super::super::commands::Command;
impl NotionApp {
    /// Retry CRDT updates that could not enter the remote worker queue.
    pub(super) fn flush_pending_crdt(&mut self) {
        while let Some((page_id, block_id, text, update)) = self.pending_crdt.first().cloned() {
            if !self.dispatch_crdt_update(page_id, block_id, text, update) {
                break;
            }
            self.pending_crdt.remove(0);
        }
    }

    /// Sync blocks created or removed by a structural command.
    pub(super) fn sync_crdt_structure(&mut self) {
        let page_id = self.current_page.clone();
        if page_id == self.collaboration.page
            && let Some(page) = self.workspace.page(&page_id).cloned()
        {
            self.collaboration.crdt.sync_new_blocks(&page);
        }
    }

    pub(super) fn apply_crdt(&mut self, command: Command) {
        match command {
            Command::CrdtTextChanged {
                page_id,
                block_id,
                text,
            } => {
                // Local mode keeps the text in the workspace and commits it
                // through the debounced REST write; only a remote server needs
                // the incremental CRDT update (spec-141).
                if page_id == self.collaboration.page
                    && let Ok(Some(update)) = self.collaboration.crdt.local_text(&block_id, &text)
                    && self.remote.is_remote()
                {
                    self.flush_pending_crdt();
                    if !self.dispatch_crdt_update(
                        page_id.clone(),
                        block_id.clone(),
                        text.clone(),
                        update.clone(),
                    ) {
                        // Never block the UI on an HTTP round trip when the
                        // worker queue is full (spec-141).
                        if self.pending_crdt.len() >= 512 {
                            self.pending_crdt.remove(0);
                        }
                        self.pending_crdt.push((page_id, block_id, text, update));
                    }
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
