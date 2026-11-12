# spec-065 - Page icon fallback to the title letter

- Status: implemented (2026-11-12)
- Related: `spec-013`, `spec-057`, `spec-060`

## Purpose and scope

A page without a custom icon still shows the open-book artwork. The requested
default is the first letter of the page title, uppercased, everywhere the page
is referenced: the page header, sidebar, tabs, breadcrumbs and search results.

## Acceptance criteria

B11. **Given** a page with an empty icon, **when** it renders in the header,
sidebar, tab, breadcrumb or search result, **then** the first letter of the
title is shown uppercased.

B12. **Given** an explicit emoji, symbol or `shape:` icon, **when** it renders,
**then** the explicit icon wins.

B13. **Given** an empty title and empty icon, **when** it renders, **then** a
neutral `?` fallback is used.

B14. **Given** a non-ASCII title, **when** it has no icon, **then** the first
Unicode scalar is uppercased when possible and rendered as text.

## Test scenarios

- Pure `page_icon_letter(icon, title)` cases: empty icon + "home" → "H";
  explicit "★" unchanged; empty title → "?"; non-ASCII first letter.
- Screenshots: `icon-fallback-header`, `icon-fallback-sidebar`,
  `icon-fallback-tab`.
- Full gate.

## Design

- Add `page_icon_letter(icon, title)` in the icon layer and use it at every
  `draw_page_icon` call site. Explicit `"book"` stays the book artwork.
- Keep the icon picker Remove action; removing an icon immediately falls back
  to the title letter.
- Do not mutate the page icon when rendering; the fallback is derived.
