//! Inline comment thread projection for the page panel.
//!
//! The workspace stores v2 `CommentDto` records with an optional block/text
//! anchor. This module groups them per discussion and resolves the quoted block
//! text so the renderer stays independent of the comment model.

use crate::model::{
    Block, CommentAnchor, CommentDisplayName, CommentDto, CommentInput, ParentDto, RichText,
    Workspace,
};
use std::collections::BTreeMap;

/// One rendered comment inside an inline thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineCommentView {
    pub id: String,
    pub author: String,
    pub text: String,
    pub created_at: String,
}

/// One discussion anchored to a block range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineThreadView {
    pub discussion_id: String,
    pub block_id: String,
    pub excerpt: String,
    pub comments: Vec<InlineCommentView>,
}

/// Group anchored comments for one page by discussion, in block order.
pub fn inline_threads(workspace: &Workspace, page_id: &str) -> Vec<InlineThreadView> {
    let Some(page) = workspace.page(page_id) else {
        return Vec::new();
    };
    let block_order: BTreeMap<&str, usize> = page
        .blocks
        .iter()
        .enumerate()
        .map(|(index, block)| (block.id.as_str(), index))
        .collect();

    let mut threads: BTreeMap<(usize, u32, String), InlineThreadView> = BTreeMap::new();
    for comment in &workspace.comments {
        if comment_page(workspace, comment).as_deref() != Some(page_id) {
            continue;
        }
        let Some(anchor) = &comment.anchor else {
            continue;
        };
        let Some(block) = workspace.block(&anchor.block_id) else {
            continue;
        };
        let key = (
            block_order
                .get(anchor.block_id.as_str())
                .copied()
                .unwrap_or(usize::MAX),
            anchor.start as u32,
            comment.discussion_id.clone(),
        );
        let comment_view = InlineCommentView {
            id: comment.id.clone(),
            author: comment
                .display_name
                .as_ref()
                .map(|display| display.resolved_name.clone())
                .unwrap_or_else(|| comment.created_by.id.clone()),
            text: comment
                .rich_text
                .iter()
                .map(RichText::plain_text)
                .collect::<Vec<_>>()
                .join(""),
            created_at: comment.created_time.clone(),
        };
        threads
            .entry(key)
            .or_insert_with(|| InlineThreadView {
                discussion_id: comment.discussion_id.clone(),
                block_id: anchor.block_id.clone(),
                excerpt: excerpt(&block.text, anchor.start, anchor.end),
                comments: Vec::new(),
            })
            .comments
            .push(comment_view);
    }

    let mut threads: Vec<InlineThreadView> = threads.into_values().collect();
    for thread in &mut threads {
        thread
            .comments
            .sort_by(|a, b| a.created_at.cmp(&b.created_at));
    }
    threads
}

/// Build the API input for an inline comment spanning the whole block.
pub fn inline_comment_input(block: &Block, text: &str) -> CommentInput {
    CommentInput {
        parent: ParentDto::Block {
            block_id: block.id.clone(),
        },
        discussion_id: None,
        property_id: None,
        anchor: Some(CommentAnchor {
            block_id: block.id.clone(),
            start: 0,
            end: block.text.chars().count(),
        }),
        rich_text: vec![RichText::text(text)],
        attachments: Vec::new(),
        display_name: Some(CommentDisplayName::Custom {
            name: "Local user".to_string(),
        }),
    }
}

fn comment_page(workspace: &Workspace, comment: &CommentDto) -> Option<String> {
    match &comment.parent {
        ParentDto::Page { page_id } => Some(page_id.clone()),
        ParentDto::Block { block_id } => workspace.page_id_for_block(block_id),
        _ => None,
    }
}

fn excerpt(text: &str, start: usize, end: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    let start = start.min(chars.len());
    let end = end.clamp(start, chars.len());
    let selected: String = chars[start..end].iter().collect();
    let source = if selected.trim().is_empty() {
        &chars[..chars.len().min(80)]
    } else {
        &chars[start..end]
    };
    let mut excerpt: String = source.iter().take(80).collect();
    if source.len() > 80 {
        excerpt.push('…');
    }
    excerpt
}
