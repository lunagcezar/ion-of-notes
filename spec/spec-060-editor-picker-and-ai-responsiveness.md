# spec-060 - Editor, picker and AI responsiveness

- Status: implemented (2026-11-11)
- Related: `spec-013`, `spec-017`, `spec-040`, `spec-043`, `spec-057`
- Reference: `reference/block-type.png`, `reference/customize-sidebar.png`
- Runtime: local desktop window, optional Ollama service or CLI

## Purpose and scope

Five reliability issues remain after the UI quality pass:

1. A block drag can start but cannot be dropped when the mouse button is
   released away from the six-dot handle.
2. The page-icon emoji grid renders the whole catalog every frame, so scrolling
   and filtering feel slow.
3. Emoji and icon glyphs are still not visually centered in their icon well.
4. The Home page shows a book in the page header but a document outline in the
   sidebar, tab and breadcrumb.
5. Testing Ollama in Settings freezes the window while the transport is probed.

This spec fixes the interaction state machine, adds lazy rendering and caching
to the picker, unifies the icon artwork, and moves all Ollama detection and
generation off the UI thread.

Out of scope:

- A new block model or rich-text engine.
- Streaming AI tokens; the answer still arrives as one result.
- Downloading an emoji font or a new icon pack.

## Evidence

The review reported:

1. "I can drag, but I can't drop when I stop holding the mouse button."
2. "Emoji list is running slow."
3. "Emoji/icon from the page still isn't vertically aligned in the
   parent/container."
4. "The main page, home, has a book emoji, but it isn't a book emoji in the
   sidebar icon."
5. "Testing Ollama in the settings freezes the application for some seconds."

Current code seams:

| Area | Code |
| ---- | ---- |
| Block drag and drop | `src/ui/render/content/blocks.rs`, `src/ui/apply/blocks.rs` |
| Drag commands/state | `src/ui/commands.rs`, `src/ui/app.rs`, `src/ui/view.rs` |
| Icon picker | `src/ui/page_icon.rs`, `src/ui/icon_catalog.rs` |
| Page icon painting | `src/ui/icons.rs` (`draw_page_icon`, `draw_book`) |
| Page header, sidebar, tabs | `src/ui/render/content/page.rs`, `src/ui/render/sidebar.rs`, `src/ui/render/chrome.rs` |
| AI commands and polling | `src/ui/apply/ai.rs`, `src/ui/apply.rs`, `src/ollama.rs` |

## Acceptance criteria

### Block drag and drop

D1. **Given** a block's six-dot handle, **when** the pointer is pressed and
moved, **then** the drag starts and the live ghost follows the pointer.

D2. **Given** an active block drag, **when** the mouse button is released over
any block or over empty editor space, **then** the block is dropped at the last
valid target; releasing over empty space cancels without moving the block.

D3. **Given** an active block drag, **when** the pointer leaves the editor and
the button is released, **then** the drag state and ghost clear and no command
is left pending.

D4. **Given** a drop, **when** it completes, **then** the insertion line
disappears, the page order is persisted through the normal edit queue and the
block remains selected/editable.

### Icon picker performance

D5. **Given** the Emoji tab, **when** it opens, **then** only the visible rows
are laid out and painted; scrolling reuses the row layout instead of rebuilding
the full catalog every frame.

D6. **Given** a filter query, **when** the query changes, **then** the filtered
index list is computed once per (tab, query) and reused until either changes.

D7. **Given** a large catalog, **when** the pointer scrolls quickly, **then**
the window stays responsive and the selected/hovered emoji still work.

### Icon alignment and consistency

D8. **Given** a page icon in the large header well, **when** it renders, **then**
the glyph's visual bounds are centered vertically and horizontally.

D9. **Given** the same page icon in the sidebar, tab and breadcrumb, **when** it
renders at small size, **then** it uses the same artwork and optical center.

D10. **Given** the Home page with its default book icon, **when** it renders in
the header, sidebar, tab and breadcrumb, **then** every surface shows a book
shape, not a document outline.

### AI responsiveness

D11. **Given** the AI settings page, **when** it opens or Refresh is clicked,
**then** detection runs in the background and the page shows "Checking…"
until a status arrives; the UI never blocks.

D12. **Given** the test question, **when** Ollama is slow or unavailable,
**then** the window stays interactive, a 20-second watchdog clears the busy
state, and the actual error or answer is shown.

D13. **Given** the AI chat and the settings test, **when** both are used,
**then** they share one async runner and one status slot; a second request is
ignored while one is in flight.

## Test scenarios (written before implementation)

- **Drag state machine:** start -> hover target -> release drops; start ->
  release outside cancels; start -> release with no target clears state; a
  second start while dragging is ignored.
- **Picker cache:** same tab/query reuses indices; a query change recomputes;
  row count/column count stay consistent at 9 columns.
- **Centering:** a pure helper returns a rect centered in the well for several
  font sizes and glyph heights; a regression check for the small book shape.
- **AI runner:** a fake runner returns success, error and timeout; the busy
  flag clears and the status slot is consumed exactly once.
- **Screenshots:** `iconpicker-scrolled`, `iconpicker-filtered`,
  `page-icon-header`, `page-icon-sidebar`, `ai-checking`, `ai-error`.
- **Regression:** existing block, CRDT, picker and Ollama tests; full gate.

## Design

- Replace the `drag_stopped` reliance with a drag state in app state:
  `BlockDrag { from, to, active }`. On every frame where `active` is true, the
  app checks `ctx.input(|i| i.pointer.any_released())`; the release always
  emits `DropBlock` or `CancelBlockDrag`, independent of where the pointer is.
- Keep the six-dot handle as the only drag source; the `+` remains the insert
  menu. The ghost and insertion line are painted from the same state.
- Add a small `IconPickerCache` keyed by `(IconTab, String)` that stores the
  filtered index vector. Use `ScrollArea::show_rows` with a fixed row height
  and 9 columns so only visible rows are built.
- Replace the 3% optical lift with a helper that lays out the glyph and centers
  the resulting galley rect inside the well. Apply the same helper to emoji and
  shape icons.
- Give `draw_page_icon` a small book branch: the default/book icon draws the
  book artwork at both sizes; the small variant is a scaled version of the same
  shape.
- Introduce an `AiRunner` abstraction with `detect` and `generate` on worker
  threads, result slots in app state, and one poll function. Startup and
  Refresh show "Checking…" and never call the transport on the UI thread.

## Plan / tasks

- [x] Drag state machine and release-based drop/cancel with tests.
- [x] Picker cache and virtualized grid.
- [x] Centering helper and unified book artwork.
- [x] Async AI runner, startup/refresh states and watchdog.
- [x] Preview states and screenshots.
- [x] Full gate and changelog.

## Decisions and open questions

- Decision: a release over empty editor space cancels the move; the block
  returns to its original position.
- Decision: the picker is virtualized, not paginated; the catalog stays
  complete and searchable.
- Decision: the Home page keeps its empty/book default; the fix is in the
  renderer, not in the seed data.
- Open: whether the AI runner should also stream partial text. The default is
  no; the answer arrives once and the UI stays responsive.
