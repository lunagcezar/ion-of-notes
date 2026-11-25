# spec-156 - Emoji picker warm-up and background rendering

- Status: implemented (2026-11-25)
- Related: `spec-057`, `spec-141`, `spec-147`, `spec-048`
- Review source: user report (first open of the emoji picker was slow)

## Purpose and scope

The emoji grid was already virtualized, but the first pass over each new row
still rasterized nine color glyphs into the font atlas. In a debug build that
cost several milliseconds per glyph, so the first scroll stuttered until the
whole catalog had been visited. The earlier workaround also replaced the whole
background with a dim backdrop while the picker was open, which did not improve
the picker and removed useful context.

### Font-stack optimization

`Cargo.toml` now sets `opt-level = 3` for the CPU rasterization stack
(`epaint`, `vello_cpu`, `vello_common`, `skrifa`, `read-fonts`, `font-types`) in
both the `dev` and `test` profiles. Debug info for this crate is unchanged; only
the dependencies that turn glyph outlines into textures are optimized. In a
measured `--screenshot` run the first-open emoji state dropped from ~21.7 s to
~3.1 s for 120 frames in debug, matching the normal page render.

### Progressive warm-up

The picker warms the catalog in batches while it is open (48 glyphs per frame)
and at idle (64 per frame) when the picker is closed. The cursor lives in
`egui` temp data, so closing and reopening resumes rather than restarting.
`request_repaint` keeps the batch loop running until all glyphs are laid out;
`warmup_batch` is the pure range helper with a regression test.

Warm-up is skipped while the user is typing, dragging or scrolling (input events
present), so it never competes with an active frame. The visible rows are still
drawn by `show_rows`; warm-up only stays ahead of the next rows.

### Background rendering

The `icon_picker_open` backdrop branch in `ui/app.rs` is removed. The sidebar,
title bar and page render normally, and the picker floats above them in its own
window/layer. This restores context and does not cost picker responsiveness.

## Acceptance criteria

P1. **Given** the picker open, **then** the sidebar and page are rendered behind
it and the picker remains interactive.

P2. **Given** the first open in a debug build, **then** the picker no longer
takes an order of magnitude longer than a normal frame batch.

P3. **Given** the catalog, **then** warm-up covers every index exactly once per
run and stops at the end.

P4. **Given** active typing/scrolling, **then** background warm-up does not run.

## Test scenarios

- `tests/ui.rs` (`emoji_warmup_batches_cover_the_catalog_once`).
- `ION_OF_NOTES_SHOW=iconpicker cargo run -- --screenshot ...` compared with the
  default screenshot; both complete without hitches.
- Existing screenshot/fidelity suites.

## Plan / tasks

- [x] Optimize the font rasterization crates in dev/test profiles.
- [x] Add batched, resumable warm-up with a pure range helper.
- [x] Run warm-up at idle and while the picker is open.
- [x] Restore normal background rendering behind the picker.
- [x] Add the batch test and update documentation.
