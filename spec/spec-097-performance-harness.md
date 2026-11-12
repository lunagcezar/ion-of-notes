# spec-097 - Deterministic performance harness

- Status: implemented (2026-11-12)
- Related: `spec-039`, `spec-048`
- Review sources: Claude and ChatGPT (no performance evidence)

## Purpose and scope

The suite proved behavior but had no repeatable measurement of the operations
the review called out: load, serialize, search, page lookup and CRDT sync. This
spec adds an opt-in integration harness in `tests/perf.rs`:

- builds workspaces at 1k, 10k and 50k blocks;
- measures JSON serialize/deserialize, ranked search, page lookup and a CRDT
  sync;
- prints timings and the JSON size;
- asserts only correctness (round-trip counts, non-empty search, non-empty
  CRDT update), so it stays deterministic on any machine.

Out of scope:

- Criterion or another benchmark dependency;
- performance budgets in the default gate;
- changing persistence based on the numbers.

## Acceptance criteria

P1. **Given** the default gate, **when** it runs, **then** the harness is
skipped unless `ION_OF_NOTES_PERF=1`.

P2. **Given** `ION_OF_NOTES_PERF=1`, **when** the harness runs, **then** it
prints timings for 1k, 10k and 50k blocks.

P3. **Given** the harness, **when** it runs, **then** the workspace round-trips,
search returns hits and CRDT sync returns an update.

P4. **Given** the default gate, **when** it runs, **then** test count and time
stay effectively unchanged.

## Measured baseline (debug build, Windows)

| Blocks | Serialize | Deserialize | Search | CRDT sync | JSON |
| ------ | --------- | ----------- | ------ | --------- | ---- |
| 1k | 27 ms | 20 ms | 0.8 ms | 1.3 ms | 0.6 MiB |
| 10k | 268 ms | 184 ms | 6.1 ms | 0.9 ms | 6.2 MiB |
| 50k | 1272 ms | 899 ms | 29.6 ms | 1.0 ms | 31.1 MiB |

The numbers are debug-build, single-run indicators, not release budgets. They
show that whole-document serialization dominates at 50k blocks; search and CRDT
sync stay cheap.

## Test scenarios

- `cargo test --no-default-features --test perf`
- `ION_OF_NOTES_PERF=1 cargo test --no-default-features --test perf -- --nocapture`
- Full quality gate

## Design

- One test file, no new dependency.
- The fixture is generated deterministically; ids and text are stable.
- `timed` prints each phase so the numbers can be compared across runs.

## Plan / tasks

- [x] Add the opt-in harness
- [x] Record the 1k/10k/50k baseline
- [x] Run the full gate and commit
