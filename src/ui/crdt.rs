//! Client-side Yrs session for the current page (spec-043).
//!
//! The editor still owns `Block.text`; this session derives a minimal diff from
//! the last known text, applies it to a Yrs document, and returns a compact
//! update for the server. Remote updates are applied back and projected into
//! the page.

use crate::model::{Block, Page, Workspace};
use std::collections::{BTreeMap, HashSet};
use yrs::updates::decoder::Decode;
use yrs::updates::encoder::Encode;
use yrs::{Doc, GetString, ReadTxn, Text, TextRef, Transact, Update};

#[derive(Default)]
pub struct CrdtSession {
    doc: Doc,
    page_id: Option<String>,
    last_text: BTreeMap<String, String>,
    last_chars: BTreeMap<String, usize>,
    structure_ids: HashSet<String>,
}

impl CrdtSession {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start an empty document and wait for the server's full update. Use this
    /// when the page already has a server-side CRDT document, so both sides do
    /// not insert the same initial text independently.
    pub fn open_empty(&mut self, page_id: &str) {
        self.doc = Doc::new();
        self.page_id = Some(page_id.to_string());
        self.last_text.clear();
        self.last_chars.clear();
        self.structure_ids.clear();
    }

    /// Refresh the local diff baseline from the merged document.
    pub fn baseline_page(&mut self, page: &Page) {
        self.last_text.clear();
        self.last_chars.clear();
        self.track_structure(page);
        let mut texts = BTreeMap::new();
        crate::crdt_text::collect_block_texts(&self.doc, &page.blocks, &mut texts);
        let txn = self.doc.transact();
        for (block_id, text) in texts {
            let value = text.get_string(&txn);
            self.last_chars
                .insert(block_id.clone(), value.chars().count());
            self.last_text.insert(block_id, value);
        }
    }

    /// Start a fresh document from the current page snapshot.
    pub fn open_page(&mut self, page: &Page) {
        self.open_page_with_workspace(page, &Workspace::new(""));
    }

    /// Start a fresh document from the page and its data-source rows.
    pub fn open_page_with_workspace(&mut self, page: &Page, workspace: &Workspace) {
        self.doc = Doc::new();
        self.page_id = Some(page.id.clone());
        self.last_text.clear();
        self.last_chars.clear();
        let title = self.doc.get_or_insert_text("title");
        let texts = crate::crdt_text::collect_text_snapshots(&self.doc, &page.blocks);
        {
            let mut txn = self.doc.transact_mut();
            title.insert(&mut txn, 0, &page.title);
            for (block_id, text, value) in texts {
                if !value.is_empty() {
                    text.insert(&mut txn, 0, &value);
                }
                self.last_chars
                    .insert(block_id.clone(), value.chars().count());
                self.last_text.insert(block_id, value);
            }
        }
        let _ = crate::server::crdt::seed_page_objects(&self.doc, page, workspace);
        self.track_structure(page);
    }

    fn track_structure(&mut self, page: &Page) {
        self.structure_ids.clear();
        collect_block_ids(&page.blocks, &mut self.structure_ids);
    }

    /// Insert structure entries for blocks created after the document opened.
    /// This keeps a split block visible before the next full structure commit.
    pub fn sync_new_blocks(&mut self, page: &Page) {
        let mut missing = Vec::new();
        collect_missing_blocks(&page.blocks, None, &self.structure_ids, &mut missing);
        for (parent, position, block) in missing {
            if crate::server::crdt::set_structure(&self.doc, block, parent.as_deref(), position)
                .is_ok()
            {
                self.structure_ids.insert(block.id.clone());
            }
        }
    }

    pub fn page_id(&self) -> Option<&str> {
        self.page_id.as_deref()
    }

    pub fn state_vector(&self) -> Vec<u8> {
        self.doc.transact().state_vector().encode_v1()
    }

    /// Encode the whole document, used to seed a peer before incremental
    /// updates (spec-141).
    pub fn full_update(&self) -> Vec<u8> {
        let txn = self.doc.transact();
        txn.encode_diff_v1(&yrs::StateVector::default())
    }

    /// Apply a remote update and refresh the local diff baseline.
    pub fn apply_update(&mut self, update: &[u8]) -> Result<(), String> {
        let decoded = Update::decode_v1(update).map_err(|error| error.to_string())?;
        {
            let mut txn = self.doc.transact_mut();
            txn.apply_update(decoded)
                .map_err(|error| error.to_string())?;
        }
        self.refresh_last_text();
        Ok(())
    }

