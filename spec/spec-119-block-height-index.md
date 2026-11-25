# spec-119 - Block height index for the page renderer

- Status: implemented (2026-11-24)
- Related: `spec-101`, `spec-110`, `spec-113`, `spec-116`
- Review source: ChatGPT performance review (2026-11-24)

## Purpose and scope

The renderer culled offscreen blocks, but it still visited every block in
document order to advance the cursor and decide what to skip. With thousands
of blocks, every frame paid O(n) even when only a screenful was visible.

This spec adds a per-page order-statistic height index:

- `ui::render::content::heights::HeightIndex` stores one slot height per block
  in a Fenwick tree plus a second tree for ordered-list counts.
- It is cached in egui temp data under the page id and rebuilt only when the
  block count or a cheap structural signature changes.
- The renderer finds the first visible block by lower-bound on the cumulative
  heights, renders only the visible window plus overscan, and represents the
  rest with one leading and one trailing space.
- Rendered blocks update their measured content height in the tree, so the
  scroll geometry converges without a full remeasure.
- Selection, drag, focus and slash-menu frames use the full-block path so an
  interaction target is never skipped.
- Ordered-list numbering is derived from the count tree, so starting mid-list
  keeps the correct number.

Out of scope:

- variable-width wrapping across window resizes (the estimate is corrected as
  blocks become visible);
- virtualizing table rows inside a database block beyond the existing culling;
- changing block layout or measurement.

## Acceptance criteria

P1. **Given** a page with many cached blocks, **when** a normal frame renders,
**then** the loop starts at the first intersecting slot and stops after the
viewport plus overscan.

P2. **Given** a structural change, **when** the next frame runs, **then** the
index rebuilds from the new block list.

P3. **Given** a text edit in a visible block, **when** it renders, **then** its
tree entry updates and the trailing space reflects the new height.

P4. **Given** selection, drag, focus or a slash menu, **when** the page renders,
**then** every block is still laid out.

P5. **Given** a page scrolled into the middle of an ordered list, **then** the
first rendered item keeps the correct sequence number.

## Test scenarios

- Existing `tests/ui.rs` (`block_viewport_culling_keeps_the_overscan_band`,
  `uncached_block_estimates_grow_with_kind_and_text`).
- `cargo test --features desktop --test ui --test fidelity --test screenshot`.
- `cargo run -- --screenshot shots/now.png` visual comparison.
- Full quality gate.

## Design

| Piece | Location |
| ----- | -------- |
| Height index and Fenwick trees | `ui/render/content/heights.rs` |
| Visible-window loop | `ui/render/content/page.rs` |
| Full-block interaction path | `ui/render/content/page.rs` |

The cache key is the page id. The structural signature hashes the block count,
every sixteenth block id and the last id. A middle reorder that keeps all of
those unchanged may use stale offsets for one frame; rendered blocks correct
their own entry immediately.

## Plan / tasks

- [x] Add the Fenwick height and numbered-count trees.
- [x] Cache the index in egui temp data and rebuild on structural change.
- [x] Render only the visible window in the normal path.
- [x] Keep the full-block path for interaction.
- [x] Correct ordered-list numbering from the count tree.
- [x] Remove the old per-block hash map.
