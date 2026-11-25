# spec-135 - Desktop CRDT operations for structure, properties and rows

- Status: implemented (2026-11-24)
- Related: `spec-043`, `spec-132`, `spec-134`
- Review source: follow-up to the server-side CRDT registers

## Purpose and scope

The server could merge Yrs `structure`, `properties` and `rows` maps, but the
desktop session only generated text updates and only projected text back.

This spec wires the desktop side:

- `CrdtSession::open_page_with_workspace` seeds the three object maps from the
  page and its data-source rows;
- `project_page` now rebuilds block structure/properties (then text) and
  `project_rows` writes row registers into the workspace;
- `baseline_page` tracks the block ids already represented in the structure
  map;
- `sync_new_blocks` inserts structure entries for blocks created locally after
  the session opened (for example, a split), so a following text update does
  not hide the new block during projection;
- `plan_page_changes(old, new, old_workspace, new_workspace)` emits one Yrs
  update for changed block parent/order/metadata, page properties and
  data-source row cells; it returns `None` when the diff includes title, page
  flags or data-source schema so the caller falls back to REST;
- remote `commit_edits` tries this CRDT path for a single current-page diff
  before dispatching a REST save.

Out of scope:

- title CRDT generation from the UI (title remains REST in this slice);
- page add/remove/reorder and workspace settings (REST);
- database schema CRDT (columns/data-source definitions remain REST);
- conflict UI beyond the existing Yrs convergence.

## Acceptance criteria

P1. **Given** a desktop structure/property change, **when** it is planned,
**then** applying the update to the server projects the new block order and
properties.

P2. **Given** a desktop row cell change, **then** the server projects the
updated data-source row.

P3. **Given** a block created locally before any text update, **then**
`sync_new_blocks` keeps it visible in the projected page.

P4. **Given** a diff outside the object maps, **then** `plan_page_changes`
returns `None` and the REST path runs.

P5. **Given** a remote CRDT update with structure/properties/rows, **then**
`project_crdt_page` writes them into both the workspace and the committed
baseline.

## Test scenarios

- `tests/ui_crdt.rs`
  (`desktop_structure_and_property_changes_plan_a_crdt_update`,
  `desktop_row_changes_plan_a_crdt_update`,
  `a_block_created_by_split_can_sync_its_text`,
  existing text/convergence tests).
- `tests/crdt.rs` (server projection).
- `cargo test --features desktop --test ui_crdt`.
- Full quality gate.

## Design

| Piece | Location |
| ----- | -------- |
| Session seeding/projection | `ui/crdt.rs` |
| New-block structure sync | `ui/crdt.rs::sync_new_blocks` |
| Change planner | `server/crdt.rs::plan_page_changes` |
| Remote commit path | `ui/apply/remote.rs::try_dispatch_crdt_commit` |
| Result projection | `ui/apply.rs::project_crdt_page` |

## Plan / tasks

- [x] Seed structure/properties/rows in the desktop session.
- [x] Project structure/properties/rows on remote updates.
- [x] Track and add structure entries for locally created blocks.
- [x] Plan structure/property/row diffs as one update.
- [x] Try the CRDT path before a remote REST page save.
- [x] Keep REST fallback for out-of-scope fields.
- [x] Add desktop plan and split-block tests.
