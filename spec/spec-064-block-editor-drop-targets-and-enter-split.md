# spec-064 - Block editor drop targets and Enter split

- Status: implemented (2026-11-12)
- Related: `spec-013`, `spec-060`
- Reference: `reference/block-type.png`

## Purpose and scope

The block editor has three interaction gaps:

1. Drag/drop only targets an existing block row, so the first line, the last
   line and the space between blocks cannot receive a drop.
2. Enter inserts a newline inside a block instead of creating a new block.
3. Empty blocks have no "Press / to insert blocks" placeholder.

This spec computes drop positions from the gaps between blocks, adds the
Notion-style Enter split and adds the localized placeholder.

## Acceptance criteria

B6. **Given** a block drag, **when** the pointer is above the first block,
between blocks or below the last block, **then** an insertion marker shows the
computed gap and releasing inserts the block at that position.

B7. **Given** an empty page with no blocks, **when** a block is dragged,
**then** dropping still appends it.

B8. **Given** a focused block, **when** Enter is pressed without Shift, **then**
a new empty block is created after it and receives focus. Shift+Enter keeps the
current newline behavior.

B9. **Given** an empty editable block, **when** it renders, **then** it shows
the localized "Press / to insert blocks" placeholder.

B10. **Given** the placeholder key, **when** the locale changes, **then** the
English, Portuguese and Esperanto translations are present.

## Test scenarios

- Pure insertion-index calculation for: above first, between two, below last,
  empty page, and same-block drop.
- `SplitBlock` creates a paragraph after the source and sets the focus target.
- Locale key completeness includes `block_placeholder`.
- Screenshots: `drop-first`, `drop-last`, `enter-split`,
  `empty-block-placeholder`.
- Full gate.

## Design

- Replace the target id with `(target_id, before)` in app state and commands.
  `render_block` writes its row rect and index to a frame-local drop map;
  `render_page` computes the nearest gap from the pointer y.
- Add `Command::SplitBlock(block_id)` and `focus_block: Option<String>`.
  `render_block` consumes Enter before the TextEdit when its edit id is
  focused, then requests focus on the new block.
- Add `block_placeholder` to all locales and pass it as the TextEdit hint for
  editable paragraph-like blocks.
- Keep Shift+Enter as the native newline.
