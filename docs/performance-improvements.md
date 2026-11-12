# Performance improvements

This document records the 2026-11-12 performance pass, what changed, and the
larger items that are deliberately staged. It complements
`docs/performance-roadmap.md` (measurements and scale decisions) and
`docs/refactoring-roadmap.md` (structural refactors).

## 1. Guiding rule

Remove unnecessary work on the UI thread before adding threads. The desktop
frame should do work proportional to what the user is interacting with, not to
the whole workspace.

## 2. Implemented

| Spec | Change | Why it matters |
| ---- | ------ | -------------- |
| `spec-101` | Frame mutation gate: no workspace clone or diff on idle and popup-only frames. | Removes the P0 per-frame deep clone and deep compare, including while scrolling the emoji picker. |
| `spec-102` | Normalize the emoji query once; scan lowercase catalog metadata directly. | Removes two `String` allocations per catalog entry per filter pass. |
| `spec-103` | Index remote cursors by block once per page per frame; share the awareness vector through an `Arc`. | Changes per-frame cursor work from `O(blocks × cursors)` to `O(blocks + cursors)` without cloning awareness records or the whole vector. |
| `spec-104` | Build the search body text once per page. | Removes duplicate `plain_text()`/row work inside a search pass. |
| `spec-105` | Trace the frame sections with `tracing` spans. | Separates render/command/persistence time from blocking I/O. |
| `spec-106` | Share the expanded-page set through an `Arc`. | Stops deep-cloning the sidebar expansion set every frame. |
| `spec-107` | Bound the remote awareness channel. | Prevents an unbounded cursor queue when the WebSocket worker falls behind. |
| `spec-108` | Background remote request worker for search, saves, replaces and CRDT. | Removes blocking `ureq` calls from the egui thread in remote mode. |

The existing good decisions are preserved: cached filtered icon indices,
`ScrollArea::show_rows` virtualization for the icon grid, the debounced edit
queue, the background WebSocket, and background webhook delivery.

## 3. Staged next

| Phase | Item | Status / trigger |
| ----- | ---- | ---------------- |
| 2 | Bounded remote API worker for REST, search and CRDT sync | Implemented in `spec-108`: one worker per connection, latest-search coalescing, in-flight commit coalescing and a synchronous CRDT fallback when the queue is full. |
| 3 | Incremental search index | Not implemented; current UI caches results by query + workspace version, and the model now avoids duplicate body work. |
| 3 | Block virtualization with measured heights | Not implemented; variable-height blocks need a layout cache and stable widget ids. High impact on very long pages. |
| 3 | Cursor geometry cache | Not implemented; cursor text widths are still measured during rendering. |
| 4 | Immutable server read snapshots + ordered writer | Not implemented; the Axum adapter still serializes through one `Mutex<WorkspaceServer>`. |
| 4 | Incremental SQLite persistence | Deferred by measurement: see `docs/performance-roadmap.md`. |

## 4. Measurement

`tests/perf.rs` measures model load/serialize/search/CRDT at 1k/10k/50k blocks.
The UI now emits `tracing` spans at the section boundaries
(`ui.view_state`, `ui.apply_commands`, `ui.commit_edits`, `ui.render_sidebar`,
`ui.render_content`), so a trace subscriber can separate CPU rendering from
blocking I/O. Frame timing is not part of the default gate.

## 5. Verification

- `cargo test --all-targets --no-default-features` (core)
- `cargo test --features desktop --test ui --test fidelity --test ui_crdt --test screenshot`
- `pwsh -File scripts/quality.ps1`
- `--screenshot` comparison against `reference/` for any visual change
