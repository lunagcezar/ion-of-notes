# spec-103 - Index remote cursors by block

- Status: implemented (2026-11-12)
- Related: `spec-043`, `spec-060`
- Review source: ChatGPT performance analysis (per-block cursor filtering)

## Purpose and scope

`render_block` filtered the whole remote-awareness vector for every block and
cloned matching cursors. With `B` blocks and `C` cursors that is `O(B * C)` per
frame plus allocations.

This spec builds one `HashMap<block_id, Vec<&Awareness>>` per page per frame and
does a lookup per block, making the work `O(B + C)` without cloning awareness
records. The awareness vector is shared through an `Arc`, so building the view
snapshot also copies one pointer instead of every cursor record.

Out of scope:

- caching cursor geometry or text widths;
- changing the awareness protocol;
- virtualizing blocks.

## Acceptance criteria

P1. **Given** several cursors on one block, **when** the page renders, **then**
all of them are drawn on that block.

P2. **Given** cursors on other pages, **when** the page renders, **then** they
are excluded before the index is built.

P3. **Given** a block with no cursor, **when** it renders, **then** the lookup
returns an empty slice.

P4. **Given** the existing CRDT/UI tests, **when** the index is used, **then**
they pass unchanged.

## Test scenarios

- `tests/ui.rs` (`cursor_index_groups_by_page_and_block`)
- `tests/ui_crdt.rs`
- `tests/fidelity.rs`
- Full quality gate

## Design

- `ui::render::index_cursors(cursors, page_id)` is a public pure helper.
- It returns references, so no awareness record is cloned.
- `draw_remote_cursors` accepts `&[&Awareness]`.

## Plan / tasks

- [x] Add the per-page cursor index
- [x] Pass it through `render_page`/`render_block`
- [x] Add the grouping test
