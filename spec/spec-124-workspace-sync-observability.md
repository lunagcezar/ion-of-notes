# spec-124 - Workspace and sync observability

- Status: implemented (2026-11-24)
- Related: `spec-037`, `spec-054`, `spec-116`
- Review source: deferred observability item in `docs/review-response.md`

## Purpose and scope

`/metrics` exposed process uptime, request volume and three webhook counters.
Operators could not see workspace size, the sync cursor, the in-memory request
log or how many webhook jobs a worker tick attempted.

This spec extends the Prometheus text exposition with:

- `ion_of_notes_pages` — current page count;
- `ion_of_notes_blocks` — current block count (pages count as one);
- `ion_of_notes_members` — workspace member count;
- `ion_of_notes_workspace_version` — current workspace version;
- `ion_of_notes_sync_seq` — latest sync-log sequence;
- `ion_of_notes_request_log_entries` — in-memory request-log size;
- a `tracing::debug!` line per webhook worker cycle with the number of jobs.

Existing metrics keep their names and meanings.

Out of scope:

- OpenTelemetry export;
- per-route latency histograms;
- a Prometheus client library.

## Acceptance criteria

P1. **Given** a running server, **when** `/metrics` is read, **then** the new
families are present with numeric values.

P2. **Given** a workspace mutation, **when** metrics are read, **then** the
version/sync sequence reflect the committed state.

P3. **Given** a webhook worker tick, **then** a debug trace records the number
of prepared jobs.

## Test scenarios

- `tests/web.rs` (`metrics_route_exposes_prometheus_text`).
- Full quality gate.

## Design

| Piece | Location |
| ----- | -------- |
| Exposition | `server/web/operations.rs` |
| Worker trace | `server/web/mod.rs` |

## Plan / tasks

- [x] Read workspace/collab counts under the server lock.
- [x] Add the new gauge families.
- [x] Trace the webhook worker job count.
- [x] Extend the metrics regression test.
