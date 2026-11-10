# ADR-013 — Remote WebSocket desktop transport

- **Status:** Accepted (2026-11-10)
- **Related:** ADR-007 (Axum), ADR-009 (realtime), ADR-012 (client Yrs),
  spec-044

## Context

The desktop was hard-wired to the in-process `WorkspaceServer`. The server
already exposes HTTP and WebSocket, so a remote mode only needed a client-side
transport behind the existing `WorkspaceApi` seam.

## Decision

1. **Blocking REST with ureq.** The egui thread is synchronous; ureq keeps the
   client simple and matches the existing webhook choice. Network failures map
   to a `503` envelope so the existing `ClientError` path applies.
2. **Background WebSocket thread.** A dedicated thread owns a current-thread
   Tokio runtime and a `tokio-tungstenite` socket. It reconnects with
   exponential backoff and tracks the last sequence for `?since=`.
3. **Typed broadcast to the UI.** The socket thread forwards
   `ServerMessage`s through a Tokio broadcast channel; `NotionApp` drains it
   each frame, exactly like the in-process hub.
4. **REST CRDT routes.** Remote clients use
   `POST /api/v2/crdt/sync` and `POST /api/v2/crdt/update` with base64 Yrs
   payloads; the WebSocket remains the live update/awareness channel.
5. **`DesktopApi` local/remote enum.** `Client<DesktopApi>` keeps the UI type
   stable. Local mode wraps `WorkspaceServer`; remote mode wraps `RemoteApi`.
6. **CLI token.** `--server` and `--token` select remote mode. A login UI and
   token storage are separate follow-ups.

## Alternatives considered

| Alternative | Why not now |
| ----------- | ----------- |
| Async UI client | egui is synchronous; an async runtime in the UI adds complexity. |
| reqwest for REST | Larger dependency and async/blocking boundary; ureq fits the sync path. |
| WebSocket for CRDT sync | Request/response correlation over the socket is more complex than a REST call. |
| Separate remote `NotionApp` type | Generic UI plumbing and duplicated wiring; the enum keeps one UI type. |
| Token in an env var only | CLI is explicit and testable; env support can be added later. |
| Full reconnect/offline queue | Larger reliability slice; the `since` cursor is the first step. |

## Consequences

Positive:

- The desktop can use a real `--serve` instance over HTTP/WebSocket.
- Local and remote modes share `Client`, commands, CRDT and awareness code.
- Reconnect resumes from the last sequence.
- Remote CRDT has a testable REST surface.

Costs:

- A token must be supplied manually; no login UI yet.
- REST requests block the UI thread briefly (loopback/LAN latency).
- The WS thread has no offline command queue; awareness sent while disconnected
  is dropped.
- TLS/private CA configuration is not exposed yet.

## Follow-up

- Add a server URL/token settings UI and login flow.
- Add a retry/offline queue for REST writes.
- Add WebSocket ticket auth instead of query tokens.
- Add multi-node fan-out and TLS deployment guidance.
