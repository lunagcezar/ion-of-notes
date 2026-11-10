# spec-045 — Server login and settings UI

## Purpose and scope

Remote mode required `--server` and `--token` on the command line. This spec
adds a Settings → Server page where the user enters the server URL, email and
password, logs in, switches to the remote workspace, and can disconnect back to
the local workspace. Starting with `--server` and no token opens the page.

In scope:

- Settings → Server page with URL, email, password, Connect and Disconnect;
- `remote::login` using `/api/v2/auth/login`;
- live switch between local and remote `DesktopApi` transports;
- workspace reload, tab/history reset, CRDT/realtime resubscribe;
- startup prompt for `--server` without `--token`;
- status and validation messages.

Out of scope:

- token persistence or OS keychain storage;
- password reset, registration or account management;
- server TLS/private-CA configuration;
- browser login.

## Evidence

- `spec/spec-039` — sessions and login route.
- `spec/spec-044` — remote transport and `--server`/`--token`.
- `src/ui/dialogs/settings.rs` and `settings_pages.rs` — settings pages.
- `src/ui/apply.rs` — command application.
- `src/remote.rs` — `RemoteApi` and `login`.

## Acceptance criteria

1. **Given** Settings, **when** the Server page is opened, **then** it shows
   server URL, email, password, Connect/Disconnect and status.
2. **Given** valid credentials, **when** Connect is clicked, **then** the app
   logs in, builds a remote client, loads the workspace and reports connected.
3. **Given** invalid credentials, **when** Connect is clicked, **then** the
   error is shown and the local workspace remains active.
4. **Given** a connected remote workspace, **when** Disconnect is clicked,
   **then** the local workspace is reloaded and the app reports local mode.
5. **Given** `--server` without `--token`, **when** the desktop starts, **then**
   the Server settings page opens with the URL prefilled.
6. **Given** `--token` without `--server`, **when** the desktop starts, **then**
   it fails with a clear error.
7. **Given** a successful login, **when** the password field is cleared,
   **then** the password is not retained in app state.
8. **Given** the existing suite, **when** it runs, **then** all tests stay green
   and the new login test passes.

## Test scenarios (written before implementation)

- `tests/remote.rs`: login success and wrong-password failure.
- `tests/cli.rs`: `--server` and `--token` parsing.
- Existing local/remote tests stay green.

## Design

```text
Settings → Server
  URL / email / password -> Command::SetServer*
  Connect -> remote::login -> SessionResponse.token
          -> app::build_remote_desktop -> replace_client
  Disconnect -> app::build_desktop -> replace_client
```

- `remote::login` posts to `/api/v2/auth/login` and parses the session envelope.
- `NotionApp` holds the form fields and status; `ViewState` exposes them.
- `replace_client` swaps the transport, workspace, tabs/history, pending edits,
  CRDT session and realtime receiver.
- The password is cleared after a successful connect and is never logged.
- `main` opens the Server page when `--server` is present without a token.

## Plan / tasks

- [x] Spec + failing tests (red).
- [x] `remote::login`.
- [x] App/ViewState/Command fields and settings page.
- [x] Connect/disconnect transport swap.
- [x] Startup prompt and CLI validation.
- [x] Tests and docs.
- [x] Build/test/clippy/fmt/docs/screenshot, commit.

## Result

- Added `remote::login` (v2 login → `SessionResponse`).
- Added Settings → Server with URL, email, masked password, Connect,
  Disconnect and status.
- `ConnectServer` logs in, builds `RemoteApi`, swaps `DesktopApi` and reloads
  workspace-derived state; `DisconnectServer` returns to local mode.
- `--server` without `--token` opens the Server page prefilled; `--token`
  without `--server` errors.
- The password is cleared after a successful connect.
- Tests: remote login success/failure plus existing remote/CLI coverage; the
  suite is 252 tests.
- Added `docs/adr-014-server-login-settings-ui.md`.

## Requirement mapping

| Requirement | How |
| ----------- | --- |
| FR (login UI) | Settings → Server fields and Connect |
| FR (remote switch) | `replace_client` local/remote |
| NFR (security) | password cleared, not stored/logged |
| NFR (compatibility) | local mode remains default |
