# spec-106 - Share the expanded page set through an Arc

- Status: implemented (2026-11-12)
- Related: `spec-101`, `spec-022`
- Review source: ChatGPT performance analysis (ViewState allocations)

## Purpose and scope

`ViewState` cloned the whole expanded-page `HashSet` every frame, even though
the renderer only reads it. This spec stores it as `Arc<HashSet<String>>` in
both `NotionApp` and `ViewState`, so building the view snapshot copies one
pointer. The single mutation site (expand/collapse) uses `Arc::make_mut`, which
clones only when another frame still holds the previous set.

Out of scope:

- tabs, sidebar layout or other view-state collections;
- changing expand/collapse behavior.

## Acceptance criteria

P1. **Given** the sidebar tree, **when** it renders, **then** the expanded
pages are read from the shared set.

P2. **Given** an expand or collapse, **when** it applies, **then** the set is
updated and the next frame sees the new state.

P3. **Given** an idle frame, **when** the view snapshot is built, **then** the
expanded set is not deep-cloned.

P4. **Given** the existing UI tests, **when** the storage changes, **then** they
pass unchanged.

## Test scenarios

- `tests/ui.rs`
- `tests/fidelity.rs`
- `tests/screenshot.rs`
- Full quality gate

## Design

- `Arc::make_mut` keeps copy-on-write semantics for the rare mutation.
- Rendering code is unchanged because `Arc<HashSet<_>>` derefs to the set.

## Plan / tasks

- [x] Store the set in an `Arc`
- [x] Use `Arc::make_mut` on toggle
- [x] Run the full gate
