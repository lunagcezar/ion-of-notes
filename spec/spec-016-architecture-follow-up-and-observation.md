# spec-016 — Architecture follow-up and clean-room server observation

## Purpose and scope

This spec records the follow-up work recommended by the spec-015 review and
defines how the project will learn about server-side behavior without crossing
the clean-room boundary. It has two tracks:

- **Track A — architecture and performance follow-up.** Close the remaining
  gaps in persistence, write amplification, lookup cost, version growth, module
  boundaries, localization, client error handling, and the optional loopback
  listener.
- **Track B — clean-room server observation.** Turn the Wireshark question into
  a repeatable, auditable process based on public behavior and the person's own
  account, with interception tools used only on traffic we control. The
  decision is recorded in `docs/adr-005-clean-room-server-observation.md`.

This spec is a plan, not an implementation. Each phase is small enough to be
its own working session and should be split into a numbered sub-spec if it
grows beyond one coherent change.

Out of scope:

- reading, listing, running or decompiling the original program or its files;
- bypassing authentication, certificate pinning, or access controls;
- copying proprietary payloads, identifiers, assets or source into the
  project;
- building the production Axum/PostgreSQL/WebSocket/Yrs server (still staged
  behind ADR-003);
- a visual redesign.

## Evidence

- `spec/spec-015-hardening-and-performance.md` and its review.
- `docs/adr-003-self-hosted-collaborative-api.md` (decided API).
- `docs/adr-004-modularization-decision.md` (split by responsibility, not by
  line count).
- `docs/adr-005-clean-room-server-observation.md` (observation decision).
- Current code:
  - `WorkspaceServer` persists only `Workspace`; `CollabState` is in memory.
  - `ui/app.rs` calls `client.replace_workspace` after any workspace change.
  - `Workspace::page`, `children` and `breadcrumbs` scan the page vector.
  - `CollabState::versions` grows without a retention limit.
  - `ui/app.rs` is about 1,500 lines and mixes state, commands and input.
  - only a few settings strings use `rust-i18n`.
  - `client.rs` uses `serde_json::to_string(...).unwrap_or_default()`.
  - `serve_forever` handles one connection at a time.
  - `cargo fmt --check` is not clean (pre-existing formatting drift).

## Track A — acceptance criteria

### A1. Collaboration state is persisted

- **Given** members, shares, the sync log and page versions, **when** the
  server is restarted, **then** all of them are restored exactly.
- **Given** a collaboration store whose write fails, **when** a mutation is
  handled, **then** the API returns `500` with an error envelope.
- **Given** an existing workspace-only data file, **when** the new adapter
  loads it, **then** the workspace is preserved and collaboration state starts
  from the documented seed without corrupting the file.

Implementation direction: add a `CollabStore` port (small, like
`WorkspaceStore`) with `JsonCollabStore` and `NoCollabStore` adapters. The
composition root wires both stores. Keep the workspace and collaboration
documents separate so a corrupt collaboration file cannot destroy pages.

### A2. Per-page writes instead of full-workspace replacement

- **Given** an edit to one page, **when** the UI commits, **then** only that
  page is sent with `PATCH /api/pages/{id}`; the full `PUT /api/workspace` is
  reserved for multi-page operations (delete subtree, reset, import).
- **Given** rapid typing, **when** the user pauses, **then** at most one
  debounced persistence request is sent for the settled page; leaving the app
  flushes pending edits.
- **Given** a structural command that changes several pages, **when** it runs,
  **then** the fallback full-snapshot path is used and still persists.

Implementation direction: a pure `PendingEdits` / `EditQueue` helper decides
which page ids changed and when to flush; the UI applies it through `Client`.
Do not send a full workspace payload for a single-page text edit.

### A3. Workspace lookup index

- **Given** a workspace with many pages, **when** `page`, `children` or
  `breadcrumbs` is queried, **then** lookup is not a full linear scan.
- **Given** pages are added, removed or replaced, **when** the index is
  rebuilt, **then** every non-null parent still resolves to an existing page
  (I2) and duplicate ids are still rejected (I1).
- **Given** JSON serialization, **when** the workspace is saved, **then** the
  on-disk shape does not change; the index is derived state.

Implementation direction: an internal `WorkspaceIndex` (id → position,
parent → children) rebuilt on mutation or maintained transactionally. Keep it
private so serde and the public DTOs stay stable.

### A4. Version retention

- **Given** more than the retention limit of versions for one page, **when** a
  new version is recorded, **then** the oldest versions are pruned.
- **Given** the version-history API, **when** it is queried, **then** results
  are still newest-first and the retention limit is documented.
- **Given** a restore, **when** the selected snapshot is restored, **then** the
  restore itself is recorded as a new version (audit trail).

Default limit: keep the newest 50 versions per page, configurable in one
constant or setting. Do not silently drop the current page's latest version.

### A5. UI module split

- **Given** `ui/app.rs`, **when** it is split, **then** `ui/commands.rs` owns
  the `Command` and `ViewState` shapes, `ui/view.rs` owns the view snapshot,
  and `ui/input.rs` owns key translation and shortcut collection.
