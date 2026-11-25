# Review response (2026-11-12)

This document maps the external review findings (Kimi, ChatGPT, Claude) to the
work done in this cycle and to the items deliberately deferred. It is the
single place to check whether a finding was fixed, mitigated or planned.

## 0. 2026-11-24 review convergence (Kimi + ChatGPT)

| Finding | Status | Response | Evidence |
| ------- | ------ | -------- | -------- |
| Nested block/comment/database/view resources bypassed page shares | fixed | `server::authorization` resolves the owning page and applies read/write access | spec-115, `tests/authorization.rs` |
| `/api/workspace`, `/api/pages`, search, sync and v1 returned every page | fixed | principal-aware projection and filtered lists | spec-115 |
| CRDT REST/WebSocket did not authorize the page | fixed | read/write checks before sync/update; backlog and live messages filtered | spec-115, `tests/authorization.rs` |
| Webhook secrets and deliveries visible to members | fixed | routes owner-only; list uses `WebhookSubscriptionDto` without the token | spec-115, `tests/web.rs` |
| Export exposed the whole workspace to any reader | fixed | owner-only v1/v2 export | spec-115 |
| `X-Forwarded-For` bypassed login throttling | fixed | trusted-proxy check (loopback or `ION_OF_NOTES_TRUSTED_PROXIES`) | spec-116, `docs/security-hardening.md` |
| Hardcoded `owner-token` reachable on a network bind | mitigated | unauthenticated non-loopback bind is refused unless explicitly overridden | spec-116, `tests/web.rs` |
| Default owner password compiled into release | mitigated | development default is debug-only | spec-116 |
| Session token in the WebSocket query string | mitigated | remote client sends `Authorization`; query fallback kept for old clients/tests | spec-116 |
| 500 leaked task internals | fixed | generic body plus `tracing::error` | spec-116 |
| Dummy hash could fall back to an empty string | fixed | constant Argon2id PHC hash | spec-116 |
| Rate-limiter maps grew without bound | fixed | purge at a bound; new keys denied while full | spec-116 |
| Shared limiter failed open | fixed | local contingency limiter on storage error | spec-116 |
| Login workspace budget charged twice | fixed | `check_login` charges IP + account + workspace once | spec-116, `tests/security.rs` |
| Session file kept permissive modes | fixed (Unix) | `set_permissions(0600)` before rewriting | spec-116 |
| JSON corruption was overwritten with a seed | fixed | only `NotFound` seeds; errors propagate; desktop opens read-only in memory | spec-116, `tests/store.rs` |
| Copy-over fallback could truncate the live file | fixed | sibling backup + rename; failure restores the backup | spec-116 |
| Webhook DNS rebinding | fixed | validated addresses pinned in a custom ureq resolver | spec-116, `tests/webhooks.rs` |
| Webhook persistence errors swallowed | fixed | `persist_collab_logged` | spec-116 |
| Sync log and delivery history unbounded | fixed | `MAX_CHANGES`, `MAX_WEBHOOK_DELIVERIES` | spec-116 |
| SQLite rewrote all positions on every edit | fixed | refresh only on create/delete/reorder | spec-116 |
| O(N²) initial search index | fixed | one page map per dirty pass | spec-116, `tests/model.rs` |
| Excerpt offset used bytes as characters | fixed | byte-to-char conversion | spec-116 |
| Rendering marked the search cache dirty | fixed | `page_mut_untracked` in the renderer | spec-116 |
| Focused text field cloned the workspace every frame | fixed | snapshot only when a text/key event arrives | spec-116, `tests/ui.rs` |
| Block fingerprint hashed all text; selection was a Vec | fixed | cheap metadata fingerprint and `HashSet` | spec-116 |
| String-prefix connection identity | fixed | `ConnectionKind` | spec-116 |
| Offline awareness lost while disconnected | fixed | one-entry pending cursor flushed on reconnect (spec-122) | `tests/remote.rs` |
| Multi-line remote caret used a first-line width | fixed | galley row geometry and per-row selection (spec-123) | `tests/ui.rs` |
| Observability stopped at request/webhook counters | fixed | pages, blocks, members, version, sync, request-log gauges (spec-124) | `tests/web.rs` |
| No randomized invariant coverage | fixed | deterministic mixed-mutation fuzz test (spec-125) | `tests/model.rs` |
| No concurrent latency baseline | fixed | opt-in 8-thread read/write p50/p95 harness (spec-126) | `tests/load.rs` |
| PowerShell-free quality gate | fixed | Markdown checks run in `tests/docs.rs` and `scripts/quality.sh` needs no pwsh (spec-128) | `tests/docs.rs` |
| Concurrent SQLite instances could fail on startup/count | fixed | busy timeout, bounded schema retry and a four-connection count test (spec-129) | `tests/shared_limits.rs` |
| No tagged binaries | fixed | `v*` tag workflow builds Linux server and Windows desktop/server archives (spec-130) | `.github/workflows/release.yml` |
| No inline comment text ranges | fixed at the API/model layer | optional `anchor { block_id, start, end }` with ownership/range validation (spec-131); the desktop comment panel still renders page-level threads | `tests/comments.rs` |
| Remote UI blocked on CRUD | fixed | create/delete/invite/share/version restore run on the worker (spec-132) | `tests/remote.rs` |
| Webhook network I/O under the global mutex | fixed | events queue in the outbox; the worker sends outside the lock (spec-117) | `tests/webhooks.rs` |
| Database view pagination after loading every row | fixed | index selection + `PageWindow`; only the requested slice is cloned (spec-118) | `tests/model.rs`, `tests/pagination.rs` |
| Renderer still visits every block; height tree absent | fixed | Fenwick height index renders only the visible window (spec-119) | `tests/ui.rs`, screenshot |
| Hand-written loopback HTTP parser and three HTTP layers | fixed | shared auth seam, bounded parser (spec-120) and `serve_on` now uses Axum (spec-121) | `tests/server.rs`, `tests/contract.rs` |
| Windows session-file ACL | fixed | explicit current-user DACL through `windows-sys` (spec-127) | `tests/remote.rs` |

