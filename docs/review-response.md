# Review response (2026-11-12)

This document maps the external review findings (Kimi, ChatGPT, Claude) to the
work done in this cycle and to the items deliberately deferred. It is the
single place to check whether a finding was fixed, mitigated or planned.

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
| Principal-aware authorization | Direct page routes and owner-only management routes enforce roles and shares | Nested resources (blocks, comments, databases, views, files) still need parent-page resolution; list/search are not filtered per principal |
| Session storage | Session tokens are hashed at rest; remote session file is private on Unix | OS keychain integration and explicit Windows ACLs are not implemented |
| Webhook egress | Private/loopback/link-local hosts are rejected before delivery | DNS rebinding between resolution and connection remains possible; an operator egress allowlist is the durable mitigation |
| Documentation | Security, authorization, persistence, performance and review response are documented | Architecture/API status text still needs a full reconciliation pass |

## 3. Deferred with a documented plan

| Finding | Why deferred | Where it is tracked |
| ------- | ------------ | ------------------- |
| Asynchronous remote CRUD | Requires a UI request model and loading states | `docs/performance-roadmap.md` |
| CRDT coverage for structure/properties/databases | Needs conflict semantics per object family | `docs/performance-roadmap.md` |
| Awareness queue while offline | WebSocket lifecycle work | `docs/performance-roadmap.md` |
| Multi-line caret overlay | Text-layout projection work | `docs/performance-roadmap.md` |
| Incremental persistence | Measure snapshot cost first | `docs/performance-roadmap.md` |
| PostgreSQL / Redis | SQLite is intentionally single-node for now | `docs/performance-roadmap.md` |
| Password reset / registration / account UI | Product decision and email delivery are missing | `docs/authorization.md`, roadmap |
| Browser UI | Out of scope for the desktop-first product | roadmap |
| Block anchors for inline comments | Needs a comment model change | roadmap |
| Release artifacts / prebuilt binaries | CI packaging work | roadmap |
| Observability beyond `/metrics` | Tracing/outbox/sync-lag metrics | roadmap |
| PowerShell-free quality gate | A pure-cargo fallback is a tooling task | roadmap |
| Fuzzing and load tests | A dedicated test-infrastructure slice | roadmap |

## 4. Review scope and limitations

The reviews were static: none of the reviewers built the project, ran the test
suite or performed a live penetration test. The findings above were verified
with targeted regression tests in this repository; the security review is still
not a substitute for an independent audit.

The most valuable next step is a focused security review of the auth, session,
webhook and authorization paths after these changes, followed by the nested
resource authorization slice.
