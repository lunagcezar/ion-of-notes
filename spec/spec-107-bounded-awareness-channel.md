# spec-107 - Bound the remote awareness channel

- Status: implemented (2026-11-12)
- Related: `spec-044`, `spec-043`
- Review source: ChatGPT performance analysis (awareness backpressure)

## Purpose and scope

The remote adapter queued awareness commands on an unbounded Tokio channel.
Fast cursor movement could grow the queue without limit if the WebSocket worker
fell behind. This spec uses a bounded channel (capacity 64) and `try_send`:

- a full queue drops the newest awareness update, which is the correct
  latest-value behavior for cursor presence;
- a closed channel still reports an error to the caller;
- CRDT and workspace traffic keep their existing reliable paths.

Out of scope:

- a latest-value slot instead of a queue;
- coalescing CRDT updates;
- changing the WebSocket protocol.

## Acceptance criteria

P1. **Given** a healthy WebSocket worker, **when** awareness is published,
**then** it is forwarded as before.

P2. **Given** a saturated awareness queue, **when** a new cursor update arrives,
**then** the send succeeds by dropping the newest update instead of growing
memory.

P3. **Given** a closed worker, **when** awareness is published, **then** the
caller receives an error.

P4. **Given** the remote tests, **when** the channel is bounded, **then** REST,
CRDT and awareness behavior still passes.

## Test scenarios

- `tests/remote.rs`
- `tests/realtime.rs`
- Full quality gate

## Design

- `mpsc::channel(64)` replaces `unbounded_channel()`.
- `publish_awareness` maps `Full` to `Ok(())` and `Closed` to `Err`.
- No protocol or DTO change.

## Plan / tasks

- [x] Bound the channel
- [x] Use `try_send` with latest-value drop semantics
- [x] Run the remote tests and full gate
