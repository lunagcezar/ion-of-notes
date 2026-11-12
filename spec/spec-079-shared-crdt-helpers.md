# spec-079 - Shared CRDT text helpers

- Status: implemented (2026-11-12)
- Related: `spec-042`, `spec-043`, `spec-080`
- Review sources: Claude and ChatGPT (duplicated CRDT helpers and block-tree
  walks)

## Purpose and scope

The server and desktop CRDT modules had near-identical `collect_texts`,
`collect_block_texts` and `project_blocks` functions, and the model had
several separate recursions over the block tree. A change to the text mapping
had to be made twice.

This spec moves the shared walk and projection into `src/crdt_text.rs` and
adds reusable block-tree walkers to the model.

Scope:

- `src/crdt_text.rs` with `block_text_name`, `collect_block_texts`,
  `collect_text_snapshots` and `project_blocks`;
- `walk_blocks` / `walk_blocks_mut` in `src/model/block.rs`;
- `count_blocks` uses the walker;
- server and UI CRDT sessions use the shared helpers;
- existing CRDT tests remain green.

Out of scope:

- changing the Yrs document shape;
- a generic `BlockTree` trait;
- CRDT coverage for structure/properties/databases.

## Acceptance criteria

P1. **Given** the server CRDT store, **when** it seeds or projects a page,
**then** it uses the shared `crdt_text` helpers.

P2. **Given** the desktop CRDT session, **when** it seeds or projects a page,
**then** it uses the same helpers.

P3. **Given** a nested block tree, **when** `count_blocks` runs, **then** the
result is unchanged and the walker visits every block once.

P4. **Given** the existing CRDT convergence tests, **when** the helpers move,
**then** they still pass.

## Test scenarios

- `tests/crdt.rs` (existing convergence and WebSocket CRDT tests)
- `tests/ui_crdt.rs` (existing client diff/projection tests)
- `tests/model.rs` (block counts and tree behavior)

## Design

- `crdt_text` is a crate-private module (no desktop feature).
- `walk_blocks` and `walk_blocks_mut` live beside the block type.
- The server keeps its `BTreeMap` projection; the UI keeps its `last_text`
  baseline. Only the traversal and projection are shared.

## Plan / tasks

- [x] Write spec-079
- [x] Add the walkers and shared helpers
- [x] Replace the server/UI copies
- [x] Run the CRDT tests and full gate
- [x] Commit
