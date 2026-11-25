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
use yrs::{Doc, GetString, ReadTxn, StateVector, Text, TextRef, Transact, Update};

#[derive(Default)]
pub struct CrdtSession {
    doc: Doc,
    page_id: Option<String>,
    last_text: BTreeMap<String, String>,
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
        self.structure_ids.clear();
    }

    /// Refresh the local diff baseline from the merged document.
    pub fn baseline_page(&mut self, page: &Page) {
        self.last_text.clear();
        self.track_structure(page);
        let mut texts = BTreeMap::new();
        crate::crdt_text::collect_block_texts(&self.doc, &page.blocks, &mut texts);
        let txn = self.doc.transact();
        for (block_id, text) in texts {
            self.last_text.insert(block_id, text.get_string(&txn));
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
        let title = self.doc.get_or_insert_text("title");
        let texts = crate::crdt_text::collect_text_snapshots(&self.doc, &page.blocks);
        {
            let mut txn = self.doc.transact_mut();
            title.insert(&mut txn, 0, &page.title);
            for (block_id, text, value) in texts {
                if !value.is_empty() {
                    text.insert(&mut txn, 0, &value);
                }
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
        let old: Vec<char> = old_text.chars().collect();
        let new: Vec<char> = new_text.chars().collect();
        let prefix = old
            .iter()
            .zip(new.iter())
            .take_while(|(left, right)| left == right)
            .count();
        let suffix = old
            .iter()
            .rev()
            .zip(new.iter().rev())
            .take_while(|(left, right)| left == right)
            .count()
            .min(old.len().min(new.len()).saturating_sub(prefix));
        let delete_len = old.len() - prefix - suffix;
        let insert_len = new.len() - prefix - suffix;
        let inserted: String = new[prefix..prefix + insert_len].iter().collect();

        let text = self.doc.get_or_insert_text(format!("block:{block_id}"));
        let mut txn = self.doc.transact_mut();
        if delete_len > 0 {
            text.remove_range(&mut txn, prefix as u32, delete_len as u32);
        }
        if !inserted.is_empty() {
            text.insert(&mut txn, prefix as u32, &inserted);
        }
        self.last_text
            .insert(block_id.to_string(), new_text.to_string());
        Ok(Some(txn.encode_diff_v1(&StateVector::default())))
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
            self.last_text.insert(key, text.get_string(&txn));
        }
    }
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