- **Given** the split, **when** the app runs, **then** behavior and visuals do
  not change; the existing screenshot and all tests stay green.
- **Given** the `ui.rs` facade, **when** other modules import UI types, **then**
  they use the facade or the new cohesive modules, not a 1,500-line `app`.

### A6. Localization coverage

- **Given** the locales `en`, `pt-BR` and `eo`, **when** the user switches
  language, **then** every key in the central string table is translated or
  falls back to English.
- **Given** a missing key in one locale, **when** the completeness test runs,
  **then** it fails with the missing key names.
- **Given** a string that is not user-visible (protocol, log, code), **when**
  the key table is built, **then** it is not required to be translated.

Implementation direction: add `tests/i18n.rs` that compares key sets across
locale files. Expand coverage incrementally, starting with the Settings
window, sidebar, page actions and dialogs.

### A7. Client serialization errors are explicit

- **Given** a value that fails serialization, **when** a client method is
  called, **then** it returns `ClientError::Encode` (or equivalent) instead of
  sending an empty body.
- **Given** the existing decode error, **when** a response is malformed, **then**
  `ClientError::Decode` is still returned.
- No client method may use `serde_json::to_string(...).unwrap_or_default()`.

### A8. Concurrent loopback listener

- **Given** two clients connected to `--serve`, **when** both send a request,
  **then** both complete without one blocking the other indefinitely.
- **Given** shared server state, **when** requests run concurrently, **then**
  access is serialized through a mutex/channel; no data race or torn write.
- **Given** a connection error, **when** the listener continues, **then** it
  logs and accepts the next connection.

Implementation direction: `Arc<Mutex<WorkspaceServer>>` with a worker per
connection or a small bounded pool. The listener remains loopback-only by
default; binding a public interface requires an explicit address and a warning.

### A9. Settings search filters rows, not only the rail

- **Given** a settings query, **when** a section is open, **then** only rows
  whose title or description match are shown inside that section.
- **Given** no row matches, **when** the section is rendered, **then** a
  section-level “no match” message is shown.
- The pure matcher stays in `ui/settings_search.rs`; the row table is the one
  source of truth.

### A10. Formatting baseline

- **Given** a clean checkout, **when** `cargo fmt --check` runs, **then** it
  exits successfully.
- The formatting change is a separate commit from behavior changes, so the
  diff remains reviewable.

## Track B — clean-room observation path

The full decision is in `docs/adr-005-clean-room-server-observation.md`. The
spec defines the operational steps.

### B1. Permission and scope gate

- **Given** any capture activity, **when** it starts, **then** the operator has
  confirmed: the account is their own, the instance is theirs or they have
  explicit permission, and no access control or term is being bypassed.
- **Given** a request to capture a third-party service without permission,
  **when** it is proposed, **then** the work stops and falls back to public
  documentation and own-account behavior.

### B2. Behavioral inventory

- **Given** the product UI and public help pages, **when** the inventory is
  written, **then** `docs/api-observations.md` lists each observed operation
  (page list, page read, create, update, delete, search, members, shares,
  sync, export, realtime) with its source.
- Every entry is marked **observed** or **inferred**.

### B3. Own-account HAR capture

- **Given** the person uses their own account normally, **when** requests are
  recorded with browser DevTools, **then** the HAR is stored under
  `fixtures/har/` only after redaction.
- **Given** a capture, **when** it is reviewed, **then** cookies, bearer
  tokens, email addresses, workspace names and other personal data are
  removed; the redaction checklist is recorded in the clean-room log.

### B4. Contract derivation

- **Given** the observed requests and responses, **when** the contract is
  written, **then** it covers methods, paths, status codes, JSON envelopes,
  pagination/cursors, error shapes and realtime framing.
- **Given** a difference from ADR-003, **when** it is confirmed, **then**
  ADR-003 is updated or a new ADR is added before implementation.

### B5. Mock server and contract tests

- **Given** the derived contract, **when** the mock server is implemented,
  **then** `tests/contract.rs` drives the real `Client` against it and asserts
  the documented responses.
- **Given** a contract test failure, **when** it is investigated, **then** the
  observation is corrected or the mock is marked as an estimate; tests are not
  weakened to hide a mismatch.

### B6. Tooling policy

- **Given** a need to inspect HTTP, **when** a tool is chosen, **then**
  DevTools/HAR is preferred; `mitmproxy` or Charles is used only on traffic we
  control; Wireshark is used only for our own loopback/self-hosted traffic or
  with explicit written permission.
- **Given** TLS, **when** decryption is proposed, **then** it uses a client we
  control (`SSLKEYLOGFILE`) or a trusted proxy certificate; certificate
  pinning and access controls are never bypassed.
- **Given** a capture of the original service, **when** it is stored, **then**
  it is limited to the minimum necessary and never includes proprietary
  source, assets or secrets.

### B7. Clean-room log

- **Given** each observation session, **when** it ends, **then**
  `docs/reverse-engineering-log.md` records the date, source, what was
  observed, what was inferred, what was rejected, and the redaction status.
