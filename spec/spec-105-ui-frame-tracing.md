# spec-105 - Trace the UI frame sections

- Status: implemented (2026-11-12)
- Related: `spec-101`, `spec-097`
- Review source: ChatGPT performance analysis (measure before optimizing)

## Purpose and scope

The performance pass needs a way to separate render time, command application
and persistence from blocking I/O. This spec adds `tracing::trace_span!` spans
at the existing section boundaries:

- `ui.view_state`
- `ui.apply_commands`
- `ui.commit_edits`
- `ui.render_sidebar`
- `ui.render_content`

The spans are cheap when no subscriber enables them and require no new
dependency (`tracing` is already present).

Out of scope:

- a built-in frame profiler or overlay;
- always-on timing storage;
- changing the tracing subscriber defaults.

## Acceptance criteria

P1. **Given** a tracing subscriber at `TRACE`, **when** a frame runs, **then**
the five spans appear with their elapsed times.

P2. **Given** the default build and gate, **when** no subscriber is installed,
**then** behavior and tests are unchanged.

P3. **Given** the existing render/command code, **when** spans are added, **then**
no control flow or output changes.

## Test scenarios

- `tests/ui.rs`
- `tests/fidelity.rs`
- `tests/screenshot.rs`
- Full quality gate

## Design

- Spans are function-local and entered for the duration of the existing work.
- No global state and no per-frame allocation beyond tracing's disabled path.
- `docs/performance-improvements.md` records the section names.

## Plan / tasks

- [x] Add the five spans
- [x] Document the section names
- [x] Run the full gate
