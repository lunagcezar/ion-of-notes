# ADR-015 - Windows DACL for the remembered session token

- **Status:** Accepted (spec-127 implemented)
- **Date:** 2026-11-24
- **Supersedes:** none; completes the spec-073/spec-116 session-file hardening
- **Related:** `spec/spec-127-windows-session-acl.md`,
  `docs/security-hardening.md`

## Context

The remote desktop stores `remote-session.json` with a session token. On Unix
the adapter opens the file with mode `0600` and also tightens an existing file
before rewriting it. On Windows the code fell back to `std::fs::write`, which
only inherits the profile directory ACL. The 2026-11-12 and 2026-11-24 reviews
kept this as a deferred security item.

`std` does not expose a Windows DACL API. The alternatives were:

1. spawn `icacls` (external process, localized output, fragile);
2. use the OS credential store (a larger UX/vault feature);
3. call the Win32 security APIs directly through `windows-sys`.

## Decision

Use `windows-sys` (Microsoft's maintained raw bindings) as a Windows-only
target dependency and build a one-ACE DACL for the current user:

- `OpenProcessToken` + `GetTokenInformation(TokenUser)` obtain the current SID;
- `InitializeAcl` / `AddAccessAllowedAce` grant that SID full control;
- `InitializeSecurityDescriptor` / `SetSecurityDescriptorDacl` build the
  descriptor;
- `SetNamedSecurityInfoW` applies it with `DACL_SECURITY_INFORMATION`, removing
  inherited entries.

The file is created empty, hardened, and only then receives the token. A
failure aborts the save; it never falls back to a permissive file.

## Consequences

- The Windows token file has an explicit owner-only DACL.
- One target-specific dependency (`windows-sys`, already present transitively
  through the TLS/HTTP stack) becomes direct.
- The unsafe FFI is isolated in `src/remote/windows_acl.rs`; no other module
  uses Win32 handles.
- A future move to the OS credential store can replace this adapter behind the
  same `write_private_file` call site.
