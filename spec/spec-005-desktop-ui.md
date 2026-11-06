---
title: Desktop window — sidebar, page, blocks, table, search, theme
version: 1.0
date_created: 2026-11-05
tags: ui, eframe, egui
status: implemented
---

# Spec 005 — Desktop UI

## 1. Purpose & Scope

Rebuild the core look and behavior of a notes workspace from the written
request and from a neutral estimate of the visual design (see
`docs/unknowns.md` U-01). No assets are copied. All visual values live in
`ui::Palette`.

## 2. Acceptance Criteria

- **AC-001** — Given the app starts, when the window appears, then a left
  sidebar (workspace header, Search, Home, Inbox, Favorites, Private tree,
  New page, Settings) and a content area are visible.
- **AC-002** — Given a page is selected, when it renders, then a breadcrumb
  trail, a large editable title, an icon, and its blocks are shown.
- **AC-003** — Given a heading / to-do / quote / callout / code / divider
  block, when it renders, then each has its distinct visual treatment.
- **AC-004** — Given a to-do, when the checkbox is clicked, then the checked
  state is sent through the client and survives a reload.
- **AC-005** — Given a block, when the plus button is clicked, then a new
  paragraph is inserted directly after it.
- **AC-006** — Given a block whose text begins with `/`, when the slash menu
  is used, then choosing a type changes that block's kind.
- **AC-007** — Given a page with table rows, when it renders, then a table
  with colored status chips is shown and a Board view is available.
- **AC-008** — Given the user presses Ctrl+K or clicks Search, when the dialog
  opens, then typing filters pages and clicking a result opens it.
- **AC-009** — Given the Settings dialog, when the theme is switched, then
  light/dark visuals apply to the whole window.
- **AC-010** — Given the sidebar, when a tree disclosure is clicked, then
  child pages expand and collapse.
- **AC-011** — Given any edit, when the frame ends, then the change is sent
  through `Client` to the server (never by direct state mutation).
- **AC-012** — Given the window is resized to 900x600, when it renders, then
  the sidebar and content remain usable (no overlapping panels).

## 3. Test Scenarios

UI behavior is verified by running the built app with `--screenshot` and
inspecting the PNG (see `spec-006`). The underlying state transitions are
covered by the `model`, `server`, and `client` unit tests. Additional pure
tests:

- TS-001 `palette_colors_are_distinct_per_theme`
- TS-002 `block_menu_options_match_block_kinds`
- TS-003 `status_chip_color_is_stable_for_known_statuses`

## 4. Plan & Tasks

1. Write the three palette/menu tests — **red**.
2. Implement `Palette`, `NotionApp`, panels, block renderer, table/board,
   dialogs, shortcuts.
3. Wire every command to `Client`.
4. Build and visually inspect the screenshot; fix layout issues.
5. Update `CHANGELOG.md`.
