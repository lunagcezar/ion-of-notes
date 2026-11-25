# spec-127 - Windows session-file DACL

- Status: implemented (2026-11-24)
- Related: `spec-044`, `spec-073`, `spec-116`, `docs/adr-015-windows-session-acl.md`
- Review source: deferred Windows ACL item in `docs/review-response.md`

## Purpose and scope

On Unix the remembered remote session is written with mode `0600`, including a
`set_permissions` pass for an existing file. On Windows the adapter used
`std::fs::write`, so the token inherited the containing directory's ACL.

This spec adds a Windows DACL adapter:

- `remote::windows_acl::restrict_to_current_user` obtains the current user SID,
  builds a one-ACE ACL granting full control, and applies it with
  `SetNamedSecurityInfoW` + `DACL_SECURITY_INFORMATION`.
- `write_private_file` creates the file empty, applies the DACL, and only then
  writes the token. A DACL failure aborts the save.
- The implementation is isolated behind `#[cfg(windows)]`; Unix is unchanged.

Out of scope:

- migrating the token to the Windows Credential Manager;
- ACLs for other application files;
- a cross-platform vault abstraction.

## Acceptance criteria

P1. **Given** Windows, **when** a session is saved, **then** the file exists and
the write succeeds with an owner-only DACL.

P2. **Given** a DACL API failure, **then** the token is not written and the save
returns an error.

P3. **Given** Unix, **then** the existing `0600` behavior is unchanged.

P4. **Given** a non-Windows build, **then** the Windows module is not compiled.

## Test scenarios

- `tests/remote.rs` (`remote_session_roundtrip_and_clear`,
  `saved_session_is_private_on_unix`).
- Windows build + `cargo test --no-default-features --test remote`.
- Full quality gate.

## Design

| Piece | Location |
| ----- | -------- |
| Win32 DACL adapter | `remote/windows_acl.rs` |
| Create → harden → write | `remote/session.rs` |
| Dependency/build rule | `Cargo.toml`, `docs/adr-015-windows-session-acl.md` |

## Plan / tasks

- [x] Add `windows-sys` as a Windows-only dependency.
- [x] Build and apply the current-user DACL.
- [x] Create the file before hardening and write only afterwards.
- [x] Keep Unix behavior unchanged.
- [x] Record the decision in ADR-007.
