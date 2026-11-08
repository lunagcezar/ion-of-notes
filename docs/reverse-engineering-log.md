# Reverse-engineering log — not-notion

This is the clean-room audit trail. It records what was observed, from which
public or authorized source, what was inferred, and what remains unknown. It
does not contain captured traffic, secrets or proprietary artifacts.

## 2026-11-07 — B1/B2: scope gate and first behavioral inventory

### B1 — permission and scope gate

- **Decision:** public behavior only — **strict clean room**.
- **Authorized sources:**
  - the 98 reference pictures in `reference/` (the original 29 plus the later
    build) and the 72 local `screenshots/` pictures;
  - behavior visible in the product UI;
  - public help/documentation pages;
  - exports the person already owns;
  - the project's own decided contract in ADR-003.
- **Not authorized:**
  - HAR or packet capture;
  - `mitmproxy`, Charles, Wireshark or any TLS interception;
  - browser source maps, bundled JavaScript or decompilation;
  - bypassing authentication, certificate pinning, rate limits or terms;
  - copying proprietary payloads, identifiers or assets into the project.
- **Capture status:** none. No traffic was recorded or inspected.
- **Account/instance scope:** not applicable; no capture is authorized under
  this decision.
- **Redaction:** not applicable (no captures). If the scope ever changes to
  own-account HAR, redaction is mandatory before storage.

### B2 — behavioral inventory

- **Output:** `docs/api-observations.md` and
  `docs/visual-reference-inventory.md`.
- **Sources used:** the 98 `reference/` pictures, the 72 `screenshots/`
  pictures, `docs/requirements.md`,
  `docs/adr-003-self-hosted-collaborative-api.md`, `docs/data-model.md`,
  `spec-011` and `spec-015`.
- **Visual note:** the `screenshots/` set shows a later product build with the
  new-tab switcher, Add members, richer page actions, Library tabs, onboarding,
  Settings General/Sidebar/People/Language/Offline, AI/agent surfaces and
  slash/icon pickers. It is recorded as UI-observed only; no Portuguese text,
  branding or assets were copied.
- **Result:** every operation is marked **UI-observed**, **Inferred**,
  **Decided** or **Unknown**. No original server response was observed, and no
  entry is presented as a fact about the original implementation.
- **Key limitation:** exact routes, auth flow, pagination, sync semantics,
  block update granularity, trash behavior, permission inheritance, export
  metadata, real-time framing and error taxonomy remain **Unknown** until
  public documentation answers them.

### Stop conditions

- If a question can only be answered by capturing traffic, stop and ask the
  person for a new B1 decision.
- If public documentation reveals a behavior that differs from ADR-003, update
  the observation file and the relevant ADR/spec before changing code.
- If a source would require bypassing a protection or violates terms, do not
  use it.

### Next steps under this scope

- Review the public documentation links listed in `docs/api-observations.md`
  and mark entries **Observed (public docs)** where confirmed.
- Update ADR-003/spec-016 if public documentation contradicts a decided
  behavior.
- B3 (own-account HAR) is intentionally **not executed** under the strict
  scope. It can only resume after a new B1 decision.

## 2026-11-08 — documentation review and B-phase status

- **Source:** the repository itself (code, tests, Markdown) plus the already
  authorized 98 `reference/` pictures and 72 `screenshots/` pictures.
- **Observed:** no new server behavior. The review checked documents against
  the code and recorded drift in `docs/review-findings.md`.
- **Inferred:** none added. No capture, proxy, source map or decompilation was
  used.
- **Rejected:** nothing was rejected; no new source was proposed.
- **Redaction:** not applicable (no capture).
- **Phase status:** B1 done, B2 done, B3 **blocked**, B4/B5 **pending** on the
  public API links, B6 recorded, B7 ongoing, B8 reviewed.
- **Documentation changes:** added `docs/index.md`, `docs/architecture.md`,
  `docs/contributor-guide.md`, `docs/testing.md`, `docs/glossary.md`,
  `docs/review-findings.md`, `docs/api-contract.md`, `docs/clean-room.md`,
  `spec/README.md` and `spec/spec-029-*`; corrected stale statuses in the
  requirements, ADRs, specs, unknowns, visual inventory and this log.
- **Verification:** 115 tests green; clippy and `cargo fmt --check` clean.
