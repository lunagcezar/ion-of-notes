# ADR-005 — Clean-room server observation

- **Status:** Accepted; B1/B2 complete, B3 blocked, B4 reviewed, B5 pending
- **Date:** 2026-11-07
- **Last review:** 2026-11-08 (spec-029)
- **Supersedes:** none
- **Related:** `AGENTS.md` (clean-room boundary), `docs/adr-003-self-hosted-collaborative-api.md`,
  `docs/clean-room.md`, `spec/spec-016-architecture-follow-up-and-observation.md`

## Context

The application is a clean-room reconstruction. The project rule is strict:
nothing inside the original program, its files, or its folder is read, listed,
run, or decompiled. The client/server direction is now decided (ADR-003), and
the next question is how to learn enough about server-side behavior to build a
compatible self-hosted service without crossing that boundary.

The specific proposal was to use **Wireshark** to inspect the original
service's traffic. That raises three separate problems:

1. **Technical:** modern traffic is TLS-protected. Wireshark alone shows
   encrypted records. Decrypting requires controlling the client (for example
   `SSLKEYLOGFILE`) or installing a trusted interception proxy.
2. **Clean-room:** a packet capture of a proprietary service is much closer to
   reading the original implementation than to observing documented behavior.
   It can reveal internal identifiers, undocumented endpoints and payload
   shapes that were never intended as a public contract.
3. **Legal / terms:** capturing or intercepting a third-party service may
   violate its terms, local law, or both, especially if authentication or
   protection mechanisms are bypassed.

At the same time, the project needs a defensible way to discover enough
behavior to make the client and a self-hosted server compatible.

## Decision

1. **Do not use Wireshark as the primary discovery tool.**
   The primary sources are, in order:
   - public product/API documentation and help pages;
   - behavior visible in the UI and in exports the person already owns;
   - requests and responses from the person's **own account**, captured with
     browser DevTools / HAR while using the product normally;
   - a local mock server built from the derived contract.

2. **Derive a written contract before implementing.**
   Observed endpoints, methods, status codes, envelopes, pagination, sync
   cursors and error shapes are recorded in `docs/api-observations.md`. Each
   entry is marked **observed** or **inferred**. Inferences are never presented
   as facts about the original.

3. **Use interception tools only on traffic we control.**
   `mitmproxy`, Charles or Wireshark may be used to debug our own client, our
   own loopback server, or a self-hosted instance we own and are authorized to
   test. They are not used to bypass authentication, certificate pinning or
   access controls.

4. **Redact before storing.**
   HAR files and fixtures must contain no session cookies, bearer tokens,
   email addresses, workspace names or other personal data. A capture is
   reviewed against a redaction checklist before it is committed.

5. **Keep a clean-room log.**
   `docs/reverse-engineering-log.md` records what was observed, from which
   public or own-account source, what was inferred, and what remains unknown.
   This is the audit trail that shows the implementation was not copied.

6. **Stop conditions.**
   If a capture would require bypassing a protection, violating terms, or
   exposing proprietary source/artifacts, stop. Fall back to public behavior,
   documented APIs and a deliberately compatible mock contract.

## Phase status (2026-11-08)

| Phase | Status |
| ----- | ------ |
| **B1** permission/scope gate | **Done:** public behavior only, strict clean room |
| **B2** behavioral inventory | **Done:** `docs/api-observations.md` + `docs/visual-reference-inventory.md` |
| **B3** own-account HAR | **Blocked** by the B1 decision; needs a new explicit scope decision |
| **B4** contract derivation | **In progress:** public Notion developer docs reviewed 2026-11-08; specs 030–037 drafted |
| **B5** mock server + contract tests | **Pending:** follows B4 |
| **B6** tooling policy | **Recorded** in this ADR |
| **B7** clean-room log | **Ongoing:** `docs/reverse-engineering-log.md` |
| **B8** stop conditions | **Reviewed** before every source |

The operating manual is `docs/clean-room.md`. No capture, proxy, source map or
decompilation has been used; B3 is intentionally not executed under the
current scope.

## Consequences

Positive:

- The work remains defensible as clean-room reconstruction.
- The derived contract is reusable by the client, the mock server and the
  future self-hosted server.
- Contract tests make compatibility explicit instead of accidental.
- No secrets or personal data end up in the repository.

Negative:

- Discovery is slower than capturing a packet stream and reading it.
- Some internal behavior will remain unknown; the implementation may be
  behaviorally compatible without being byte-for-byte identical.
- Public/own-account observations may be incomplete, so the contract needs
  versioning and tests.

## Alternatives considered

| Alternative | Why rejected |
| ----------- | ------------ |
| Wireshark as the main tool | TLS encryption, clean-room risk, terms/legal risk, and captures reveal implementation details rather than a public contract. |
| Decompiling the client or server | Explicitly forbidden by the clean-room rule. |
| Copying captured payloads directly | Mixes proprietary internals into the project and undermines the clean-room claim. |
| Public documentation only | Useful but incomplete; it does not cover every route, status or edge case. |
| Blind UI imitation | Produces a look-alike without a testable server contract. |

## References

- Clean-room boundary: `AGENTS.md` (R-00, NFR-07).
- Decided API: `docs/adr-003-self-hosted-collaborative-api.md`.
- Modularization decision: `docs/adr-004-modularization-decision.md`.
- Wireshark TLS documentation: <https://wiki.wireshark.org/TLS>
- Browser HAR format: <https://w3c.github.io/web-performance/specs/HAR/Overview.html>
