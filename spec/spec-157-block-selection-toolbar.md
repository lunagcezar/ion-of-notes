# spec-157 - Two-stage select-all and the block selection toolbar

- Status: implemented (2026-11-25)
- Related: `spec-032`, `spec-057`, `spec-064`
- Reference images: the three pictures supplied in the session (select-all
  inside a block, multi-block selection and the expanded block-type menu).

## Purpose and scope

Bring `Ctrl+A` closer to the editor the person compares against, and give the
page-level block selection an editing surface:

1. **Two-stage select-all.** Inside a focused block, the first `Ctrl+A` selects
   the whole text of that block (the native `TextEdit` behavior). A second
   `Ctrl+A`, while that text is still fully selected, promotes the selection to
   every block on the page.
2. **Selection toolbar.** While page-level block selection is active, a floating
   toolbar anchored beside the selected block(s) offers the block-type menu and
   the common text formats. The block-type menu lists the kinds the editor can
   render, marks the current kind and turns every selected block into the chosen
   kind.

Out of scope:

- Rich inline editing (links, mentions, equations) beyond the existing
  annotation model.
- New block structures the renderer does not implement (columns, toggle
  headings, page-in and block equations). The menu only offers kinds that
  render correctly; unsupported reference entries are not shown as dead rows.
- AI "Skills" actions from the first reference picture; Settings → AI remains
  the assistant surface.

## Evidence

The person supplied three pictures:

1. A single block with all text selected and a compact toolbar at its right:
   block-type control (`Text ▾`), `B` / `I` / `U` / strikethrough / code,
   link, comment and overflow controls.
2. A page-level multi-block selection: every selected row has a blue fill and
   outline across the content column, and the block controls are visible at the
   left edge.
3. The expanded block-type menu: Text, Heading 1-4, Page, Page in, Bulleted
   list, Numbered list, To-do list, Toggle list, Code, Quote, Callout, Block
   equation, Toggle heading 1-4 and column layouts.

Current code seams:

| Area | Code |
| ---- | ---- |
| Block editing, selection and Ctrl+A | `src/ui/render/content/blocks.rs` |
| Page body, selection union and toolbar anchor | `src/ui/render/content/page.rs` |
| Commands and state transitions | `src/ui/commands.rs`, `src/ui/apply/blocks.rs` |
| App state and view snapshot | `src/ui/app.rs`, `src/ui/view.rs` |
| Pure selection/format helpers | new `src/ui/block_selection.rs` |
| Selection toolbar | new `src/ui/render/content/toolbar.rs` |

## Acceptance criteria

A1. **Given** a focused block with the caret somewhere in its text, **when**
`Ctrl+A` is pressed once, **then** the entire text of that block is selected and
no page-level selection is started.

A2. **Given** a focused block whose entire text is already selected, **when**
`Ctrl+A` is pressed again, **then** every block on the current page is selected,
the text editor loses focus and the selection toolbar becomes visible.

A3. **Given** an empty focused block, **when** `Ctrl+A` is pressed, **then**
every block on the page is selected (there is no text to select first).

A4. **Given** a page-level block selection, **when** it renders, **then** each
selected block has a blue fill and outline across the content column and the
plus/six-dot controls are available at the left edge.

A5. **Given** a page-level block selection, **when** the toolbar is shown,
**then** it is anchored next to the selection and contains the block-type
control, `B`, `I`, `U`, strikethrough and code format controls, plus copy,
comment, duplicate and delete actions.

A5b. **Given** a text selection inside one block, **when** the toolbar is
shown, **then** it is anchored under the selection, the format controls apply
only to the selected characters, and copy/comment/delete act on that block.

A6. **Given** the block-type control, **when** it is opened, **then** a menu
lists the renderable kinds (Text, Heading 1-4, Bulleted list, Numbered list,
To-do list, Toggle list, Code, Quote, Callout, Divider), marks the kind shared
by the selection, and closes after a choice.

A7. **Given** selected blocks, **when** a kind is chosen from the menu, **then**
every selected block adopts that kind, the text is preserved (except a divider,
which clears it) and the document is persisted through the normal client path.

A8. **Given** selected blocks, **when** a format control is pressed, **then**
the annotation is toggled on every selected block, the rich-text projection and
the plain `text` field stay consistent, and the formatted text renders with the
matching style.

A9. **Given** selected blocks, **when** copy, duplicate or delete is pressed,
**then** the whole selection is copied as text, duplicated in place with new
ids, or removed respectively, and the selection is kept coherent (duplicates
become the new selection; delete clears it).

A10. **Given** a selection, **when** `Esc` or a click elsewhere is used, **then**
the selection and the toolbar clear without changing the document.

## Test scenarios (written before implementation)

- Pure unit tests in `tests/ui.rs`:
  - `select_all_stage` returns `Text` for a partial/absent range, `Blocks` for
    a full range, an empty block or an already active page selection.
  - `toggle_block_format` toggles each annotation and leaves `text` and the
    concatenated rich text unchanged; `block_has_format` reports the state.
  - `toggle_text_format` / `text_has_format` only change the selected character
    range, including a stale rich-text projection reset.
  - `common_kind` returns the shared kind and `None` for a mixed selection.
  - `turn_into_kinds` contains exactly the renderable kinds.
- Screenshot states: `ION_OF_NOTES_SHOW=selection` shows the page selection with
  the toolbar and its open type menu, and `selection-text` shows a text
  selection with the same toolbar; compare with the supplied pictures.
- Regression: desktop UI gate (`cargo test --features desktop --test ui --test
  fidelity --test ui_crdt --test screenshot`), core tests and clippy/fmt.

## Design

- `ui::block_selection` is a pure module: `SelectAllStage`,
  `select_all_stage(text_len, selected_chars, page_selection_active)`,
  `BlockFormat`, `toggle_block_format`, `block_has_format`, `common_kind`,
  `turn_into_kinds` and `rich_spans` (annotated spans only when they still
  match the block text).
- `blocks.rs` loads `TextEditState` before consuming `Ctrl+A`, so the first
  press is left to egui and the second press promotes to the page selection.
  It returns the selected block rect to the page renderer, paints the selection
  fill/outline and keeps the gutter controls visible for selected rows.
- `page.rs` unions the selected rects and calls the new toolbar renderer.
- `toolbar.rs` paints an `egui::Area` with compact buttons and a second area for
  the block-type menu; every action is a `Command`.
- New `Command` variants: `SetSelectedBlocksKind`, `ToggleSelectedBlocksFormat`,
  `DuplicateBlockSelection`, `DeleteBlockSelection`, `ToggleBlockToolbarMenu`
  and `CloseBlockToolbarMenu`. They are classified as `Blocks`, blocked on a
  locked page where they mutate the document, and they close the toolbar menu
  when the selection ends.
- Annotations are rendered through a `TextEdit::layouter` that builds a
  `LayoutJob` from the annotated spans, so the format buttons are visible and
  still edit plain text. If editing invalidates the span text, the layouter
  falls back to plain text instead of styling the wrong characters.
- `block_toolbar_menu` is app state (not egui memory) and is cleared with the
  selection and on page changes.

## Plan / tasks

- [x] Write the failing pure tests and the spec.
- [x] Add the pure block-selection helpers and commands.
- [x] Wire the two-stage `Ctrl+A`, selection paint and gutter controls.
- [x] Add the toolbar, the type menu and the annotated-text layouter.
- [x] Capture `ION_OF_NOTES_SHOW=selection` and `selection-text`, compare and
      adjust.
- [x] Run the gates, update `CHANGELOG.md`, the spec index and docs.
