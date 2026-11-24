# spec-116 - Operational hardening and hot paths

- Status: implemented (2026-11-24)
- Related: `spec-037`, `spec-039`, `spec-041`, `spec-054`, `spec-073`
- Review source: Kimi and ChatGPT security/performance reviews (2026-11-24)

## Purpose and scope

This spec closes the operational findings that do not need a new architecture:

- **Proxy trust:** `X-Forwarded-For` / `X-Real-IP` are used only when the
  direct peer is loopback or listed in `ION_OF_NOTES_TRUSTED_PROXIES`
  (comma-separated IPs or `*`). Otherwise the socket peer is the login key.
- **Fail-closed binding:** an unauthenticated `NoAuthStore` server refuses a
  non-loopback bind unless `ION_OF_NOTES_ALLOW_INSECURE_SERVER=1` is set.
- **Default password:** the development default is compiled into debug builds
  only; a release build requires an explicit owner password.
- **Login timing:** the dummy Argon2id hash is a constant, never an empty
  string fallback.
- **Errors:** a failed server task returns a generic 500 body and logs the
  detail with `tracing`.
- **Rate limits:** in-process maps purge expired windows and deny fresh keys
  when an attacker tries to grow them without bound; the login workspace budget
  is charged once per attempt; the shared SQLite limiter falls back to a local
  contingency on storage errors instead of failing open.
- **Session file:** Unix rewrites set `0600` on an existing file too.
- **JSON persistence:** only a missing file seeds; corrupt/unreadable files are
  preserved and the load returns an error. Replacement uses a sibling backup
  rather than copying over the live file.
- **Webhook egress:** the validated DNS result is pinned into a custom ureq
  resolver, closing the DNS-rebinding gap; deliveries also respect an explicit
  URL port.
- **Webhook secrets:** list responses use `WebhookSubscriptionDto`; only the
  owner and only the creation response see `verification_token`.
- **Retention:** the sync log and webhook delivery history have explicit caps.
- **Webhook persistence:** failed `persist_collab` writes are logged, not
  silently ignored.
- **SQLite:** one-page edits skip the position rewrite unless the page was
  created, deleted or reordered.
- **Search:** the initial index builds a page map once instead of an O(N²)
  scan; excerpts reuse the cached lowercase body and convert byte offsets to
  character offsets correctly.
- **UI:** rendering no longer marks the search cache dirty; a focused text
  field alone no longer triggers the per-frame workspace clone; block height
  fingerprints are cheap and selection is a `HashSet`.
- **Identity:** connections carry a `ConnectionKind` instead of relying on a
  `"session-"` string prefix.

Deferred with a documented plan:

- moving all webhook network I/O out of the global workspace lock (a full
  transactional outbox/worker split);
- a query-level database pagination layer;
- a block-id index and a height tree for the renderer;
- removing the hand-written loopback HTTP parser;
- OS keychain / explicit Windows ACLs for the remote session token.

## Acceptance criteria

P1. **Given** a request with a spoofed `X-Forwarded-For` from an untrusted peer,
**when** login is rate limited, **then** the socket peer is used.

P2. **Given** a local `NoAuthStore` server, **when** it is asked to bind a
public address, **then** startup fails unless the development override is set.

P3. **Given** a release build without `ION_OF_NOTES_OWNER_PASSWORD`, **when**
`ION_OF_NOTES_ALLOW_DEFAULT_PASSWORD=1` is set, **then** startup still fails.

P4. **Given** a rate-limit store error, **when** a request is checked, **then**
the local contingency still enforces the budget.

P5. **Given** a corrupt JSON workspace, **when** it is loaded, **then** the file
is unchanged and the load returns `InvalidData`.

P6. **Given** a webhook whose DNS answer changes between validation and
connection, **when** it is delivered, **then** the validated address list is
used without a second lookup.

P7. **Given** a one-page text edit on SQLite, **when** it persists, **then** no
`UPDATE workspace_pages SET position` statement runs.

## Test scenarios

- `tests/security.rs` (`login_workspace_budget_is_charged_once_per_attempt`).
- `tests/web.rs` (`unauthenticated_server_refuses_a_public_bind`).
- `tests/store.rs` (`corrupt_file_is_preserved_and_loading_fails`).
- `tests/webhooks.rs` (existing delivery/redirect tests now exercise the pinned
  resolver and explicit port).
- `tests/sqlite_store.rs` (`incremental_page_persist_updates_one_page_and_preserves_order`).
- `tests/model.rs` (search ranking/cache), `tests/ui.rs` (frame gating and
  block estimates).
- Full quality gate.

## Plan / tasks

- [x] Trust forwarded headers only from trusted proxies.
- [x] Refuse an unauthenticated public bind.
- [x] Gate the development password to debug builds.
- [x] Replace the dummy-hash fallback with a constant.
- [x] Log and generalize the 500 body.
- [x] Bound and purge the rate-limit maps; charge login once; add a local
  contingency.
- [x] Tighten an existing session file on Unix.
- [x] Preserve corrupt JSON and stop the copy-over fallback.
- [x] Pin validated webhook addresses and honor the URL port.
- [x] Redact webhook secrets from list responses.
- [x] Cap the sync log and webhook deliveries; log persistence failures.
- [x] Skip unchanged SQLite position rewrites.
- [x] Fix the O(N²) search index and the multibyte excerpt offset.
- [x] Stop marking the search cache dirty from rendering.
- [x] Stop cloning the workspace on every focused frame.
- [x] Cheap block fingerprints and `HashSet` selection.
- [x] Add `ConnectionKind`.
