//! Shared Yrs text helpers for the server and desktop CRDT sessions.
//!
//! The page/block structure stays REST-owned. Both sides walk the same block
//! tree and address each block's text as `block:{id}`; keeping the walk and
//! projection here avoids two slightly different copies.

use crate::model::{Block, RichText, walk_blocks, walk_blocks_mut};
use std::collections::BTreeMap;
use yrs::{Doc, GetString, TextRef, Transaction};

/// The Yrs text name for one block.
pub(crate) fn block_text_name(block_id: &str) -> String {
    format!("block:{block_id}")
}

/// Collect the Yrs text handle for every block in the tree.
pub(crate) fn collect_block_texts(
    doc: &Doc,
    blocks: &[Block],
    out: &mut BTreeMap<String, TextRef>,
) {
    walk_blocks(blocks, &mut |block| {
        out.insert(
            block.id.clone(),
            doc.get_or_insert_text(block_text_name(&block.id)),
        );
    });
}

/// Collect `(block_id, text handle, current plain text)` for the tree.
pub(crate) fn collect_text_snapshots(
    doc: &Doc,
    blocks: &[Block],
) -> Vec<(String, TextRef, String)> {
    let mut out = Vec::new();
    walk_blocks(blocks, &mut |block| {
        out.push((
            block.id.clone(),
            doc.get_or_insert_text(block_text_name(&block.id)),
            block.plain_text(),
        ));
    });
    out
}

/// Project the merged Yrs text back into the block tree.
pub(crate) fn project_blocks(
    texts: &BTreeMap<String, TextRef>,
    txn: &Transaction<'_>,
    blocks: &mut [Block],
) {
    walk_blocks_mut(blocks, &mut |block| {
        if let Some(text) = texts.get(&block.id) {
            block.text = text.get_string(txn);
            if block.rich_text.len() <= 1 {
                block.rich_text = vec![RichText::text(block.text.clone())];
            }
        }
    });
}
