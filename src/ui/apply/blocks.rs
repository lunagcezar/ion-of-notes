//! Command family: blocks.

use crate::model::{Block, BlockKind, TableRow};
use std::collections::HashSet;
use uuid::Uuid;

use super::super::app::NotionApp;
use super::super::block_selection::{toggle_block_format, toggle_text_format};
use super::super::commands::Command;
impl NotionApp {
    pub(super) fn apply_blocks(&mut self, command: Command, ctx: &egui::Context) {
        match command {
            Command::OpenSlash { block, pos, query } => {
                self.slash_for = Some(block);
                self.slash_pos = pos;
                self.slash_query = query;
            }
            Command::CloseSlash => {
                // Dismissing keeps the typed "/" text and stops the menu from
                // reopening until the slash is removed (spec-057).
                self.slash_dismissed = self.slash_for.take();
                self.slash_query.clear();
            }
            Command::ClearSlashDismissed => self.slash_dismissed = None,
            Command::SetBlockKind(block, kind) => {
                if let Some(b) = self.find_block_mut(&block) {
                    b.set_kind(kind);
                    if kind.is_divider() || b.text.starts_with('/') {
                        b.text.clear();
                    }
                }
                self.slash_for = None;
                self.slash_query.clear();
            }
            Command::ApplyMarkdown {
                block,
                kind,
                prefix,
            } => {
                if let Some(b) = self.find_block_mut(&block) {
                    if let Some(rest) = b.text.strip_prefix(&prefix) {
                        b.text = rest.to_string();
                    }
                    b.set_kind(kind);
                    if kind.is_divider() {
                        b.text.clear();
                    }
                }
            }
            Command::InsertBlockAfter(block_id) => {
                if let Some(page) = self.current_page_mut()
                    && let Some(index) = page.blocks.iter().position(|b| b.id == block_id)
                {
                    page.blocks.insert(
                        index + 1,
                        Block::new(Uuid::new_v4().to_string(), BlockKind::Paragraph, ""),
                    );
                }
                self.sync_crdt_structure();
            }
            Command::AppendBlock => {
                if let Some(page) = self.current_page_mut() {
                    page.blocks.push(Block::new(
                        Uuid::new_v4().to_string(),
                        BlockKind::Paragraph,
                        "",
                    ));
                }
                self.sync_crdt_structure();
            }
            Command::AddBlock { block, above, pos } => {
                // The "+" beside a block adds a new block below (or above
                // with Alt) and immediately opens the block menu for it.
                let new_id = Uuid::new_v4().to_string();
                if let Some(page) = self.current_page_mut()
                    && let Some(index) = page.blocks.iter().position(|b| b.id == block)
                {
                    let at = if above { index } else { index + 1 };
                    page.blocks
                        .insert(at, Block::new(new_id.clone(), BlockKind::Paragraph, ""));
                    self.slash_for = Some(new_id);
                    self.slash_pos = pos;
                    self.slash_query.clear();
                }
                self.sync_crdt_structure();
            }
            Command::StartBlockDrag(id) => {
                self.drag_from = Some(id);
                self.drag_to = None;
                self.drag_before = false;
            }
            Command::HoverBlockDrag { block, before } => {
                // Only track a target while a drag is actually in progress;
                // this also prevents a late hover command (pushed later in
                // the same frame as the drop) from leaving stale state.
                if self.drag_from.is_some() && self.drag_from.as_deref() != Some(block.as_str()) {
                    self.drag_to = Some(block);
                    self.drag_before = before;
                }
            }
            Command::DropBlock => {
                let from = self.drag_from.take();
                let to = self.drag_to.take();
                let before = self.drag_before;
                if let Some(from) = from
                    && let Some(page) = self.current_page_mut()
                    && let Some(from_idx) = page.blocks.iter().position(|b| b.id == from)
                {
                    if to.as_deref() == Some(from.as_str()) {
                        return;
                    }
                    let block = page.blocks.remove(from_idx);
                    let target = to
                        .as_deref()
                        .and_then(|id| page.blocks.iter().position(|block| block.id == id));
                    let target =
                        crate::ui::block_ops::drop_index(page.blocks.len(), target, before);
                    page.blocks.insert(target, block);
                }
                self.sync_crdt_structure();
            }
            Command::CancelBlockDrag => {
                self.drag_from = None;
                self.drag_to = None;
                self.drag_before = false;
            }
            Command::SplitBlock(block_id) => {
                if let Some(page) = self.current_page_mut()
                    && let Some(index) = page.blocks.iter().position(|b| b.id == block_id)
                    && let Some(at) = crate::ui::block_ops::split_index(page.blocks.len(), index)
                {
                    let new_id = Uuid::new_v4().to_string();
                    page.blocks
                        .insert(at, Block::new(new_id.clone(), BlockKind::Paragraph, ""));
                    self.focus_block = Some(new_id);
                }
                self.sync_crdt_structure();
            }
            Command::ClearBlockFocus => self.focus_block = None,
            Command::DeleteBlock(block_id) => {
                if let Some(page) = self.current_page_mut() {
                    page.blocks.retain(|b| b.id != block_id);
                }
                self.sync_crdt_structure();
            }
            Command::ToggleTodo(block_id) => {
                if let Some(b) = self.find_block_mut(&block_id) {
                    b.toggle_todo();
                }
            }
            Command::CycleStatus(row_id) => {
                for page in &mut self.workspace.pages {
                    for row in &mut page.rows {
                        if row.id == row_id {
                            row.status = match row.status.as_str() {
                                "Not started" => "In progress",
                                "In progress" => "Done",
                                _ => "Not started",
                            }
                            .to_string();
                        }
                    }
                }
            }
            Command::AddRow => {
                if let Some(page) = self.current_page_mut() {
                    page.rows.push(TableRow::new(
                        Uuid::new_v4().to_string(),
                        "New task",
                        "Not started",
                        "",
                        "",
                    ));
                }
            }
            Command::SelectAllBlocks => {
                self.page_selection = self
                    .workspace
                    .page(&self.current_page)
                    .map(|page| page.blocks.iter().map(|b| b.id.clone()).collect())
                    .unwrap_or_default();
                self.block_selecting = false;
                self.block_toolbar_menu = false;
            }
            Command::ExtendBlockSelection(id) => {
                if !self.page_selection.contains(&id) {
                    self.page_selection.push(id);
                }
                self.block_selecting = true;
            }
            Command::ClearBlockSelection => {
                self.page_selection.clear();
                self.block_selecting = false;
                self.block_toolbar_menu = false;
            }
            Command::SetSelectedBlocksKind(kind) => {
                let selection: HashSet<String> = self.page_selection.iter().cloned().collect();
                if let Some(page) = self.current_page_mut() {
                    for block in &mut page.blocks {
                        if selection.contains(&block.id) {
                            block.set_kind(kind);
                            if kind.is_divider() {
                                block.text.clear();
                            }
                        }
                    }
                }
                self.block_toolbar_menu = false;
                self.sync_crdt_structure();
            }
            Command::ToggleSelectedBlocksFormat(format) => {
                let selection: HashSet<String> = self.page_selection.iter().cloned().collect();
                if let Some(page) = self.current_page_mut() {
                    for block in &mut page.blocks {
                        if selection.contains(&block.id) {
                            toggle_block_format(block, format);
                        }
                    }
                }
            }
            Command::DuplicateBlockSelection => {
                let selection: HashSet<String> = self.page_selection.iter().cloned().collect();
                let mut duplicates = Vec::new();
                if let Some(page) = self.current_page_mut() {
                    let mut index = 0;
                    while index < page.blocks.len() {
                        if selection.contains(&page.blocks[index].id) {
                            let mut copy = page.blocks[index].clone();
                            copy.id = Uuid::new_v4().to_string();
                            page.blocks.insert(index + 1, copy);
                            duplicates.push(page.blocks[index + 1].id.clone());
                            index += 2;
                        } else {
                            index += 1;
                        }
                    }
                }
                if duplicates.is_empty() {
                    self.page_selection.clear();
                } else {
                    self.page_selection = duplicates;
                }
                self.block_selecting = false;
                self.block_toolbar_menu = false;
                self.sync_crdt_structure();
            }
            Command::DeleteBlockSelection => {
                let selection: HashSet<String> = self.page_selection.iter().cloned().collect();
                if let Some(page) = self.current_page_mut() {
                    page.blocks.retain(|block| !selection.contains(&block.id));
                    if page.blocks.is_empty() {
                        page.blocks.push(Block::new(
                            Uuid::new_v4().to_string(),
                            BlockKind::Paragraph,
                            "",
                        ));
                    }
                }
                self.page_selection.clear();
                self.block_selecting = false;
                self.block_toolbar_menu = false;
                self.sync_crdt_structure();
            }
            Command::ToggleBlockToolbarMenu => {
                self.block_toolbar_menu = !self.block_toolbar_menu;
            }
            Command::CloseBlockToolbarMenu => self.block_toolbar_menu = false,
            Command::ToggleBlockTextFormat {
                block,
                start,
                end,
                format,
            } => {
                if let Some(block) = self.find_block_mut(&block) {
                    toggle_text_format(block, start, end, format);
                }
            }
            Command::CopyBlockText { block, start, end } => {
                if let Some(block) = self.workspace.block(&block) {
                    let chars: Vec<char> = block.text.chars().collect();
                    let start = start.min(chars.len());
                    let end = end.min(chars.len());
                    let (start, end) = if start <= end {
                        (start, end)
                    } else {
                        (end, start)
                    };
                    if start < end {
                        ctx.copy_text(chars[start..end].iter().collect());
                    }
                }
            }
            Command::CopyBlockSelection => {
                if let Some(page) = self.workspace.page(&self.current_page) {
                    let text = page
                        .blocks
                        .iter()
                        .filter(|b| self.page_selection.contains(&b.id))
                        .map(|b| b.text.as_str())
                        .collect::<Vec<_>>()
                        .join("\n");
                    if !text.is_empty() {
                        ctx.copy_text(text);
                    }
                }
            }
            other => debug_assert!(false, "unexpected blocks command: {other:?}"),
        }
    }
}
