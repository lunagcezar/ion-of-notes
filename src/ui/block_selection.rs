//! Pure helpers for two-stage select-all, page-level block selection and
//! rich text/format operations on a character range (specs 157-158).
//!
//! The UI reads these decisions and transformations; keeping them here makes
//! the keyboard, toolbar, comment and reaction behavior testable without a
//! window.

use crate::model::{
    Annotations, Block, BlockKind, BlockReaction, Equation, RichText, TextContent, TextLink,
};

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

/// The whole-block or selected-text annotations offered by the toolbar.
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

/// One documented inline-editor shortcut (spec-158).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineAction {
    Bold,
    Italic,
    Underline,
    Strikethrough,
    Code,
    Equation,
    ColorPicker,
    ClearFormatting,
    LinkPicker,
    CommentComposer,
}

/// Map an editor shortcut to its action. Only the exact combinations listed in
/// the reference are accepted, so typing `Ctrl+B` never falls through to a
/// broader binding.
pub fn inline_action(ctrl: bool, shift: bool, alt: bool, key: &str) -> Option<InlineAction> {
    if !ctrl {
        return None;
    }
    match (shift, alt, key) {
        (true, false, "s" | "x") => Some(InlineAction::Strikethrough),
        (true, false, "e") => Some(InlineAction::Equation),
        (true, false, "h") => Some(InlineAction::ColorPicker),
        (true, false, "m") => Some(InlineAction::CommentComposer),
        (false, true, "\\") => Some(InlineAction::ClearFormatting),
        (false, false, "b") => Some(InlineAction::Bold),
        (false, false, "i") => Some(InlineAction::Italic),
        (false, false, "u") => Some(InlineAction::Underline),
        (false, false, "e") => Some(InlineAction::Code),
        (false, false, "k") => Some(InlineAction::LinkPicker),
        _ => None,
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

/// One rich-text run as seen by the editor: text plus all inline attributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RichRun {
    pub text: String,
    pub annotations: Annotations,
    pub link: Option<String>,
    pub equation: bool,
}

impl RichRun {
    fn plain(text: String) -> Self {
        Self {
            text,
            annotations: Annotations::default(),
            link: None,
            equation: false,
        }
    }
}

/// Annotated runs for the text currently shown by the block editor.
///
/// Stored spans are used only while they still concatenate to the editable
/// `text`. After a plain-text edit the stale spans are ignored instead of
/// styling the wrong characters.
pub(crate) fn rich_runs(block: &Block) -> Vec<RichRun> {
    if block.text.is_empty() {
        return Vec::new();
    }
    let joined: String = block
        .rich_text
        .iter()
        .map(RichText::plain_text)
        .collect::<Vec<_>>()
        .join("");
    if block.rich_text.is_empty() || joined != block.text {
        return vec![RichRun::plain(block.text.clone())];
    }
    block
        .rich_text
        .iter()
        .map(|span| RichRun {
            text: span.plain_text().to_string(),
            annotations: span.annotations().clone(),
            link: span.link().map(str::to_string),
            equation: span.is_equation(),
        })
        .collect()
}

fn normalize_range(start: usize, end: usize, len: usize) -> (usize, usize) {
    let start = start.min(len);
    let end = end.min(len);
    if start <= end {
        (start, end)
    } else {
        (end, start)
    }
}

fn merge_runs(runs: Vec<RichRun>) -> Vec<RichRun> {
    let mut merged: Vec<RichRun> = Vec::with_capacity(runs.len());
    for run in runs {
        if let Some(last) = merged.last_mut()
            && last.annotations == run.annotations
            && last.link == run.link
            && last.equation == run.equation
        {
            last.text.push_str(&run.text);
            continue;
        }
        merged.push(run);
    }
    merged
}

fn rebuild(block: &mut Block, runs: Vec<RichRun>) {
    block.rich_text = merge_runs(runs)
        .into_iter()
        .map(|run| {
            if run.equation {
                RichText::Equation {
                    equation: Equation {
                        expression: run.text.clone(),
                    },
                    annotations: run.annotations,
                    plain_text: run.text,
                    href: None,
                }
            } else if let Some(url) = run.link {
                RichText::Text {
                    text: TextContent {
                        content: run.text.clone(),
                        link: Some(TextLink { url: url.clone() }),
                    },
                    annotations: run.annotations,
                    plain_text: run.text,
                    href: Some(url),
                }
            } else {
                RichText::Text {
                    text: TextContent {
                        content: run.text.clone(),
                        link: None,
                    },
                    annotations: run.annotations,
                    plain_text: run.text,
                    href: None,
                }
            }
        })
        .collect();
    block.sync_text_from_rich_text();
}

fn replace_range(block: &mut Block, start: usize, end: usize, mut edit: impl FnMut(&mut RichRun)) {
    let len = block.text.chars().count();
    let (start, end) = normalize_range(start, end, len);
    if start == end {
        return;
    }
    let mut runs = Vec::new();
    let mut position = 0usize;
    for run in rich_runs(block) {
        let chars: Vec<char> = run.text.chars().collect();
        let run_len = chars.len();
        let run_start = position;
        let run_end = position + run_len;
        position = run_end;
        if run_end <= start || run_start >= end {
            runs.push(run);
            continue;
        }
        let slice_start = start.saturating_sub(run_start).min(run_len);
        let slice_end = (end.min(run_end)) - run_start;
        if slice_start > 0 {
            let mut before = run.clone();
            before.text = chars[..slice_start].iter().collect();
            runs.push(before);
        }
        let mut middle = run.clone();
        middle.text = chars[slice_start..slice_end].iter().collect();
        edit(&mut middle);
        if !middle.text.is_empty() {
            runs.push(middle);
        }
        if slice_end < run_len {
            let mut after = run;
            after.text = chars[slice_end..].iter().collect();
            runs.push(after);
        }
    }
    rebuild(block, runs);
}

fn all_in_range(
    block: &Block,
    start: usize,
    end: usize,
    predicate: impl Fn(&RichRun) -> bool,
) -> bool {
    let len = block.text.chars().count();
    let (start, end) = normalize_range(start, end, len);
    if start == end {
        return false;
    }
    let mut position = 0usize;
    let mut seen = false;
    for run in rich_runs(block) {
        let run_len = run.text.chars().count();
        let run_start = position;
        let run_end = position + run_len;
        position = run_end;
        if run_start.max(start) < run_end.min(end) {
            seen = true;
            if !predicate(&run) {
                return false;
            }
        }
    }
    seen
}

/// True when every selected character carries the annotation.
pub fn text_has_format(block: &Block, start: usize, end: usize, format: BlockFormat) -> bool {
    all_in_range(block, start, end, |run| {
        has_annotation(&run.annotations, format)
    })
}

/// True when every selected rich-text span of the block carries the annotation.
pub fn block_has_format(block: &Block, format: BlockFormat) -> bool {
    let runs = rich_runs(block);
    !runs.is_empty()
        && runs
            .iter()
            .all(|run| has_annotation(&run.annotations, format))
}

/// Toggle one annotation on every run of a block.
pub fn toggle_block_format(block: &mut Block, format: BlockFormat) {
    let enable = !block_has_format(block, format);
    let len = block.text.chars().count();
    replace_range(block, 0, len, |run| {
        set_annotation(&mut run.annotations, format, enable);
    });
}

/// Toggle one annotation on the selected character range.
pub fn toggle_text_format(block: &mut Block, start: usize, end: usize, format: BlockFormat) {
    let enable = !text_has_format(block, start, end, format);
    replace_range(block, start, end, |run| {
        set_annotation(&mut run.annotations, format, enable);
    });
}

/// The color shared by the selected range, if any.
pub fn text_color(block: &Block, start: usize, end: usize) -> Option<String> {
    let len = block.text.chars().count();
    let (start, end) = normalize_range(start, end, len);
    if start == end {
        return None;
    }
    let mut color: Option<String> = None;
    let mut position = 0usize;
    for run in rich_runs(block) {
        let run_len = run.text.chars().count();
        let run_start = position;
        let run_end = position + run_len;
        position = run_end;
        if run_start.max(start) < run_end.min(end) {
            match &color {
                None => color = Some(run.annotations.color.clone()),
                Some(existing) if existing != &run.annotations.color => return None,
                Some(_) => {}
            }
        }
    }
    color
}

/// Set the text color on the selected range.
pub fn set_text_color(block: &mut Block, start: usize, end: usize, color: &str) {
    replace_range(block, start, end, |run| {
        run.annotations.color = color.to_string();
    });
}

/// True when every selected character has a link.
pub fn text_has_link(block: &Block, start: usize, end: usize) -> bool {
    all_in_range(block, start, end, |run| run.link.is_some())
}

/// Wrap the selected range in a link and keep its other attributes.
pub fn set_text_link(block: &mut Block, start: usize, end: usize, url: &str) {
    replace_range(block, start, end, |run| {
        run.link = Some(url.to_string());
        run.equation = false;
    });
}

/// True when every selected character is marked as an equation.
pub fn text_is_equation(block: &Block, start: usize, end: usize) -> bool {
    all_in_range(block, start, end, |run| run.equation)
}

/// Mark the selected range as an equation.
pub fn mark_text_equation(block: &mut Block, start: usize, end: usize) {
    replace_range(block, start, end, |run| {
        run.equation = true;
        run.link = None;
    });
}

/// Clear annotations, colors, links and equation marks on the selected range.
pub fn clear_text_format(block: &mut Block, start: usize, end: usize) {
    replace_range(block, start, end, |run| {
        run.annotations = Annotations::default();
        run.link = None;
        run.equation = false;
    });
}

/// Add or remove a reaction for this range/emoji/user. Returns `true` when the
/// reaction was added.
pub fn toggle_block_reaction(
    block: &mut Block,
    start: usize,
    end: usize,
    emoji: &str,
    user: &str,
) -> bool {
    let position = block.reactions.iter().position(|reaction| {
        reaction.emoji == emoji
            && reaction.start == start
            && reaction.end == end
            && reaction.created_by.as_deref() == Some(user)
    });
    if let Some(position) = position {
        block.reactions.remove(position);
        false
    } else {
        block.reactions.push(BlockReaction {
            emoji: emoji.to_string(),
            start,
            end,
            created_by: Some(user.to_string()),
        });
        true
    }
}

/// Grouped reactions for rendering: `(emoji, start, end, count)`.
pub fn reaction_counts(block: &Block) -> Vec<(String, usize, usize, usize)> {
    let mut grouped: Vec<(String, usize, usize, usize)> = Vec::new();
    for reaction in &block.reactions {
        if let Some(entry) = grouped.iter_mut().find(|(emoji, start, end, _)| {
            emoji == &reaction.emoji && *start == reaction.start && *end == reaction.end
        }) {
            entry.3 += 1;
        } else {
            grouped.push((reaction.emoji.clone(), reaction.start, reaction.end, 1));
        }
    }
    grouped
}

/// The kind shared by every block, or `None` for an empty or mixed selection.
pub fn common_kind(blocks: &[Block]) -> Option<BlockKind> {
    let mut iter = blocks.iter();
    let first = iter.next()?.kind;
    iter.all(|block| block.kind == first).then_some(first)
}

/// Kinds the block renderer supports and the toolbar can therefore offer.
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
