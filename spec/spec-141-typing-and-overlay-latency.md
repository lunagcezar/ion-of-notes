# spec-141 - Typing and overlay latency

- Status: implemented (2026-11-25)
- Related: `spec-043`, `spec-132`, `spec-133`, `docs/performance-roadmap.md`
- Review source: 2026-11-25 ChatGPT latency review

## Purpose and scope

The second audit traced typing lag to synchronous work around every keystroke
and slow emoji scrolling to the sidebar/page still being rebuilt behind the
picker. This spec removes the highest-cost work from both paths.

### CRDT text edits

- `local_text` no longer collects both strings into `Vec<char>`. It uses byte
  prefix/backspace fast paths and lazy character iterators, keeping one small
  allocation for the new baseline.
- It returns `encode_update_v1()` (only this transaction) instead of
  `encode_diff_v1(&StateVector::default())` (the whole document).
- `CrdtSession::full_update()` remains available to seed a peer; clients must
  seed from the server document before sending incremental updates.

### Local typing no longer round-trips through the server

In local mode the editor already owns the text and the debounced REST commit
persists it; applying the CRDT update to the in-process server on every
keystroke duplicated the work and persisted twice. The UI now applies CRDT
updates only for remote servers, through the bounded worker. When the worker
queue is full the update is kept in a bounded `pending_crdt` queue and retried
from the UI loop instead of a synchronous HTTP call.

### Structural synchronization

`sync_new_blocks` is called from structural commands (insert, append, add after,
split, delete, drop) instead of every text change.

### Undo

A burst of typing is grouped into one undo entry: the workspace snapshot is
taken at most once per 0.8 s of continuous mutation instead of on every key.

### Icon picker

> **Superseded by spec-156 (2026-11-25):** the backdrop-only render was
> removed because it did not improve picker responsiveness. The background now
> renders normally and the emoji catalog is warmed in progressive batches.

While the icon picker is open, the sidebar tree and page body were not rendered;
a dimmed backdrop was painted instead. The picker grid already virtualizes rows.

### Snapshot cache

Writes no longer set the workspace snapshot to `None`. The cache is keyed by
`workspace.version` (bumped by every write), so reads after a write still see
the new version without forcing a clone.

### Projection

`project_workspace` builds a workspace from authorized pages and resources
instead of cloning the complete workspace and then discarding most of it.

## Acceptance criteria

P1. **Given** two clients seeded from the same document, **when** each edits the
same block, **then** both project the same text.

P2. **Given** an incremental local edit, **then** the encoded update excludes the
rest of the document.

P3. **Given** local typing, **then** no server CRDT apply/persist happens per
keystroke; the debounced commit persists the page.

P4. **Given** a full remote worker queue, **then** the UI keeps the update in the
pending queue and stays responsive.

P5. **Given** the icon picker open, **then** the sidebar and page body are not
rendered (superseded by spec-156: background renders normally).

P6. **Given** a member read after a write, **then** the snapshot cache version
matches `workspace.version` without manual invalidation.

## Test scenarios

- `tests/ui_crdt.rs` (`two_client_sessions_converge` uses an explicit seed;
  planning/projection suites).
- `tests/ui.rs` (undo/frame/commit helpers).
- `tests/web.rs` (snapshot projection suites).
- Manual `--screenshot` against the reference pictures.

## Plan / tasks

- [x] Replace the per-keystroke whole-document diff with a transaction update.
- [x] Remove the synchronous local CRDT apply and add a pending retry queue.
- [x] Move structural sync to structural commands.
- [x] Group typing into undo entries.
- [x] Skip background rendering behind the icon picker.
- [x] Remove forced snapshot invalidation and clone only authorized resources.
