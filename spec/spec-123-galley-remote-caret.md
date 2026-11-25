# spec-123 - Galley-based remote caret overlay

- Status: implemented (2026-11-24)
- Related: `spec-043`, `spec-103`, `spec-112`
- Review source: deferred item in `docs/review-response.md`

## Purpose and scope

The remote caret overlay estimated a character position from a first-line text
width. A caret or selection on a wrapped paragraph could therefore land on the
wrong row, and a multi-line selection became one horizontal rectangle.

This spec uses the laid-out `Galley` from the block's `TextEditOutput`:

- `selection_rects(galley, start, end)` maps character indices through
  `layout_from_cursor` and returns one rectangle per visual row;
- `draw_remote_cursors` receives `galley` and `galley_pos`, so the caret line
  and its name label use the real row position;
- the old text-width cache and its 1024-entry map are removed;
- the geometry function is public through `ui::selection_rects` so it can be
  tested without a window.

Out of scope:

- bidi-specific caret affinity beyond egui's own cursor layout;
- touch/mobile selection handles;
- changing awareness payloads.

## Acceptance criteria

P1. **Given** a wrapped paragraph, **when** a selection spans rows, **then**
one highlight rectangle is produced per visual row.

P2. **Given** a caret index on a later row, **when** it is painted, **then** its
vertical position matches the galley row, not the first line.

P3. **Given** an empty selection, **then** no highlight rectangle is produced.

P4. **Given** a long text block, **then** no per-character width cache is
consulted for cursor geometry.

## Test scenarios

- `tests/ui.rs` (`remote_selection_rects_follow_wrapped_rows`).
- Existing `tests/ui.rs` (`cursor_index_groups_by_page_and_block`).
- `cargo test --features desktop --test ui --test fidelity`.
- `--screenshot` smoke render.
- Full quality gate.

## Design

| Piece | Location |
| ----- | -------- |
| Galley geometry and painter | `ui/render/content/cursors.rs` |
| Block call site | `ui/render/content/blocks.rs` |
| Public facade | `ui/render/content.rs`, `ui/render.rs`, `ui.rs` |

## Plan / tasks

- [x] Map indices through `Galley::layout_from_cursor`.
- [x] Paint one selection rectangle per visual row.
- [x] Position the caret and label from `pos_from_cursor`.
- [x] Remove the width cache.
- [x] Expose and test `selection_rects`.
