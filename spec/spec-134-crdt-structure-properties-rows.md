# spec-134 - CRDT registers for structure, properties and database rows

- Status: implemented (2026-11-24)
- Related: `spec-042`, `spec-043`, `spec-133`
- Review source: deferred CRDT coverage for structure/properties/databases

## Purpose and scope

The Yrs page document merged only the title and block plain text; block order,
page properties and database row cells stayed REST-owned. Two clients editing
different cells or moving blocks could overwrite each other through REST.

This spec adds three Yrs maps to each page document:

- `structure`: block id → JSON `BlockStructure { parent_id, position, block }`
  (block without children);
- `properties`: property key → JSON `PageProperty`;
- `rows`: `data_source_id/row_id` → JSON `RowRegister { data_source_id, page,
  deleted }`.

Behavior:

- `ensure_page` seeds all three maps from the page, its properties and rows of
  data sources attached to that page;
- `project` rebuilds the block tree from `structure` (ordered by `position`,
  parent-aware, cycle-protected), replaces page properties, and applies row
  registers (explicit `deleted` removes a row);
- text is projected after the structural rebuild, so structure and text merge
  in one pass;
- Yrs map semantics provide per-key last-writer-wins for structure entries,
  properties and rows; removing a map key removes the object on projection;
- REST writes still reset the page document, so the next sync reseeds from the
  committed workspace (REST remains the structural source of truth after a
  REST mutation).

Out of scope:

- desktop UI generation of structure/property/row updates (the server merge and
  projection are implemented and tested; UI still edits those through REST);
- database schema CRDT (columns/data-source definitions remain REST-owned);
- a global workspace CRDT document;
- presence/undo integration.

## Acceptance criteria

P1. **Given** a client inserts or moves `structure` entries, **when** the
server applies the update, **then** the projected page tree reflects the new
parent, order and deletion.

P2. **Given** a client sets a `properties` entry, **then** the page property map
is updated; removing the entry removes the property.

P3. **Given** a client updates a `rows` register, **then** the matching
data-source row is replaced; a `deleted` register removes it.

P4. **Given** a text-only CRDT update, **then** structure/properties/rows keep
their seeded values and text projection still works.

P5. **Given** a REST mutation, **then** the next CRDT sync reseeds from the
committed workspace.

## Test scenarios

- `tests/crdt.rs`
  (`structure_registers_project_inserts_moves_and_deletes`,
  `property_registers_project_and_remove_page_properties`,
  `row_registers_project_database_cells_and_explicit_deletion`,
  existing text convergence/WebSocket tests).
- `tests/blocks.rs`, `tests/properties.rs` (REST paths unchanged).
- Full quality gate.

## Design

| Piece | Location |
| ----- | -------- |
| Register types | `server/crdt.rs` (`BlockStructure`, `RowRegister`) |
| Seed/projection | `server/crdt.rs` |
| Text overlay | `crdt_text.rs` |
| Reset after REST writes | `WorkspaceServer::commit_write` |

## Plan / tasks

- [x] Add `structure`, `properties` and `rows` maps.
- [x] Seed them from the page/workspace.
- [x] Project structure into the block tree.
- [x] Project properties and row registers.
- [x] Keep text projection after the structure rebuild.
- [x] Add insert/move/delete, property and row tests.
