//! Server-side Yrs text CRDT per page (spec-042).
//!
//! The page/block structure stays REST-owned; the CRDT merges the page title
//! and each block's plain text. The merged text is projected back into the
//! workspace snapshot after every update, so a restart can re-seed from the
//! durable page document.

use crate::model::Workspace;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use yrs::updates::decoder::Decode;
use yrs::updates::encoder::Encode;
use yrs::{Doc, GetString, ReadTxn, StateVector, Text, Transact, Update};

/// In-memory CRDT documents keyed by page id.
#[derive(Clone, Default)]
pub struct CrdtStore {
    docs: Arc<Mutex<BTreeMap<String, Doc>>>,
}

impl CrdtStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create the CRDT document from the current page snapshot if needed.
    pub fn ensure_page(&self, workspace: &Workspace, page_id: &str) -> Result<(), String> {
        let mut docs = self
            .docs
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if docs.contains_key(page_id) {
            return Ok(());
        }
        let page = workspace
            .page(page_id)
            .ok_or_else(|| format!("page not found: {page_id}"))?;
        let doc = Doc::new();
        let title = doc.get_or_insert_text("title");
        let texts = crate::crdt_text::collect_text_snapshots(&doc, &page.blocks);
        {
            let mut txn = doc.transact_mut();
            title.insert(&mut txn, 0, &page.title);
            for (_, text, value) in texts {
                if !value.is_empty() {
                    text.insert(&mut txn, 0, &value);
                }
            }
        }
        docs.insert(page_id.to_string(), doc);
        Ok(())
    }

    /// Return the update after `state_vector` plus the current state vector.
    pub fn sync(
        &self,
        workspace: &Workspace,
        page_id: &str,
        state_vector: Option<&[u8]>,
    ) -> Result<(Vec<u8>, Vec<u8>), String> {
        self.ensure_page(workspace, page_id)?;
        let docs = self
            .docs
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let doc = docs
            .get(page_id)
            .ok_or_else(|| format!("page not found: {page_id}"))?;
        let txn = doc.transact();
        let update = match state_vector {
            Some(bytes) => {
                let state_vector =
                    StateVector::decode_v1(bytes).map_err(|error| error.to_string())?;
                txn.encode_diff_v1(&state_vector)
            }
            None => txn.encode_diff_v1(&StateVector::default()),
        };
        Ok((update, txn.state_vector().encode_v1()))
    }

    /// Apply one update and return the same update for broadcasting.
    pub fn apply_update(
        &self,
        workspace: &Workspace,
        page_id: &str,
        update: &[u8],
    ) -> Result<Vec<u8>, String> {
        self.ensure_page(workspace, page_id)?;
        let mut docs = self
            .docs
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let doc = docs
            .get_mut(page_id)
            .ok_or_else(|| format!("page not found: {page_id}"))?;
        let decoded = Update::decode_v1(update).map_err(|error| error.to_string())?;
        {
            let mut txn = doc.transact_mut();
            txn.apply_update(decoded)
                .map_err(|error| error.to_string())?;
        }
        Ok(update.to_vec())
    }

    /// Project the merged title and block text back into the workspace page.
    pub fn project(&self, workspace: &mut Workspace, page_id: &str) -> Result<(), String> {
        let docs = self
            .docs
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let doc = docs
            .get(page_id)
            .ok_or_else(|| format!("page not found: {page_id}"))?;
        let (title, texts) = {
            let page = workspace
                .page(page_id)
                .ok_or_else(|| format!("page not found: {page_id}"))?;
            let title = doc.get_or_insert_text("title");
            let mut texts = BTreeMap::new();
            crate::crdt_text::collect_block_texts(doc, &page.blocks, &mut texts);
            (title, texts)
        };
        let txn = doc.transact();
        let page = workspace
            .page_mut(page_id)
            .ok_or_else(|| format!("page not found: {page_id}"))?;
        page.title = title.get_string(&txn);
        crate::crdt_text::project_blocks(&texts, &txn, &mut page.blocks);
        page.updated_at = crate::model::now_label();
        Ok(())
    }

    /// Drop one document so the next sync re-seeds from the page snapshot.
    pub fn reset_page(&self, page_id: &str) {
        self.docs
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(page_id);
    }

    /// Drop every document (used after a whole-workspace replace).
    pub fn reset_all(&self) {
        self.docs
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
    }
}