The security items above are enforced server-side. The desktop UI is not an
authorization boundary.

## 1. Implemented in this cycle

| Area | Finding | Response | Spec | Evidence |
| ---- | ------- | -------- | ---- | -------- |
| Auth boundary | v1 routes were unauthenticated on a production-capable server | v1 now requires a session token when `AuthStore::requires_auth()` is true; `/health` stays public; local `NoAuthStore` remains in-process | spec-073 | `tests/security.rs` |
| Static token | `owner-token` was registered even with a real auth store | The demo owner connection is only registered for `NoAuthStore`; a real store cannot authenticate with `owner-token` | spec-073 | `tests/security.rs` |
| Remote v1 | The remote client did not attach its token to v1 | `Client::call` adds the bearer header when a token is configured | spec-073 | `tests/security.rs` |
| Login limits | One global login bucket allowed lockout and no per-account protection | Dedicated per-client and per-account login limiter with a workspace cap | spec-073 | `tests/security.rs` |
| Memory growth | Request log and session connections grew without bound | Request log cap; sessions resolved from the auth store instead of retained connections | spec-073 | `tests/security.rs` |
| Session file | Write-then-chmod race on Unix | File is opened with `0600` before writing; Windows limitation documented | spec-073 | `tests/remote.rs` (`cfg(unix)`) |
| Timeouts | No request timeout in the Axum adapter | 30-second `TimeoutLayer` | spec-073 | `src/server/web.rs` |
| Owner password | Development password could be used silently in production | Startup fails unless an env/file password is set; `ION_OF_NOTES_ALLOW_DEFAULT_PASSWORD=1` is the explicit development override | spec-073 | `tests/security.rs` |
| Login timing | Unknown email returned early without hashing | Dummy Argon2id verification before the same `401` | spec-073 | `src/server/handlers/auth.rs` |
| Persistence | Workspace and collaboration snapshots could commit separately | `SnapshotStore` port; SQLite writes both rows in one transaction; failed writes reload the durable state | spec-074 | `tests/persistence.rs` |
| Webhook SSRF | Any HTTPS URL was accepted, including private/link-local targets | Creation-time private-host policy plus delivery-time DNS resolution guard; development override for loopback | spec-075 | `tests/webhooks.rs` |
| Authorization | Sessions received all capabilities and page shares were not enforced | Role-derived capabilities; direct/general page share checks; owner-only management routes; v1 and v2 page checks | spec-076 | `tests/authorization.rs` |

## 2. Partially addressed

| Finding | What is done | What remains |
| ------- | ------------ | ------------ |
| Principal-aware authorization | Direct and nested resources, projections, CRDT/WebSocket, webhooks and export are enforced (spec-115) | Client-side hiding and page-level-only grant granularity |
| Session storage | Session tokens are hashed at rest; the remote session file is private on Unix and has an owner-only DACL on Windows (spec-127) | OS credential-store/keychain integration would be a further hardening step |
| Webhook egress | Private hosts are rejected and the validated DNS answer is pinned | An operator egress allowlist is still recommended |
| Documentation | Security, authorization, persistence, performance and review response are documented | Architecture/API status text still needs a full reconciliation pass |

## 3. Deferred with a documented plan

| Finding | Why deferred | Where it is tracked |
| ------- | ------------ | ------------------- |
| CRDT coverage for structure/properties/databases | Needs conflict semantics per object family | `docs/performance-roadmap.md` |
| Per-block persistence | Page rows are incremental (spec-114); block-level tables would add schema and migration complexity | `docs/performance-roadmap.md` |
| PostgreSQL / Redis | SQLite is intentionally single-node for now | `docs/performance-roadmap.md` |
| Password reset / registration / account UI | Product decision and email delivery are missing | `docs/authorization.md`, roadmap |
| Browser UI | Out of scope for the desktop-first product | roadmap |
| Block anchors for inline comments | API/model anchors are implemented (spec-131); UI inline threads remain | roadmap |

## 4. Review scope and limitations

The reviews were static: none of the reviewers built the project, ran the test
suite or performed a live penetration test. The findings above were verified
with targeted regression tests in this repository; the security review is still
not a substitute for an independent audit.

The most valuable next step is a focused security review of the auth, session,
webhook and authorization paths after these changes, followed by the nested
resource authorization slice.
