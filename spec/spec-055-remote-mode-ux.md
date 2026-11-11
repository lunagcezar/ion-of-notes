# spec-055 - Remote-mode UX

- Status: implemented (2026-11-11)
- Related: `spec-044`, `spec-045`, `spec-052`, `docs/deployment.md`

## Purpose and scope

Remote mode currently asks for the password again after every restart and runs
the login plus initial workspace fetch on the egui thread. A slow or
unreachable server therefore freezes the window for up to the 15-second HTTP
timeout. This spec makes the connection flow non-blocking and remembers the
session token safely enough for a self-hosted desktop client.

In scope:

- Persist the remote session (`base_url`, `email`, `token`) in
  `remote-session.json` next to the workspace data; never persist the password.
- Restore the remembered session on startup when `--server` is provided without
  `--token`; fall back to the login page when there is no session or it fails.
- Move login and the initial `build_remote_desktop` fetch to a background
  thread; the UI polls a result slot and shows `Connecting...`.
- Disable the Connect button while a connection is in progress and clear the
  saved session on an explicit disconnect or failed restore.
- Use the same non-blocking path for `--server --token`.
- Add session round-trip/clear tests and update the docs.

Out of scope:

- No per-request async conversion of every `Client` CRUD call. Those calls
  remain synchronous with the existing 15-second timeout; the worst blocking
  case (login + initial workspace fetch) is removed here.
- No OS keychain integration. The session file is the bearer-token boundary;
  Windows profile ACLs and Unix `0600` are the protection.
- No refresh-token or server-side session extension.

## Evidence

- `src/ui/apply/server.rs` calls `remote::login` and
  `app::build_remote_desktop` synchronously inside `ConnectServer`.
- `main.rs` opens the login page on `--server` without a token and never tries
  a remembered session.
- `remote.rs` already owns `RemoteApi::connect` and `login`; the HTTP timeout is
  15 seconds.
- `NotionApp` already has a background result-slot pattern for Ollama
  (`ai_slot`/`poll_ai`), so the same shape can be reused for the server.
- `tests/remote.rs` starts an in-process Axum server and covers login and
  REST/WebSocket behavior.

## Acceptance criteria

1. **Given** a successful Settings → Server login, **when** the app restarts
   with `--server`, **then** it loads `remote-session.json` and reconnects
   without asking for the password.
2. **Given** a missing, malformed, other-server or rejected session, **when**
   startup restores it, **then** the session file is cleared and the Server
   settings page opens for a normal login.
3. **Given** a slow login or workspace fetch, **when** Connect is pressed,
   **then** the UI keeps rendering, shows `Connecting...`, and applies the
   result on a later frame.
4. **Given** Disconnect, **when** it runs, **then** the saved session is
   removed and the local workspace is restored.
5. **Given** `--server --token`, **when** the app starts, **then** the token
   connection also runs in the background.
6. **Given** the full gate, **when** fmt, both clippy passes, core + desktop
   tests, docs and the preview screenshots run, **then** all are green.

## Test scenarios (written before implementation)

- **Red:** add the session round-trip test and the background connect path
  before the helpers exist; `cargo test --test remote` and `cargo test --test
  ui` fail to compile.
- **Session:** save a session, load it for the same base URL, reject a
  different base URL, clear it and confirm it is gone.
- **UI:** the existing `tests/ui.rs` and `--screenshot` previews still render;
  the Server settings page shows the busy state through `ViewState`.
- **Manual:** run a local `--serve`, connect through Settings, restart with
  `--server`, and confirm the workspace opens without a password prompt.
- **Full gate:** `pwsh -File scripts/quality.ps1` and the preview screenshots.

## Design

```text
src/remote.rs
  RemoteSession { base_url, email, token }
  session_path() / save_session / load_session / clear_session
  *_at(path, ...) variants for tests
  Unix: chmod 0600

src/ui/app.rs
  server_busy, server_slot
  restore_server_session(), connect_server_with_token()
  poll_server() on every frame

src/ui/apply/server.rs
  start_server_connect() spawns a thread
  ConnectServer / DisconnectServer use the worker
  poll_server applies success/error and clears a rejected session

src/main.rs
  --server without token -> restore_server_session
  --server with token    -> connect_server_with_token
```

The password is only kept in `server_password` for the current login attempt
and is cleared after success. The token file is a bearer secret; the README
documents the file location and the keychain follow-up.

## Plan / tasks

- [x] Write this spec and record the red step.
- [x] Add the session model and file helpers with tests.
- [x] Add `server_busy`/`server_slot`, background connect and `poll_server`.
- [x] Restore saved sessions on startup and clear on disconnect/failure.
- [x] Add the busy state to the Server settings page.
- [x] Run tests, clippy, fmt, docs and the preview screenshots.
- [x] Update README, architecture, deployment, testing, requirements,
      glossary, spec index and changelog.
- [x] Commit the whole `app/` tree naming this spec.
