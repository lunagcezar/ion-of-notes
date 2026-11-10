# ADR-014 — Server login/settings UI

- **Status:** Accepted (2026-11-10)
- **Related:** ADR-013 (remote transport), spec-039 (sessions),
  spec-045

## Context

Remote mode required a token on the command line. A user-facing login flow was
needed without adding a second app shell or a browser page.

## Decision

1. **Settings → Server page.** Add a settings page with server URL, email,
   password, Connect and Disconnect. It reuses the existing settings window and
   command pipeline.
2. **Login before transport swap.** `remote::login` posts to
   `/api/v2/auth/login`; only a successful session builds `RemoteApi` and swaps
   the `DesktopApi`.
3. **One `NotionApp`.** The local and remote modes share the same app state,
   commands, CRDT session and realtime receiver; `replace_client` resets
   workspace-derived state when the transport changes.
4. **Startup prompt.** `--server` without `--token` starts locally but opens
   Settings → Server with the URL prefilled. `--token` without `--server` is an
   error.
5. **No token persistence.** The token lives only in the `Client`/`RemoteApi`
   for the session. The password is cleared after a successful connect and is
   never logged.

## Alternatives considered

| Alternative | Why not now |
| ----------- | ----------- |
| Separate login window/app shell | More state and duplicate UI; the settings page is sufficient. |
| Store token in config/keychain | Useful, but a security/storage decision of its own. |
| CLI-only token | Current behavior; not user-friendly. |
| Browser OAuth | No browser UI and no OAuth server yet. |
| Restart after login | Poor UX; swapping the transport in place is straightforward. |

## Consequences

Positive:

- Users can connect without copying tokens into a terminal.
- Wrong credentials leave the local workspace untouched.
- Local and remote modes share one UI and command path.
- The password is not retained after success.

Costs:

- Login is synchronous and briefly blocks the UI thread.
- The token is not persisted; users log in again after restart.
- There is no password reset/registration flow.
- TLS/private-CA settings are still external.

## Follow-up

- Persist the server URL and optionally the token in the OS keychain.
- Add password reset/registration and account management.
- Move login/REST calls to a background task with a progress state.
