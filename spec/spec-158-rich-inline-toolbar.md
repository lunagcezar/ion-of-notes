# spec-158 - Rich inline toolbar, comments, reactions and select-all delete

- Status: implemented (2026-11-25)
- Related: `spec-032`, `spec-035`, `spec-057`, `spec-157`
- Reference images: the six Notion pictures supplied in the session.

## Purpose and scope

Make the selection toolbar actually edit the document and complete the
Notion-style selection workflow:

1. Every toolbar control applies to the selected text or blocks, including
   text color, bold, italic, underline, strikethrough, code, links, equations
   and clear formatting.
2. The three-dot menu exposes the secondary actions, and the comment button
   opens a floating composer with text, user mentions and file attachments.
3. The reaction button opens the existing emoji catalog in a compact picker and
   attaches the chosen emoji to the selected text range.
4. The listed keyboard shortcuts work inside the editor.
5. `Ctrl+A` twice followed by `Backspace`/`Delete` removes every selected
   block.
6. Structural toolbar actions participate in undo, so `Ctrl+Z` reverts a
   deleted/duplicated/restyled selection.

Out of scope:

- Rendering real LaTeX equations (the expression is stored and styled).
- Background/highlight colors beyond the existing `Annotations` model.
- A native OS file dialog; attachments use a path field and the existing
  upload API.

## Evidence

The person reported that only the delete button had an effect (and that undo
did not restore the block) and supplied six pictures showing:

1. the two-row toolbar with type, color, `B`, `I`, `U`, `Tx`, link, comment,
   code, check/equation and overflow;
2. the text-color and background-color picker;
3. the link popover with a field and recent pages;
4. the floating comment composer with attach and mention controls;
5. the reaction picker reusing the emoji grid;
6. the three-dot menu and the full turn-into list.

The Notion shortcut list in the message is the binding table.

## Acceptance criteria

A1. **Given** selected text, **when** a format, color, link, equation or clear
control is used, **then** the change is visible immediately, survives CRDT
projection while the text is unchanged, and is persisted normally.

A2. **Given** selected text, **when** `Ctrl+B`, `Ctrl+I`, `Ctrl+U`,
`Ctrl+Shift+S`/`Ctrl+Shift+X`, `Ctrl+E`, `Ctrl+Shift+E`, `Ctrl+Shift+H`,
`Ctrl+Alt+\`, `Ctrl+K` or `Ctrl+Shift+M` is pressed, **then** the matching
action runs (toggle format, mark equation, open color, clear, open link or open
comment).

A3. **Given** the toolbar, **when** the color/text-style/link/comment/reaction
or overflow control is chosen, **then** a compact popover opens next to the
toolbar, only one panel is open at a time, and an outside click or `Esc` closes
it.

A4. **Given** the comment composer, **when** it is submitted, **then** a comment
anchored to the selected range is created with the entered text, optional
`@member` mention and optional attached file.

A5. **Given** the reaction picker, **when** an emoji is chosen, **then** it is
attached to the selected range, rendered as a chip below the block and can be
removed by clicking the chip again.

A6. **Given** a page-level block selection, **when** `Backspace` or `Delete` is
pressed, **then** every selected block is removed (leaving one empty paragraph
when the page would otherwise be empty).

A7. **Given** any structural toolbar action (delete, duplicate, kind, format,
color, link, equation, reaction), **when** `Ctrl+Z` is pressed immediately
afterwards, **then** the document returns to its previous state.

A8. **Given** the block selection toolbar, **when** the type menu is opened,
**then** the reference list remains available for every kind the renderer
supports and the choice applies to all selected blocks.

## Test scenarios (written before implementation)

- `tests/model.rs`: a block reaction round-trips through JSON; changing only the
  plain text clears stale annotations, while an unchanged projection keeps them.
- `tests/ui.rs`: text color, clear formatting, link, equation and reaction
  helpers operate on the selected range and leave the rest untouched; the
  shortcut mapper returns the expected action for each documented combo.
- Manual/screenshot states: `selection-text`, `selection-text-color`,
  `selection-text-link`, `selection-comment`, `selection-react`,
  `selection-more`; the six supplied pictures are compared by eye.
- Regression: the full quality gate and the desktop UI tests.

## Design

- Model: `BlockReaction { emoji, start, end, created_by }` on `Block` (serde
  default, SQLite/JSON block rows carry it). `crdt_text::project_blocks` only
  resets annotations when the projected text actually changed.
- Pure `ui::block_selection` run model: `RichRun` keeps text, annotations,
  link, equation and async range edits; formatting, color, clear, link,
  equation and reactions build on one `replace_range` helper.
- Toolbar state: one `BlockToolbarPanel` enum (Type, Color, Link, Comment,
  React, More) on `NotionApp`/`ViewState`; commands toggle/close it.
- Emoji reuse: extract the page-icon emoji grid into `ui::emoji_picker` and use
  it for both the page icon and reactions.
- Comments: the floating composer uses the existing inline-comment fields plus
  a selected range, member list and attachment path; the upload uses the v2
  file-upload API with a byte-preserving client method.
- Undo: `Command::forces_undo_snapshot` forces one entry for document-changing
  commands even inside the typing debounce window.

Implementation notes (2026-11-25):

- Two causes made toolbar controls look inert: `crdt_text::project_blocks`
  replaced every single-span rich-text value on each sync, and a toolbar click
  made the block `TextEdit` surrender focus before the toolbar was rendered.
  The projection now uses `Block::apply_plain_text` (reset only on a real text
  change), and the last inline selection is persisted in app state
  (`SetInlineSelection` / `ClearInlineSelection`) while the pointer is over the
  toolbar or panel. An outside click clears it.
- The comment composer attaches a file by reading the path and sending the
  bytes through the v2 upload API; there is no native OS file dialog.
- The toolbar shortcuts are editor-local. In particular, `Ctrl+K` is left to
  the focused block editor (add link) instead of the global quick search; the
  shortcut collector skips it while the `block_editor_focused` flag is set.
- Mentions are inserted as `@Display Name` in the comment text (the model does
  not yet need a separate Mention span for comments).
- Equation spans are stored and styled (italic monospace) but not typeset with
  a math renderer.
- Preview states: `selection-text-color`, `selection-text-link`,
  `selection-comment`, `selection-react` and `selection-more`; selection
  previews open `Getting Started` so they remain useful after Home is edited.

## Plan / tasks

- [x] Write the failing model/UI tests and the spec.
- [x] Fix CRDT projection and add the run-based text helpers.
- [x] Add the toolbar panels, shortcuts and delete/backspace behavior.
- [x] Add comments, attachments, mentions and reactions using the shared emoji
      grid.
- [x] Capture the new preview states, run the gates and update docs/changelog.
