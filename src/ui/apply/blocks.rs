//! Command family: blocks.

use crate::model::{Block, BlockKind, TableRow};
use uuid::Uuid;

use super::super::app::NotionApp;
use super::super::commands::Command;
impl NotionApp {
    pub(super) fn apply_blocks(&mut self, command: Command) {
        match command {
            Command::OpenSlash { block, pos } => {
                if let Some(b) = self.find_block_mut(&block) {
                    b.text = b.text.trim_start_matches('/').trim_start().to_string();
                }
                self.slash_for = Some(block);
                self.slash_pos = pos;
            }
            Command::CloseSlash => self.slash_for = None,
            Command::SetBlockKind(block, kind) => {
                if let Some(b) = self.find_block_mut(&block) {
                    b.set_kind(kind);
                    if kind.is_divider() {
                        b.text.clear();
                    } else {
                        b.text = b.text.trim_start_matches('/').trim_start().to_string();
                    }
                }
                self.slash_for = None;
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
            }
            Command::AppendBlock => {
                if let Some(page) = self.current_page_mut() {
                    page.blocks.push(Block::new(
                        Uuid::new_v4().to_string(),
                        BlockKind::Paragraph,
                        "",
                    ));
                }
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
                }
            }
            Command::StartBlockDrag(id) => {
                self.drag_from = Some(id);
                self.drag_to = None;
            }
            Command::HoverBlockDrag(id) => {
                // Only track a target while a drag is actually in progress;
                // this also prevents a late hover command (pushed later in
                // the same frame as the drop) from leaving stale state.
                if self.drag_from.is_some() && self.drag_from.as_deref() != Some(id.as_str()) {
                    self.drag_to = Some(id);
                }
            }
            Command::DropBlock => {
                let from = self.drag_from.take();
                let to = self.drag_to.take();
                if let (Some(from), Some(to)) = (from, to)
                    && from != to
                    && let Some(page) = self.current_page_mut()
                    && let Some(from_idx) = page.blocks.iter().position(|b| b.id == from)
                {
                    let block = page.blocks.remove(from_idx);
                    let target = page
                        .blocks
                        .iter()
                        .position(|b| b.id == to)
                        .map_or(page.blocks.len(), |i| i + 1);
                    page.blocks.insert(target.min(page.blocks.len()), block);
                }
            }
            Command::DeleteBlock(block_id) => {
                if let Some(page) = self.current_page_mut() {
                    page.blocks.retain(|b| b.id != block_id);
                }
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
            other => debug_assert!(false, "unexpected blocks command: {other:?}"),
        }
    }
}
