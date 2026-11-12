# spec-112 - Cache remote cursor geometry

- Status: implemented (2026-11-12)
- Related: `spec-043`, `spec-103`, `spec-110`
- Review source: ChatGPT performance analysis (cursor geometry)

## Purpose and scope

Remote cursor rendering rebuilt a prefix `String` and re-measured the same text
width for every cursor and selection endpoint on every frame. This spec adds a
small runtime cache:

- key = text hash, character index, font size and font-family discriminant;
- value = measured width in points;
- stored in egui temp data behind an `Arc<Mutex<HashMap<...>>>`;
- capped at 1024 entries and cleared when the cap is reached;
- layout is only performed on a cache miss.

The cache is independent of color because text width does not depend on it.

Out of scope:

- sharing the editor's text layout;
- persisting geometry;
- changing cursor drawing or colors.

## Acceptance criteria

P1. **Given** an unchanged block text and cursor position, **when** a later
frame draws the cursor, **then** the width is read from the cache.

P2. **Given** a text, character-index, font-size or family change, **when** the
cursor is drawn, **then** the key changes and the width is measured again.

P3. **Given** more than 1024 distinct keys, **when** a new key is inserted,
**then** the cache clears instead of growing without bound.

P4. **Given** the existing cursor/CRDT tests and screenshots, **when** the cache
is used, **then** overlays render at the same positions.

## Test scenarios

- `tests/ui.rs` (cursor index grouping)
- `tests/ui_crdt.rs`
- `tests/screenshot.rs`
- Full quality gate

## Design

- `CursorWidthKey` is a copyable hash key.
- `text_width` checks the cache before building the prefix.
- The cache lives in `egui::Context` temp data, not in the model or app state.

## Plan / tasks

- [x] Add the key and cache
- [x] Measure only on a miss
- [x] Cap the cache
- [x] Run the full gate
