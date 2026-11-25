//! Order-statistic index of block slot heights.
//!
//! The renderer used to visit every block to know where the visible window
//! starts. This index stores one slot height per block in a Fenwick tree, so a
//! normal frame only pays O(log n) for the window lookup plus the visible
//! blocks. The structure is rebuilt only when the block count or the cheap
//! order signature changes; text edits update one tree entry when the block is
//! rendered.

use crate::model::{Block, BlockKind};
use crate::ui::constants::{BLOCK_GAP, BLOCK_HEADING_GAP};
use egui::Context;

#[derive(Clone, Copy, Debug, Default)]
struct Entry {
    fingerprint: u64,
    slot: f32,
    numbered: bool,
}

/// One block's slot height and the two prefix trees.
#[derive(Clone, Debug, Default)]
pub(crate) struct HeightIndex {
    page_id: String,
    len: usize,
    order_signature: u64,
    entries: Vec<Entry>,
    heights: FloatTree,
    numbered: CountTree,
}

impl HeightIndex {
    /// Load the cached index for this page, rebuilding it when needed.
    ///
    /// The value is temporarily removed from egui's temp map so the caller owns
    /// it without cloning the trees each frame; it is put back by [`Self::save`].
    pub(crate) fn sync(ctx: &Context, page_id: &str, blocks: &[Block]) -> Self {
        let id = egui::Id::unique(("block_height_index", page_id));
        let signature = order_signature(blocks);
        if let Some(index) = ctx.data_mut(|data| data.remove_temp::<Self>(id))
            && index.len == blocks.len()
            && index.order_signature == signature
        {
            return index;
        }
        Self::build(page_id, blocks, signature)
    }

    fn build(page_id: &str, blocks: &[Block], signature: u64) -> Self {
        let entries: Vec<Entry> = blocks
            .iter()
            .map(|block| {
                let content =
                    crate::ui::estimate_block_height(block.kind, block.text.chars().count());
                Entry {
                    fingerprint: block_fingerprint(block),
                    slot: slot_gap(block.kind) + content,
                    numbered: block.kind == BlockKind::Numbered,
                }
            })
            .collect();
        let heights = FloatTree::build(&entries.iter().map(|entry| entry.slot).collect::<Vec<_>>());
        let numbered = CountTree::build(
            &entries
                .iter()
                .map(|entry| entry.numbered)
                .collect::<Vec<_>>(),
        );
        Self {
            page_id: page_id.to_string(),
            len: entries.len(),
            order_signature: signature,
            entries,
            heights,
            numbered,
        }
    }

    pub(crate) fn save(self, ctx: &Context) {
        let id = egui::Id::unique(("block_height_index", self.page_id.as_str()));
        ctx.data_mut(|data| data.insert_temp(id, self));
    }

    /// The slot offset where index `index` starts.
    pub(crate) fn prefix(&self, index: usize) -> f32 {
        self.heights.prefix(index.min(self.len))
    }

    pub(crate) fn total(&self) -> f32 {
        self.heights.total()
    }

    /// The first block that intersects `offset` from the content top.
    pub(crate) fn start_index(&self, offset: f32) -> usize {
        self.heights.lower_bound(offset.max(0.0)).min(self.len)
    }

    /// Number of ordered-list items before `index`.
    pub(crate) fn numbered_before(&self, index: usize) -> usize {
        self.numbered.prefix(index.min(self.len))
    }

    /// Store the measured content height for one rendered block.
    pub(crate) fn update(&mut self, index: usize, block: &Block, content_height: f32) {
        let Some(entry) = self.entries.get_mut(index) else {
            return;
        };
        let fingerprint = block_fingerprint(block);
        let slot = slot_gap(block.kind) + content_height;
        let numbered = block.kind == BlockKind::Numbered;
        if entry.fingerprint == fingerprint && (entry.slot - slot).abs() < 0.5 {
            return;
        }
        entry.fingerprint = fingerprint;
        let delta = slot - entry.slot;
        if delta.abs() >= f32::EPSILON {
            entry.slot = slot;
            self.heights.update(index, delta);
        }
        if entry.numbered != numbered {
            entry.numbered = numbered;
            self.numbered.update(index, numbered);
        }
    }
}

