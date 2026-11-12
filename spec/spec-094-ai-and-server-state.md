# spec-094 - AiState and ServerState grouping

- Status: implemented (2026-11-12)
- Related: `spec-088`, `spec-092`, `spec-093`, `spec-045`, `spec-057`
- Review sources: Claude and ChatGPT (composable app state, narrow view models)

## Purpose and scope

The AI assistant and server-login fields were still flat on `NotionApp` and
repeated in `ViewState`. This spec groups them into runtime and presentation
values:

- `AiState`: config, probe status, status slot, query, answer, error, busy,
  start instant and answer slot;
- `ServerState`: URL, email, password, status, connected, busy and result slot;
- `AiView` and `ServerView`: the presentation fields;
- `ViewState` holds `ai: AiView` and `server: ServerView`.

Out of scope:

- changing the Ollama probe, answer polling or login flow;
- grouping settings preferences;
- narrowing `ViewState`.

## Acceptance criteria

P1. **Given** the AI settings page, **when** it opens, **then** the endpoint,
model, preference, status and answer are unchanged.

P2. **Given** a login attempt, **when** it succeeds or fails, **then** the URL,
email, status and busy indicator are unchanged.

P3. **Given** the AI and server renderers, **when** they run, **then** they read
`view.ai.*` and `view.server.*`.

P4. **Given** the existing UI/screenshot tests, **when** the fields move,
**then** they pass unchanged.

## Test scenarios

- `tests/ui.rs`
- `tests/screenshot.rs`
- `tests/fidelity.rs`
- `tests/remote.rs`
- Full quality gate

## Design

- Public fields, matching `SearchState`/`SharingState`/`CollaborationState`.
- No behavior moves: only field paths and the two view structs change.
- The runtime/view split mirrors the earlier state slices.

## Plan / tasks

- [x] Add `AiState` and `ServerState`
- [x] Add `AiView` and `ServerView`
- [x] Update the field references
- [x] Run the full gate and commit
