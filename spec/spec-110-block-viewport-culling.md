# spec-110 - Block height cache and viewport culling

- Status: implemented (2026-11-12)
- Related: `spec-060`, `spec-103`, `spec-109`
- Review source: ChatGPT performance analysis (editor and database virtualization)

## Purpose and scope

The page body rendered every block on every frame. This spec adds a conservative
viewport culling layer:

- each rendered block's measured height is cached in egui temp data, keyed by
  page and block id, with a content fingerprint (text hash, rich-text length,
  kind discriminant and child count);
- a block with a matching cached height is skipped when it is outside the
  viewport plus 1.5 screens of overscan;
- skipped blocks still advance the layout by their cached height and heading
  gap, so positions and the scrollbar stay stable;
- culling is disabled while a page-level selection or block drag is active, and
  the focused/slash/selected block is always rendered;
- the database table skips offscreen rows with the same viewport helper and a
  fixed 28 px row height, and the board builds one status grouping instead of
  filtering all rows three times.

The first frame of a page still renders every block because heights are not
known yet; later frames skip unchanged offscreen blocks. A text/kind/child
change invalidates that block's height and it is measured again.

Out of scope:

- estimated heights for never-rendered blocks;
- a persistent height cache;
- changing block widgets or text editing semantics.

## Acceptance criteria

P1. **Given** a long page, **when** a later frame renders with the viewport
unchanged, **then** unchanged blocks outside the overscan are not rendered.

P2. **Given** a skipped block, **when** layout advances, **then** it uses the
cached height plus the heading and block gaps, so visible positions do not jump.

P3. **Given** a block text/kind/child change, **when** the frame renders, **then**
the fingerprint no longer matches and the block is rendered and re-measured.

P4. **Given** a focused, slash-menu or selected block, **when** it is offscreen,
**then** it is still rendered.

P5. **Given** a selection drag or block reorder, **when** the frame renders,
**then** culling is disabled.

P6. **Given** a table with many rows, **when** rows are offscreen, **then** they
advance by the fixed row height without building widgets.

P7. **Given** a board, **when** it renders, **then** each row is visited once for
grouping instead of once per column.

## Test scenarios

- `tests/ui.rs` (`block_viewport_culling_keeps_the_overscan_band`)
- `tests/fidelity.rs`
- `tests/screenshot.rs` (default page and Tasks table)
- Full quality gate

## Design

- `ui::block_outside_viewport(top, height, viewport, overscan)` is pure and
  tested.
- Heights live in `egui::Context` temp data, not in the serialized model.
- Culling is deliberately conservative: interactive states and uncached blocks
  render normally.

## Plan / tasks

- [x] Add the height cache and fingerprint
- [x] Skip unchanged offscreen blocks
- [x] Skip offscreen table rows and group board columns once
- [x] Add the viewport test and screenshots
- [x] Run the full gate
