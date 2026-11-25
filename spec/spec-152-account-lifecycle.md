# spec-152 - Account registration, password change and reset

- Status: implemented (2026-11-25)
- Related: `spec-039`, `spec-073`, `spec-145`, `spec-148`
- Review source: deferred "Password reset / registration / account UI"

## Purpose and scope

Accounts existed but only the seeded owner could sign in; there was no way to
claim an invited member, change a password or recover one without editing the
database.

### Registration (`POST /api/v2/auth/register`)

- Public route (same login rate limit as `/auth/login`).
- The email must belong to an existing workspace member (invite-first
  registration). Unknown emails get 403 without revealing whether an account
  exists.
- An existing account returns 409.
- The password needs at least 8 characters. Success creates the Argon2id hash,
  stores the account and returns a session (auto-login).
- Disabled for the local `NoAuthStore` demo server.

### Change password (`POST /api/v2/auth/password`)

- Authenticated. Requires the current password.
- Hashes the new password, revokes **all** sessions for that member, clears the
  in-memory session/ticket caches and returns a fresh session token so the
  caller stays signed in.

### Owner reset (`POST /api/v2/auth/reset`)

- Owner-only (`UpdateContent`, `owner_only` in the route table).
- Resets the password for a member email, revokes every session for that member
  and returns `{ "status": "password_reset", "member_id": ... }`.

### Store and UI

- `AuthStore::delete_member_sessions` is implemented by the memory, SQLite and
  PostgreSQL adapters.
- Desktop Settings → Server has a register checkbox on the login form, a Change
  password form and an owner Reset member form.
- The browser UI serves the same three flows (spec-153).

Out of scope: email delivery, self-service reset tokens, email verification and
administrator-created passwords shown once.

## Acceptance criteria

P1. **Given** an invited member with no account, **then** register returns a
session and login with the new password works.

P2. **Given** an already-registered email, **then** register returns 409.

P3. **Given** a wrong current password, **then** change returns 401.

P4. **Given** a successful change, **then** all previous tokens stop resolving
and the returned token works.

P5. **Given** an owner reset, **then** the member's sessions are revoked and the
new password works.

P6. **Given** the local demo server, **then** registration is refused.

## Test scenarios

- `tests/auth.rs`
  (`invited_member_can_register_change_and_reset_password`, existing login and
  SQLite restart suites).
- `tests/web.rs` browser app shell.
- Desktop Settings → Server rendering (screenshot/manual).

## Plan / tasks

- [x] Extend `AuthStore` with member-session deletion.
- [x] Add register/change/reset routes and validation.
- [x] Revoke sessions and clear the auth caches.
- [x] Add desktop forms and browser forms.
- [x] Add the lifecycle test.
