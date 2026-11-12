# spec-113 - Estimate uncached block heights

- Status: implemented (2026-11-12)
- Related: `spec-110`
- Review source: ChatGPT performance analysis (first-frame block virtualization)

## Purpose and scope

`spec-110` skipped only blocks with a measured height, so the first frame of a
long page still rendered every block. This spec adds a first-frame estimate:

- `estimate_block_height(kind, text_chars)` gives a cheap height per block kind
  and text length;
- an uncached block **below** the viewport plus overscan is skipped using the
  estimate;
- an uncached block above the viewport is still rendered, so scrolling into a
  page does not shift the visible content;
- once a block is rendered, its measured height replaces the estimate;
- cached blocks keep the exact measured height and the existing fingerprint
  invalidation.

Out of scope:

- persisting estimates;
- a second layout pass;
- changing the editor widget model.

## Acceptance criteria

P1. **Given** the first frame of a long page at the top, **when** blocks are far
below the viewport, **then** they are skipped using estimated heights.

P2. **Given** an uncached block above the viewport, **when** the page is
scrolled, **then** it is rendered rather than skipped by estimate.

P3. **Given** a block that becomes visible, **when** it renders, **then** its
measured height replaces the estimate.

P4. **Given** a text/kind/child change, **when** the block renders, **then** the
fingerprint invalidates the measured height.

P5. **Given** focus, slash menu, selection or drag, **when** the frame renders,
**then** culling follows the `spec-110` rules.

## Test scenarios

- `tests/ui.rs` (`uncached_block_estimates_grow_with_kind_and_text`,
  `block_viewport_culling_keeps_the_overscan_band`)
- `tests/fidelity.rs`
- `tests/screenshot.rs`
- Full quality gate

## Design

- `estimate_block_height` is pure and testable.
- Estimates are only used below the viewport; above-viewport blocks render.
- Measured heights are still cached per page/block/fingerprint.

## Plan / tasks

- [x] Add the pure estimator
- [x] Use it for uncached blocks below the viewport
- [x] Add the estimator test
- [x] Run the full gate
