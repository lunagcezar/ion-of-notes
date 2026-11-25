//! Server-side Yrs text CRDT per page (spec-042).
//!
//! The page/block structure stays REST-owned; the CRDT merges the page title
//! and each block's plain text. The merged text is projected back into the
//! workspace snapshot after every update, so a restart can re-seed from the
//! durable page document.

#[cfg(feature = "desktop")]
use crate::model::DataSource;
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
        seed_page_objects(&doc, page, workspace)?;
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
        let mut page = workspace
            .page(page_id)
            .cloned()
            .ok_or_else(|| format!("page not found: {page_id}"))?;
        project_page_objects(doc, &mut page)?;
        let mut texts = BTreeMap::new();
        crate::crdt_text::collect_block_texts(doc, &page.blocks, &mut texts);
        let txn = doc.transact();
        crate::crdt_text::project_blocks(&texts, &txn, &mut page.blocks);
        page.title = title.get_string(&txn);
        drop(txn);
        page.updated_at = crate::model::now_label();
        if let Some(slot) = workspace.page_mut(page_id) {
            *slot = page;
        }
        workspace.blocks_changed();
        project_page_rows(doc, workspace, page_id)?;
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

/// Seed the structure, property and row maps for one page.
pub(crate) fn seed_page_objects(
    doc: &Doc,
    page: &Page,
    workspace: &Workspace,
) -> Result<(), String> {
    let structure = doc.get_or_insert_map("structure");
    let properties = doc.get_or_insert_map("properties");
    let rows = doc.get_or_insert_map("rows");
    let mut txn = doc.transact_mut();
    seed_structure(&structure, &mut txn, &page.blocks, None)?;
    for (key, property) in &page.properties {
        let json = serde_json::to_string(property).map_err(|error| error.to_string())?;
        properties.insert(&mut txn, key.clone(), json);
    }
    seed_rows(workspace, &page.id, &rows, &mut txn)
}

/// Project structural blocks and page properties from the Yrs maps.
pub(crate) fn project_page_objects(doc: &Doc, page: &mut Page) -> Result<(), String> {
    let structure = doc.get_or_insert_map("structure");
    let properties = doc.get_or_insert_map("properties");
    let blocks = {
        let txn = doc.transact();
        project_structure(&structure, &txn)?
    };
    page.blocks = blocks;
    let txn = doc.transact();
    page.properties = project_properties(&properties, &txn)?;
    Ok(())
}

/// Project data-source rows for one page into the workspace.
pub(crate) fn project_page_rows(
    doc: &Doc,
    workspace: &mut Workspace,
    page_id: &str,
) -> Result<(), String> {
    let _ = page_id;
    let rows = doc.get_or_insert_map("rows");
    let txn = doc.transact();
    let registers = project_rows(&rows, &txn)?;
    drop(txn);
    apply_rows(workspace, &registers);
    Ok(())
}

/// Set one block structure entry and return the full document update.
#[cfg(feature = "desktop")]
pub(crate) fn set_structure(
    doc: &Doc,
    block: &Block,
    parent_id: Option<&str>,
    position: u32,
) -> Result<Vec<u8>, String> {
    let mut stored = block.clone();
    stored.children.clear();
    let entry = BlockStructure {
        parent_id: parent_id.map(str::to_string),
        position,
        block: stored,
    };
    let json = serde_json::to_string(&entry).map_err(|error| error.to_string())?;
    let map = doc.get_or_insert_map("structure");
    let mut txn = doc.transact_mut();
    map.insert(&mut txn, block.id.clone(), json);
    Ok(txn.encode_diff_v1(&StateVector::default()))
}

/// Remove one block structure entry.
#[cfg(feature = "desktop")]
pub(crate) fn remove_structure(doc: &Doc, block_id: &str) -> Vec<u8> {
    let map = doc.get_or_insert_map("structure");
    let mut txn = doc.transact_mut();
    map.remove(&mut txn, block_id);
    txn.encode_diff_v1(&StateVector::default())
}

/// Set one page property.
#[cfg(feature = "desktop")]
pub(crate) fn set_property(
    doc: &Doc,
    key: &str,
    property: &PageProperty,
) -> Result<Vec<u8>, String> {
    let json = serde_json::to_string(property).map_err(|error| error.to_string())?;
    let map = doc.get_or_insert_map("properties");
    let mut txn = doc.transact_mut();
    map.insert(&mut txn, key.to_string(), json);
    Ok(txn.encode_diff_v1(&StateVector::default()))
}

/// Remove one page property.
#[cfg(feature = "desktop")]
pub(crate) fn remove_property(doc: &Doc, key: &str) -> Vec<u8> {
    let map = doc.get_or_insert_map("properties");
    let mut txn = doc.transact_mut();
    map.remove(&mut txn, key);
    txn.encode_diff_v1(&StateVector::default())
}

/// Set one data-source row register.
#[cfg(feature = "desktop")]
pub(crate) fn set_row(doc: &Doc, data_source_id: &str, row: &Page) -> Result<Vec<u8>, String> {
    let entry = RowRegister {
        data_source_id: data_source_id.to_string(),
        page: row.clone(),
        deleted: false,
    };
    let json = serde_json::to_string(&entry).map_err(|error| error.to_string())?;
    let map = doc.get_or_insert_map("rows");
    let mut txn = doc.transact_mut();
    map.insert(&mut txn, format!("{data_source_id}/{}", row.id), json);
    Ok(txn.encode_diff_v1(&StateVector::default()))
}

