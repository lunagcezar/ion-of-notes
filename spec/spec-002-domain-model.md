---
title: Workspace domain model, page tree, and search
version: 1.0
date_created: 2026-11-05
tags: model, tree, search
status: implemented
---

# Spec 002 — Domain Model

## 1. Purpose & Scope

Implement the pure data layer described in `docs/data-model.md`: workspace,
pages, blocks, table rows, tree helpers, search ranking, and JSON round-trip.
No I/O and no UI in this module.

## 2. Acceptance Criteria

- **AC-001** — Given an empty workspace, when `seed()` runs, then a neutral
  demo workspace with named root pages (Home, Getting Started, Projects,
  Tasks, Notes) exists.
- **AC-002** — Given a workspace, when a page with an existing id is added,
  then `add_page` returns `DuplicateId` and the workspace is unchanged.
- **AC-003** — Given a page with children, when `remove_page(parent)` runs,
  then the parent and every descendant are removed and the removed ids are
  returned.
- **AC-004** — Given a page with ancestors, when `breadcrumbs()` runs, then
  titles are returned root-first and the page's own title is last.
- **AC-005** — Given pages titled "Roadmap" and "Notes" with the word
  "roadmap" in a block, when `search("road")` runs, then the title match ranks
  above the body match and results are case-insensitive.
- **AC-006** — Given a page with blocks, when `plain_text()` runs, then all
  block texts are joined with spaces and divider blocks contribute nothing.
- **AC-007** — Given any workspace, when it is serialized to JSON and back,
  then the value is equal to the original.
- **AC-008** — Given a `Todo` block, when `toggle_todo()` runs, then `checked`
  flips and other blocks are untouched.
- **AC-009** — Given a block, when `set_kind()` changes its type, then its
  `text` and `id` are preserved.
- **AC-010** — Given any successful write, when `bump_version()` runs, then
  `version` increases by exactly one.

## 3. Test Scenarios

- TS-001 `seed_has_expected_pages`
- TS-002 `duplicate_page_id_is_rejected`
- TS-003 `removing_page_removes_descendants`
- TS-004 `breadcrumbs_are_root_first`
- TS-005 `search_ranks_title_above_body_and_ignores_case`
- TS-006 `plain_text_joins_block_text`
- TS-007 `workspace_json_roundtrip`
- TS-008 `toggle_todo_flips_only_that_block`
- TS-009 `set_kind_preserves_text_and_id`
- TS-010 `bump_version_increments_once`

## 4. Plan & Tasks

1. Write the ten tests above in `src/model.rs` (`#[cfg(test)]`) — **red**.
2. Add `BlockKind`, `Block`, `TableRow`, `Page`, `Workspace`, helpers.
3. Run `cargo test` — **green**; refactor only after green.
4. Update `CHANGELOG.md`.
