//! Server-side Yrs text CRDT per page (spec-042).
//!
//! The page/block structure stays REST-owned; the CRDT merges the page title
//! and each block's plain text. The merged text is projected back into the
//! workspace snapshot after every update, so a restart can re-seed from the
//! durable page document.

use crate::model::{Block, Page, PageProperty, ParentDto, Workspace};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, Mutex};
use yrs::updates::decoder::Decode;
use yrs::updates::encoder::Encode;
use yrs::{
    Doc, GetString, Map, MapRef, ReadTxn, StateVector, Text, Transact, TransactionMut, Update,
};

/// A block's position in the page structure CRDT.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockStructure {
    pub parent_id: Option<String>,
    pub position: u32,
    /// The block without children; children are separate entries.
    pub block: Block,
}

/// A data-source row register. `deleted` is used for explicit removal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowRegister {
    pub data_source_id: String,
    pub page: Page,
    pub deleted: bool,
}

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
        let structure = doc.get_or_insert_map("structure");
        let properties = doc.get_or_insert_map("properties");
        let rows = doc.get_or_insert_map("rows");
        let texts = crate::crdt_text::collect_text_snapshots(&doc, &page.blocks);
        {
            let mut txn = doc.transact_mut();
            title.insert(&mut txn, 0, &page.title);
            for (_, text, value) in texts {
                if !value.is_empty() {
                    text.insert(&mut txn, 0, &value);
                }
            }
            seed_structure(&structure, &mut txn, &page.blocks, None)
                .map_err(|error| error.to_string())?;
            for (key, property) in &page.properties {
                let json = serde_json::to_string(property).map_err(|error| error.to_string())?;
                properties.insert(&mut txn, key.clone(), json);
            }
            seed_rows(workspace, page_id, &rows, &mut txn).map_err(|error| error.to_string())?;
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

    /// Project the merged title, block structure/text, page properties and
    /// data-source rows back into the workspace page.
    pub fn project(&self, workspace: &mut Workspace, page_id: &str) -> Result<(), String> {
        let docs = self
            .docs
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let doc = docs
            .get(page_id)
            .ok_or_else(|| format!("page not found: {page_id}"))?;
        let title = doc.get_or_insert_text("title");
        let structure = doc.get_or_insert_map("structure");
        let properties = doc.get_or_insert_map("properties");
        let rows = doc.get_or_insert_map("rows");

        let mut blocks = {
            let txn = doc.transact();
            project_structure(&structure, &txn)?
        };
        let mut texts = BTreeMap::new();
        crate::crdt_text::collect_block_texts(doc, &blocks, &mut texts);

        let txn = doc.transact();
        crate::crdt_text::project_blocks(&texts, &txn, &mut blocks);
        let title_text = title.get_string(&txn);
        let page_properties = project_properties(&properties, &txn)?;
        let row_registers = project_rows(&rows, &txn)?;
        drop(txn);

        {
            let page = workspace
                .page_mut(page_id)
                .ok_or_else(|| format!("page not found: {page_id}"))?;
            page.title = title_text;
            page.blocks = blocks;
            page.properties = page_properties;
            page.updated_at = crate::model::now_label();
        }
        apply_rows(workspace, &row_registers);
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

fn seed_structure(
    map: &MapRef,
    txn: &mut TransactionMut<'_>,
    blocks: &[Block],
    parent_id: Option<&str>,
) -> Result<(), String> {
    for (position, block) in blocks.iter().enumerate() {
        let mut stored = block.clone();
        stored.children.clear();
        let entry = BlockStructure {
            parent_id: parent_id.map(str::to_string),
            position: position as u32,
            block: stored,
        };
        let json = serde_json::to_string(&entry).map_err(|error| error.to_string())?;
        map.insert(txn, block.id.clone(), json);
        seed_structure(map, txn, &block.children, Some(&block.id))?;
    }
    Ok(())
}

fn project_structure(map: &MapRef, txn: &yrs::Transaction<'_>) -> Result<Vec<Block>, String> {
    let mut levels: HashMap<Option<String>, Vec<(u32, Block)>> = HashMap::new();
    for (_, value) in map.iter(txn) {
        let json = value.to_string(txn);
        let entry: BlockStructure =
            serde_json::from_str(&json).map_err(|error| error.to_string())?;
        levels
            .entry(entry.parent_id)
            .or_default()
            .push((entry.position, entry.block));
    }
    let mut visited = HashSet::new();
    Ok(build_block_level(None, &mut levels, &mut visited))
}

fn build_block_level(
    parent: Option<&str>,
    levels: &mut HashMap<Option<String>, Vec<(u32, Block)>>,
    visited: &mut HashSet<String>,
) -> Vec<Block> {
    let mut level = levels
        .remove(&parent.map(str::to_string))
        .unwrap_or_default();
    level.sort_by_key(|(position, _)| *position);
    level
        .into_iter()
        .filter_map(|(_, mut block)| {
            if !visited.insert(block.id.clone()) {
                return None;
            }
            block.children = build_block_level(Some(&block.id), levels, visited);
            Some(block)
        })
        .collect()
}

fn page_of_parent(workspace: &Workspace, parent: &ParentDto) -> Option<String> {
    match parent {
        ParentDto::Page { page_id } => Some(page_id.clone()),
        ParentDto::Block { block_id } => workspace.page_id_for_block(block_id),
        ParentDto::Database { database_id } => workspace
            .database(database_id)
            .and_then(|database| page_of_parent(workspace, &database.parent)),
        ParentDto::DataSource { data_source_id } => {
            workspace
                .data_source(data_source_id)
                .and_then(|data_source| {
                    page_of_parent(workspace, &data_source.database_parent)
                        .or_else(|| page_of_parent(workspace, &data_source.parent))
                })
        }
        ParentDto::Workspace { .. } => None,
    }
}

fn seed_rows(
    workspace: &Workspace,
    page_id: &str,
    map: &MapRef,
    txn: &mut TransactionMut<'_>,
) -> Result<(), String> {
    for data_source in &workspace.data_sources {
        let belongs = page_of_parent(workspace, &data_source.database_parent).as_deref()
            == Some(page_id)
            || page_of_parent(workspace, &data_source.parent).as_deref() == Some(page_id);
        if !belongs {
            continue;
        }
        for row in &data_source.rows {
            let entry = RowRegister {
                data_source_id: data_source.id.clone(),
                page: row.clone(),
                deleted: false,
            };
            let json = serde_json::to_string(&entry).map_err(|error| error.to_string())?;
            map.insert(txn, format!("{}/{}", data_source.id, row.id), json);
        }
    }
    Ok(())
}

fn project_rows(map: &MapRef, txn: &yrs::Transaction<'_>) -> Result<Vec<RowRegister>, String> {
    let mut registers = Vec::new();
    for (_, value) in map.iter(txn) {
        let json = value.to_string(txn);
        registers.push(serde_json::from_str(&json).map_err(|error| error.to_string())?);
    }
    Ok(registers)
}

fn project_properties(
    map: &MapRef,
    txn: &yrs::Transaction<'_>,
) -> Result<BTreeMap<String, PageProperty>, String> {
    let mut properties = BTreeMap::new();
    for (key, value) in map.iter(txn) {
        let json = value.to_string(txn);
        let property = serde_json::from_str(&json).map_err(|error| error.to_string())?;
        properties.insert(key.to_string(), property);
    }
    Ok(properties)
}

fn apply_rows(workspace: &mut Workspace, registers: &[RowRegister]) {
    for register in registers {
        let Some(data_source) = workspace
            .data_sources
            .iter_mut()
            .find(|data_source| data_source.id == register.data_source_id)
        else {
            continue;
        };
        if register.deleted {
            data_source.rows.retain(|row| row.id != register.page.id);
            continue;
        }
        if let Some(row) = data_source
            .rows
            .iter_mut()
            .find(|row| row.id == register.page.id)
        {
            *row = register.page.clone();
        } else {
            data_source.rows.push(register.page.clone());
        }
    }
}