/// Mark one data-source row register deleted.
#[cfg(feature = "desktop")]
pub(crate) fn remove_row(doc: &Doc, data_source_id: &str, row_id: &str) -> Vec<u8> {
    let entry = RowRegister {
        data_source_id: data_source_id.to_string(),
        page: Page::new(row_id, "", ""),
        deleted: true,
    };
    let map = doc.get_or_insert_map("rows");
    let mut txn = doc.transact_mut();
    if let Ok(json) = serde_json::to_string(&entry) {
        map.insert(&mut txn, format!("{data_source_id}/{row_id}"), json);
    }
    txn.encode_diff_v1(&StateVector::default())
}

/// Plan one page's structural/property/row changes as a single Yrs update.
///
/// Returns `None` when the change includes fields outside this CRDT scope
/// (title, page flags, data-source schema) so the caller can use the REST
/// path instead.
#[cfg(feature = "desktop")]
pub(crate) fn plan_page_changes(
    doc: &Doc,
    old: &Page,
    new: &Page,
    old_workspace: &Workspace,
    new_workspace: &Workspace,
) -> Result<Option<Vec<u8>>, String> {
    if old.title != new.title || !page_meta_equal(old, new) {
        return Ok(None);
    }
    let old_blocks = flatten_blocks(&old.blocks, None);
    let new_blocks = flatten_blocks(&new.blocks, None);
    if !data_source_schema_equal(old_workspace, new_workspace, &old.id) {
        return Ok(None);
    }

    let mut changed = false;
    for (id, (parent, position, block)) in &new_blocks {
        let differs = match old_blocks.get(id) {
            Some((old_parent, old_position, old_block)) => {
                old_parent != parent
                    || old_position != position
                    || !structure_equivalent(old_block, block)
            }
            None => true,
        };
        if differs {
            set_structure(doc, block, parent.as_deref(), *position)?;
            changed = true;
        }
    }
    for id in old_blocks.keys() {
        if !new_blocks.contains_key(id) {
            remove_structure(doc, id);
            changed = true;
        }
    }

    for (key, property) in &new.properties {
        if old.properties.get(key) != Some(property) {
            set_property(doc, key, property)?;
            changed = true;
        }
    }
    for key in old.properties.keys() {
        if !new.properties.contains_key(key) {
            remove_property(doc, key);
            changed = true;
        }
    }

    let old_rows = attached_rows(old_workspace, &old.id);
    let new_rows = attached_rows(new_workspace, &new.id);
    for (key, (data_source_id, row)) in &new_rows {
        if old_rows.get(key) != Some(&(data_source_id.clone(), row.clone())) {
            set_row(doc, data_source_id, row)?;
            changed = true;
        }
    }
    for (data_source_id, row) in old_rows.values() {
        let key = (data_source_id.clone(), row.id.clone());
        if !new_rows.contains_key(&key) {
            remove_row(doc, data_source_id, &row.id);
            changed = true;
        }
    }

    if !changed {
        return Ok(None);
    }
    let txn = doc.transact();
    Ok(Some(txn.encode_diff_v1(&StateVector::default())))
}

#[cfg(feature = "desktop")]
fn flatten_blocks(
    blocks: &[Block],
    parent: Option<&str>,
) -> BTreeMap<String, (Option<String>, u32, Block)> {
    let mut out = BTreeMap::new();
    for (position, block) in blocks.iter().enumerate() {
        let mut stored = block.clone();
        stored.children.clear();
        out.insert(
            block.id.clone(),
            (parent.map(str::to_string), position as u32, stored),
        );
        out.extend(flatten_blocks(&block.children, Some(&block.id)));
    }
    out
}

#[cfg(feature = "desktop")]
fn structure_equivalent(left: &Block, right: &Block) -> bool {
    fn normalize(mut block: Block) -> Block {
        block.text.clear();
        block.rich_text.clear();
        block.children.clear();
        block.last_edited_time.clear();
        block.last_edited_by = None;
        block
    }
    normalize(left.clone()) == normalize(right.clone())
}

#[cfg(feature = "desktop")]
fn page_meta_equal(left: &Page, right: &Page) -> bool {
    fn normalize(mut page: Page) -> Page {
        page.title.clear();
        page.blocks.clear();
        page.properties.clear();
        page.updated_at.clear();
        page
    }
    normalize(left.clone()) == normalize(right.clone())
}

#[cfg(feature = "desktop")]
fn attached_rows(
    workspace: &Workspace,
    page_id: &str,
) -> BTreeMap<(String, String), (String, Page)> {
    let mut rows = BTreeMap::new();
    for data_source in &workspace.data_sources {
        let belongs = page_of_parent(workspace, &data_source.database_parent).as_deref()
            == Some(page_id)
            || page_of_parent(workspace, &data_source.parent).as_deref() == Some(page_id);
        if !belongs {
            continue;
        }
        for row in &data_source.rows {
            rows.insert(
                (data_source.id.clone(), row.id.clone()),
                (data_source.id.clone(), row.clone()),
            );
        }
    }
    rows
}

#[cfg(feature = "desktop")]
fn data_source_schema_equal(left: &Workspace, right: &Workspace, page_id: &str) -> bool {
    fn schemas(workspace: &Workspace, page_id: &str) -> BTreeMap<String, DataSource> {
        workspace
            .data_sources
            .iter()
            .filter(|data_source| {
                page_of_parent(workspace, &data_source.database_parent).as_deref() == Some(page_id)
                    || page_of_parent(workspace, &data_source.parent).as_deref() == Some(page_id)
            })
            .map(|data_source| {
                let mut copy = data_source.clone();
                copy.rows.clear();
                (data_source.id.clone(), copy)
            })
            .collect()
    }
    schemas(left, page_id) == schemas(right, page_id)
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
