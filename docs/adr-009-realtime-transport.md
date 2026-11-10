# ADR-009 — Real-time transport: WebSocket change feed first

- **Status:** Accepted (2026-11-09)
- **Related:** ADR-003 (self-hosted API), ADR-007 (Axum server),
  spec-040 (real-time change feed)

## Context

`GET /api/realtime` described a WebSocket channel and a CRDT merge model as
data, but no socket ran. The server already records every mutation in
`collab.changes` (`seq`, `page_id`, `op`, `actor`) and supports `If-Match`
conflict detection. Full CRDT integration (Yrs/Automerge) is a large change to
the page/block model and is not required to make collaboration observable.

## Decision

Ship a **WebSocket change feed + presence** first:

- `GET /ws/workspace/{id}` upgrades through Axum.
- Auth uses the existing Bearer token (header or `?token=` query).
- The server sends `ready`, replays changes after `?since=<seq>`, then streams
  live `change` messages.
- `presence` lists active sessions; `ping`/`pong` keeps the socket alive.
- `WorkspaceServer::write` publishes the committed `Change` through a Tokio
  broadcast hub after a successful persist.
- The protocol is JSON text frames with a `type` discriminator.
- Full CRDT merge remains a follow-up; `RealtimeContract.conflict_resolution`
  says so explicitly.

## Alternatives considered

| Alternative | Why not now |
| ----------- | ----------- |
| Server-Sent Events | Simpler, but one-way; presence and client pings need a second channel. |
| Long polling | Works through proxies, but higher latency and more request churn. |
| Full Yrs/Automerge now | Correct long-term, but requires a CRDT document model, per-block operations and a client integration; too large for this slice. |
| gRPC streaming | Strong typing, but heavier client/tooling and not browser-native. |
| Redis/NATS fan-out | Needed for multi-node, not for a single-node self-hosted server. |

## Consequences

Positive:

- Real-time changes are observable without a CRDT rewrite.
- `?since=` gives a simple, testable resume story.
- Presence and ping/pong are enough for a first collaborative UI.
- The existing change log, auth and REST mutations stay the source of truth.

Costs:

- Two clients editing the same page still rely on `If-Match` and page
  snapshots; there is no automatic text merge yet.
- A slow socket can lag the broadcast channel; lagged clients skip to the
  latest events and can refetch via the REST API.
- Multi-node fan-out and sticky sessions are not implemented.
- The browser client cannot set a Bearer header on a WebSocket, so the query
  token is the browser path; it must be short-lived and TLS-protected.

## Follow-up

- Add a client-side WebSocket adapter and UI presence indicators.
- **Yrs per-page text is implemented (spec-042);** awareness payloads and the
  client editor adapter remain.
- Add Redis/NATS fan-out when multi-node deployment is required.
- Consider short-lived WebSocket tickets instead of putting session tokens in
  the query string.