    /// Apply one local text edit and return an update when the text changed.
    pub fn local_text(
        &mut self,
        block_id: &str,
        new_text: &str,
    ) -> Result<Option<Vec<u8>>, String> {
        let old_text = self.last_text.get(block_id).cloned().unwrap_or_default();
        if old_text == new_text {
            return Ok(None);
        }
        let old_chars = self
            .last_chars
            .get(block_id)
            .copied()
            .unwrap_or_else(|| old_text.chars().count());
        let new_chars = new_text.chars().count();

        // Typing at the end and backspace are by far the common cases; they
        // are answered without scanning both strings character by character.
        let (prefix, delete_len, inserted): (usize, usize, &str) =
            if new_text.starts_with(old_text.as_str()) {
                (old_chars, 0, &new_text[old_text.len()..])
            } else if old_text.starts_with(new_text) {
                (new_chars, old_chars.saturating_sub(new_chars), "")
            } else {
                let prefix = old_text
                    .chars()
                    .zip(new_text.chars())
                    .take_while(|(left, right)| left == right)
                    .count();
                let suffix = old_text
                    .chars()
                    .rev()
                    .zip(new_text.chars().rev())
                    .take_while(|(left, right)| left == right)
                    .count()
                    .min(old_chars.min(new_chars).saturating_sub(prefix));
                let delete_len = old_chars - prefix - suffix;
                let insert_chars = new_chars - prefix - suffix;
                let start = char_byte_index(new_text, prefix);
                let end = char_byte_index(new_text, prefix + insert_chars);
                (prefix, delete_len, &new_text[start..end])
            };

        let text = self.doc.get_or_insert_text(format!("block:{block_id}"));
        let mut txn = self.doc.transact_mut();
        if delete_len > 0 {
            text.remove_range(&mut txn, prefix as u32, delete_len as u32);
        }
        if !inserted.is_empty() {
            text.insert(&mut txn, prefix as u32, inserted);
        }
        self.last_text
            .insert(block_id.to_string(), new_text.to_string());
        self.last_chars.insert(block_id.to_string(), new_chars);
        // `encode_update_v1` encodes only this transaction; the previous
        // `encode_diff_v1(&StateVector::default())` re-sent the whole document
        // on every keystroke (spec-141).
        Ok(Some(txn.encode_update_v1()))
    }

    /// Write the merged title/structure/properties/block text back into a page.
    pub fn project_page(&self, page: &mut Page) -> Result<(), String> {
        crate::server::crdt::project_page_objects(&self.doc, page)?;
        let title = self.doc.get_or_insert_text("title");
        let mut texts = BTreeMap::new();
        crate::crdt_text::collect_block_texts(&self.doc, &page.blocks, &mut texts);
        let txn = self.doc.transact();
        page.title = title.get_string(&txn);
        crate::crdt_text::project_blocks(&texts, &txn, &mut page.blocks);
        Ok(())
    }

    /// Write merged data-source rows for this page into the workspace.
    pub fn project_rows(&self, workspace: &mut Workspace) -> Result<(), String> {
        let page_id = self
            .page_id
            .as_deref()
            .ok_or_else(|| "no CRDT page is open".to_string())?;
        crate::server::crdt::project_page_rows(&self.doc, workspace, page_id)
    }

    /// Plan structure/property/row differences as one update.
    pub fn plan_page_changes(
        &self,
        old: &Page,
        new: &Page,
        old_workspace: &Workspace,
        new_workspace: &Workspace,
    ) -> Result<Option<Vec<u8>>, String> {
        crate::server::crdt::plan_page_changes(&self.doc, old, new, old_workspace, new_workspace)
    }

    fn refresh_last_text(&mut self) {
        let keys: Vec<String> = self.last_text.keys().cloned().collect();
        let refs: Vec<(String, TextRef)> = keys
            .iter()
            .map(|key| {
                (
                    key.clone(),
                    self.doc.get_or_insert_text(format!("block:{key}")),
                )
            })
            .collect();
        let txn = self.doc.transact();
        for (key, text) in refs {
            let value = text.get_string(&txn);
            self.last_chars.insert(key.clone(), value.chars().count());
            self.last_text.insert(key, value);
        }
    }
}

/// Byte offset of character `index` (or the string length).
fn char_byte_index(text: &str, index: usize) -> usize {
    text.char_indices()
        .nth(index)
        .map(|(offset, _)| offset)
        .unwrap_or(text.len())
}

fn collect_block_ids(blocks: &[Block], out: &mut HashSet<String>) {
    for block in blocks {
        out.insert(block.id.clone());
        collect_block_ids(&block.children, out);
    }
}

fn collect_missing_blocks<'a>(
    blocks: &'a [Block],
    parent: Option<&'a str>,
    known: &HashSet<String>,
    out: &mut Vec<(Option<String>, u32, &'a Block)>,
) {
    for (position, block) in blocks.iter().enumerate() {
        if !known.contains(&block.id) {
            out.push((parent.map(str::to_string), position as u32, block));
        }
        collect_missing_blocks(&block.children, Some(&block.id), known, out);
    }
}