fn slot_gap(kind: BlockKind) -> f32 {
    if kind.is_heading() {
        BLOCK_HEADING_GAP + BLOCK_GAP
    } else {
        BLOCK_GAP
    }
}

/// A cheap content fingerprint; a change invalidates the cached height.
fn block_fingerprint(block: &Block) -> u64 {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    block.text.len().hash(&mut hasher);
    block.rich_text.len().hash(&mut hasher);
    std::mem::discriminant(&block.kind).hash(&mut hasher);
    block.children.len().hash(&mut hasher);
    block.checked.hash(&mut hasher);
    block.collapsed.hash(&mut hasher);
    block.cells.len().hash(&mut hasher);
    hasher.finish()
}

/// Cheap order signature: count, endpoints and every sixteenth block id. It
/// catches structural edits without hashing every id on every frame.
fn order_signature(blocks: &[Block]) -> u64 {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    blocks.len().hash(&mut hasher);
    for block in blocks.iter().step_by(16) {
        block.id.hash(&mut hasher);
    }
    if let Some(last) = blocks.last() {
        last.id.hash(&mut hasher);
    }
    hasher.finish()
}

#[derive(Clone, Debug, Default)]
struct FloatTree {
    tree: Vec<f32>,
}

impl FloatTree {
    fn build(values: &[f32]) -> Self {
        let mut tree = Self {
            tree: vec![0.0; values.len() + 1],
        };
        for (index, value) in values.iter().enumerate() {
            tree.update(index, *value);
        }
        tree
    }

    fn update(&mut self, index: usize, delta: f32) {
        let mut position = index + 1;
        while position < self.tree.len() {
            self.tree[position] += delta;
            position += position.isolate_lowest_one();
        }
    }

    fn prefix(&self, index: usize) -> f32 {
        let mut position = index.min(self.tree.len().saturating_sub(1));
        let mut sum = 0.0;
        while position > 0 {
            sum += self.tree[position];
            position -= position.isolate_lowest_one();
        }
        sum
    }

    fn total(&self) -> f32 {
        self.prefix(self.tree.len().saturating_sub(1))
    }

    /// Number of leading entries whose cumulative height is `<= target`.
    fn lower_bound(&self, target: f32) -> usize {
        let mut index = 0;
        let mut sum = 0.0;
        let mut bit = 1;
        while bit * 2 < self.tree.len() {
            bit *= 2;
        }
        while bit > 0 {
            let next = index + bit;
            if next < self.tree.len() && sum + self.tree[next] <= target {
                index = next;
                sum += self.tree[next];
            }
            bit /= 2;
        }
        index
    }
}

#[derive(Clone, Debug, Default)]
struct CountTree {
    tree: Vec<usize>,
}

impl CountTree {
    fn build(values: &[bool]) -> Self {
        let mut tree = Self {
            tree: vec![0; values.len() + 1],
        };
        for (index, value) in values.iter().enumerate() {
            tree.update(index, *value);
        }
        tree
    }

    fn update(&mut self, index: usize, value: bool) {
        let mut position = index + 1;
        while position < self.tree.len() {
            if value {
                self.tree[position] += 1;
            } else {
                self.tree[position] = self.tree[position].saturating_sub(1);
            }
            position += position.isolate_lowest_one();
        }
    }

    fn prefix(&self, index: usize) -> usize {
        let mut position = index.min(self.tree.len().saturating_sub(1));
        let mut sum = 0;
        while position > 0 {
            sum += self.tree[position];
            position -= position.isolate_lowest_one();
        }
        sum
    }
}
