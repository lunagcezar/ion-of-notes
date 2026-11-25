//! Pure helpers for two-stage select-all, page-level block selection and
//! whole-block formatting (spec-157).
//!
//! The UI reads these decisions; keeping them here makes the keyboard and
//! toolbar behavior testable without opening a window.

use crate::model::{Annotations, Block, BlockKind, RichText};

/// What `Ctrl+A` should do in the focused block editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectAllStage {
    /// Leave the key to the text editor: select the block's text.
    Text,
    /// Promote the selection to every block on the page.
    Blocks,
}

/// Decide the `Ctrl+A` stage from the block text, the current character range
/// and whether a page-level selection is already active.
///
/// The range is a pair of character indices in either order; a range that
/// covers the whole block text (or an empty block) promotes immediately.
pub fn select_all_stage(
    text_len: usize,
    selected_chars: Option<(usize, usize)>,
    page_selection_active: bool,
) -> SelectAllStage {
    if page_selection_active || text_len == 0 {
        return SelectAllStage::Blocks;
    }
    match selected_chars {
        Some((start, end)) => {
            let (start, end) = if start <= end {
                (start, end)
            } else {
                (end, start)
            };
            if start == 0 && end >= text_len {
                SelectAllStage::Blocks
            } else {
                SelectAllStage::Text
            }
        }
        None => SelectAllStage::Text,
    }
}

/// The whole-block annotations offered by the selection toolbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockFormat {
    Bold,
    Italic,
    Underline,
    Strikethrough,
    Code,
}

impl BlockFormat {
    /// Short button label used by the toolbar.
    pub fn label(self) -> &'static str {
        match self {
            BlockFormat::Bold => "B",
            BlockFormat::Italic => "I",
            BlockFormat::Underline => "U",
            BlockFormat::Strikethrough => "S",
            BlockFormat::Code => "</>",
        }
    }

    /// Accessible name for tooltips.
    pub fn name(self) -> &'static str {
        match self {
            BlockFormat::Bold => "Bold",
            BlockFormat::Italic => "Italic",
            BlockFormat::Underline => "Underline",
            BlockFormat::Strikethrough => "Strikethrough",
            BlockFormat::Code => "Code",
        }
    }
}

fn has_annotation(annotations: &Annotations, format: BlockFormat) -> bool {
    match format {
        BlockFormat::Bold => annotations.bold,
        BlockFormat::Italic => annotations.italic,
        BlockFormat::Underline => annotations.underline,
        BlockFormat::Strikethrough => annotations.strikethrough,
        BlockFormat::Code => annotations.code,
    }
}

fn set_annotation(annotations: &mut Annotations, format: BlockFormat, value: bool) {
    match format {
        BlockFormat::Bold => annotations.bold = value,
        BlockFormat::Italic => annotations.italic = value,
        BlockFormat::Underline => annotations.underline = value,
        BlockFormat::Strikethrough => annotations.strikethrough = value,
        BlockFormat::Code => annotations.code = value,
    }
}

/// True when every rich-text span of the block carries the annotation.
pub fn block_has_format(block: &Block, format: BlockFormat) -> bool {
    let mut any = false;
    for span in &block.rich_text {
        if !has_annotation(span.annotations(), format) {
            return false;
        }
        any = true;
    }
    any
}

/// Toggle one annotation on every span of a block. The plain `text`
/// projection is never changed, so the block remains plain-text editable.
pub fn toggle_block_format(block: &mut Block, format: BlockFormat) {
    let enable = !block_has_format(block, format);
    if block.rich_text.is_empty() {
        block.rich_text = vec![RichText::text(block.text.clone())];
    }
    for span in &mut block.rich_text {
        set_annotation(span.annotations_mut(), format, enable);
    }
}

fn annotations_by_char(block: &Block) -> Vec<Annotations> {
    let len = block.text.chars().count();
    let mut annotations = Vec::with_capacity(len);
    for (text, span) in rich_spans(block) {
        for _ in text.chars() {
            if annotations.len() < len {
                annotations.push(span.clone());
            }
        }
    }
    annotations.resize(len, Annotations::default());
    annotations.truncate(len);
    annotations
}

/// True when every selected character of the block carries the annotation.
pub fn text_has_format(block: &Block, start: usize, end: usize, format: BlockFormat) -> bool {
    let len = block.text.chars().count();
    let (start, end) = (start.min(len), end.min(len));
    let (start, end) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };
    if start == end {
        return false;
    }
    annotations_by_char(block)[start..end]
        .iter()
        .all(|annotations| has_annotation(annotations, format))
}

/// Toggle one annotation on the selected character range of a block.
pub fn toggle_text_format(block: &mut Block, start: usize, end: usize, format: BlockFormat) {
    let len = block.text.chars().count();
    let (start, end) = (start.min(len), end.min(len));
    let (start, end) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };
    if start == end {
        return;
    }
    let chars: Vec<char> = block.text.chars().collect();
    let mut annotations = annotations_by_char(block);
    let enable = !annotations[start..end]
        .iter()
        .all(|annotations| has_annotation(annotations, format));
    for annotations in &mut annotations[start..end] {
        set_annotation(annotations, format, enable);
    }

    let mut spans: Vec<RichText> = Vec::new();
    for (character, annotations) in chars.iter().zip(annotations) {
        if let Some(last) = spans.last_mut()
            && *last.annotations() == annotations
        {
            let mut text = last.plain_text().to_string();
            text.push(*character);
            last.set_plain_text(text);
        } else {
            let mut span = RichText::text(character.to_string());
            *span.annotations_mut() = annotations;
            spans.push(span);
        }
    }
    block.rich_text = spans;
    block.sync_text_from_rich_text();
}

/// The kind shared by every block, or `None` for an empty or mixed selection.
pub fn common_kind(blocks: &[Block]) -> Option<BlockKind> {
    let mut iter = blocks.iter();
    let first = iter.next()?.kind;
    iter.all(|block| block.kind == first).then_some(first)
}

/// Kinds the block renderer supports and the toolbar can therefore offer.
/// The reference menu also lists page, equation, toggle-heading and column
/// entries that need structures this editor does not render yet.
pub fn turn_into_kinds() -> &'static [BlockKind] {
    &[
        BlockKind::Paragraph,
        BlockKind::Heading1,
        BlockKind::Heading2,
        BlockKind::Heading3,
        BlockKind::Heading4,
        BlockKind::Bulleted,
        BlockKind::Numbered,
        BlockKind::Todo,
        BlockKind::Toggle,
        BlockKind::Code,
        BlockKind::Quote,
        BlockKind::Callout,
        BlockKind::Divider,
    ]
}

/// Annotated spans for the text currently shown by the block editor.
///
/// Annotations are used only while the stored spans still concatenate to the
/// editable `text`. After a plain-text edit the stale spans are ignored instead
/// of styling the wrong characters.
pub(crate) fn rich_spans(block: &Block) -> Vec<(String, Annotations)> {
    let joined: String = block
        .rich_text
        .iter()
        .map(RichText::plain_text)
        .collect::<Vec<_>>()
        .join("");
    if block.text.is_empty() {
        Vec::new()
    } else if !block.rich_text.is_empty() && joined == block.text {
        block
            .rich_text
            .iter()
            .map(|span| (span.plain_text().to_string(), span.annotations().clone()))
            .collect()
    } else {
        vec![(block.text.clone(), Annotations::default())]
    }
}
