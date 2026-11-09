# ADR-008 — Sessions, Argon2id and SQLite storage

- **Status:** Accepted (2026-11-09)
- **Related:** ADR-003 (self-hosted API), ADR-007 (Axum server),
  spec-039 (sessions and SQLite)

## Context

The server authenticated every request with the demo `owner-token` and stored
workspace/collaboration state in JSON files. The self-hosted target needs real
accounts, durable storage and sessions that survive a restart. The desktop must
keep working without a server, and the JSON adapter must remain a migration
source and fallback.

## Decision

1. **SQLite first.** Add `SqliteStore` implementing `WorkspaceStore`,
   `CollabStore` and `AuthStore`. Workspace and collaboration snapshots are
   stored as JSON documents in SQLite (WAL mode); normalized page/block tables
   are deferred until the document store becomes the bottleneck.
2. **JSON remains the default.** The desktop and plain `--serve` use JSON.
   `--database <path>` selects SQLite for the server. An empty SQLite database
   imports the existing JSON documents once.
3. **Argon2id passwords.** Accounts store PHC strings produced by the
   `argon2` crate; plaintext passwords are never persisted.
4. **Opaque sessions.** Login returns `sess_<uuid>` exactly once. Only its
   SHA-256 hash is stored, with a 30-day expiry. The session table is the
   source of truth after a restart; `connection_for_token` falls back to it.
5. **Bearer transport.** Existing v2 Bearer auth stays the transport. Login is
   the only route handled before authentication; logout and session info run
   after the connection is resolved.
6. **Owner bootstrap.** The first SQLite run seeds the owner account from
   `ION_OF_NOTES_OWNER_PASSWORD`; without it, a documented development default
   is used and a warning is printed.

## Alternatives considered

| Alternative | Why not now |
| ----------- | ----------- |
| JWT access tokens | Stateless, but revocation and logout need a server-side denylist; opaque sessions are simpler and auditable. |
| Cookie sessions only | Browser-friendly, but the desktop and API clients already use Bearer tokens; cookies can be added later for a browser UI. |
| PostgreSQL first | Better for multi-node, but heavier to self-host; SQLite is enough for a single-node server and keeps the first slice small. |
| SQLx async store | Natural with Axum, but the existing store ports are synchronous; `rusqlite` keeps the ports stable. A Postgres/SQLx adapter can replace it later. |
| Store raw tokens | Simpler lookup, but a database leak would expose live sessions. Hashing at rest is cheap. |
| Full account UI | Larger scope; the API and storage are the foundation for a later UI spec. |

## Consequences

Positive:

- Passwords and sessions are not stored in plaintext.
- Sessions survive server restarts.
- JSON data migrates without a separate tool.
- The store ports and domain stay unchanged.

Costs:

- SQLite is single-node; PostgreSQL and pooling remain future work.
- `rusqlite` is synchronous, so the Axum adapter keeps using `spawn_blocking`.
- The development default owner password is not production-safe; deployments
  must set `ION_OF_NOTES_OWNER_PASSWORD`.
- Browser cookie/CSRF support and account management UI are still missing.

## Follow-up

- spec-040: WebSocket/CRDT transport.
- spec-041: TLS webhook delivery and shared rate limiting.
- Normalize pages/blocks and add a PostgreSQL adapter when multi-node
  deployment is required.
- Add account management UI and a first-run password setup flow.
