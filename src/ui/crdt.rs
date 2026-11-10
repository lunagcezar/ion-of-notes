//! Client-side Yrs session for the current page (spec-043).
//!
//! The editor still owns `Block.text`; this session derives a minimal diff from
//! the last known text, applies it to a Yrs document, and returns a compact
//! update for the server. Remote updates are applied back and projected into
//! the page.

use crate::model::{Block, Page, RichText};
use std::collections::BTreeMap;
use yrs::updates::decoder::Decode;
use yrs::updates::encoder::Encode;
use yrs::{Doc, GetString, ReadTxn, StateVector, Text, TextRef, Transact, Transaction, Update};

#[derive(Default)]
pub struct CrdtSession {
    doc: Doc,
    page_id: Option<String>,
    last_text: BTreeMap<String, String>,
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
    }

    /// Refresh the local diff baseline from the merged document.
    pub fn baseline_page(&mut self, page: &Page) {
        self.last_text.clear();
        let mut texts = BTreeMap::new();
        collect_block_texts(&self.doc, &page.blocks, &mut texts);
        let txn = self.doc.transact();
        for (block_id, text) in texts {
            self.last_text.insert(block_id, text.get_string(&txn));
        }
    }

    /// Start a fresh document from the current page snapshot.
    pub fn open_page(&mut self, page: &Page) {
        self.doc = Doc::new();
        self.page_id = Some(page.id.clone());
        self.last_text.clear();
        let title = self.doc.get_or_insert_text("title");
        let mut texts = Vec::new();
        collect_texts(&self.doc, &page.blocks, &mut texts);
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

    /// Write the merged title/block text back into a page.
    pub fn project_page(&self, page: &mut Page) {
        let title = self.doc.get_or_insert_text("title");
        let mut texts = BTreeMap::new();
        collect_block_texts(&self.doc, &page.blocks, &mut texts);
        let txn = self.doc.transact();
        page.title = title.get_string(&txn);
        project_blocks(&texts, &txn, &mut page.blocks);
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

fn collect_texts(doc: &Doc, blocks: &[Block], out: &mut Vec<(String, TextRef, String)>) {
    for block in blocks {
        let text = doc.get_or_insert_text(format!("block:{}", block.id));
        out.push((block.id.clone(), text, block.plain_text()));
        collect_texts(doc, &block.children, out);
    }
}

fn collect_block_texts(doc: &Doc, blocks: &[Block], out: &mut BTreeMap<String, TextRef>) {
    for block in blocks {
        out.insert(
            block.id.clone(),
            doc.get_or_insert_text(format!("block:{}", block.id)),
        );
        collect_block_texts(doc, &block.children, out);
    }
}

fn project_blocks(texts: &BTreeMap<String, TextRef>, txn: &Transaction<'_>, blocks: &mut [Block]) {
    for block in blocks {
        if let Some(text) = texts.get(&block.id) {
            block.text = text.get_string(txn);
            if block.rich_text.len() <= 1 {
                block.rich_text = vec![RichText::text(block.text.clone())];
            }
        }
        project_blocks(texts, txn, &mut block.children);
    }
}