- **Given** a reviewer, **when** they read the log, **then** they can trace
  every implemented server behavior back to a public or own-account source.

### B8. Stop conditions

- **Given** a capture would require bypassing authentication, certificate
  pinning or terms, **when** it is discovered, **then** stop immediately.
- **Given** a capture reveals proprietary source or internal code, **when** it
  is recognized, **then** do not copy it; record only the behavioral fact and
  rely on the public contract.

## Test scenarios (written before implementation)

- `tests/collab_store.rs` (new): JSON collaboration round-trip, missing file
  seed, corrupt file fallback, failing store → `500`.
- `tests/model.rs`: index rebuild after add/remove/replace; I1/I2 still hold;
  JSON shape unchanged.
- `tests/collab.rs`: version retention prunes oldest; restore records a new
  version.
- `tests/client.rs`: `ClientError::Encode` on a failing serializer; per-page
  `update_page` path.
- `tests/ui.rs`: `PendingEdits`/debounce helper, settings row matcher.
- `tests/i18n.rs` (new): locale key completeness and English fallback.
- `tests/server.rs`: two concurrent loopback requests; failing connection does
  not stop the listener.
- `tests/contract.rs` (new, Track B): client against the derived mock contract.
- `cargo fmt --check`: clean baseline.

## Plan / tasks

### Phase 0 — baseline

- [ ] Run `cargo fmt` once as a formatting-only commit; keep behavior unchanged.
- [ ] Confirm `cargo build`, `cargo test --all-targets`, and
  `cargo clippy --all-targets -- -D warnings` are green.

### Phase A — architecture and performance

- [ ] A1 `CollabStore` port + JSON adapter + composition-root wiring + tests.
- [ ] A2 `PendingEdits`/debounce helper + per-page `PATCH` path + tests.
- [ ] A3 private `WorkspaceIndex` + invariant tests.
- [ ] A4 version retention constant + pruning + restore audit test.
- [ ] A5 split `ui/app.rs` into `ui/commands.rs`, `ui/view.rs`, `ui/input.rs`.
- [ ] A6 expand locale keys and add `tests/i18n.rs`.
- [ ] A7 `ClientError::Encode` and remove `unwrap_or_default` from `client.rs`.
- [ ] A8 concurrent `--serve` listener + concurrency test.
- [ ] A9 settings row filtering inside sections.
- [ ] A10 keep `cargo fmt --check` green.

### Phase B — clean-room observation

- [ ] B1 record the permission/scope gate.
- [ ] B2 write `docs/api-observations.md` from public behavior.
- [ ] B3 capture own-account HAR, redact, store under `fixtures/har/`.
- [ ] B4 derive/update the contract and ADR if needed.
- [ ] B5 implement the mock server and `tests/contract.rs`.
- [ ] B6 record the tooling choice and TLS handling.
- [ ] B7 maintain `docs/reverse-engineering-log.md`.
- [ ] B8 review stop conditions before every capture.

## Risks and open questions

- **Collab persistence format:** separate file vs. a single `ServerSnapshot`.
  Decision: separate files keep a corrupt collaboration document from
  destroying pages; revisit if atomic multi-document transactions are needed.
- **Debounce and crashes:** a debounce can lose the last keystrokes on a hard
  crash. Flush on focus loss, page change and app exit; keep the write cheap.
- **Index consistency:** derived state must be rebuilt after every structural
  mutation; a stale index is worse than a linear scan. Tests must cover it.
- **Retention vs. audit:** pruning versions reduces history. The limit is
  documented and configurable; the restore operation always creates a new
  version.
- **Concurrent listener:** a global mutex serializes mutations; reads may still
  clone snapshots. Measure before adding a pool.
- **i18n scope:** translating every string is a large mechanical change; do it
  by surface (settings, sidebar, dialogs) and keep the key-completeness test.
- **Clean-room scope:** public/own-account observations may be incomplete; the
  contract must mark estimates and never claim access to proprietary internals.
- **Formatting drift:** the repository was not rustfmt-clean before spec-016;
  the formatting commit is intentionally separate.

## Requirement mapping

| Requirement | Spec |
| ----------- | ---- |
| FR-63 — collaboration state persists through `CollabStore` | `spec-016` A1 |
| FR-64 — per-page writes and debounced persistence | `spec-016` A2 |
| FR-65 — workspace lookup index | `spec-016` A3 |
| FR-66 — version retention and restore audit | `spec-016` A4 |
| FR-67 — UI module split | `spec-016` A5 |
| FR-68 — localization coverage | `spec-016` A6 |
| FR-69 — explicit client encode errors | `spec-016` A7 |
| FR-70 — concurrent `--serve` | `spec-016` A8 |
| FR-71 — settings row filtering | `spec-016` A9 |
| NFR-13 — `cargo fmt --check` clean | `spec-016` A10 |
| NFR-14 — clean-room observation protocol | `spec-016` Track B, ADR-005 |
| NFR-15 — performance budgets | `spec-016` A2, A3, A8 |
