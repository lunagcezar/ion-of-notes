# spec-027 — Onboarding and templates

## Purpose and scope

Add the first-run “What is this space for?” onboarding and a starter template
library. Both are pure-data driven and persist the chosen purpose.

Scope:

- `WorkspaceSettings` gains `onboarding_completed` and `workspace_purpose`;
- a starter template catalog (`work-project`, `personal-notes`,
  `school-course`, `meeting-notes`, `blank`);
- an onboarding modal with For work / For personal life / For school and
  Skip for now;
- a template library window reachable from Library → More → Templates and
  Settings → General → Get started;
- applying a template creates a real page with blocks and optional table rows;
- preview hooks `NOT_NOTION_SHOW=onboarding|templates`.

Out of scope:

- custom user-defined templates;
- template search/categories;
- automatic first-run onboarding for existing non-empty workspaces (it opens
  only for an empty, not-yet-onboarded workspace or the preview hook);
- syncing the onboarding choice to a remote analytics service.

## Evidence

- `screenshots/picture-69.png`: “What is this space for?” with For work /
  For personal life / For school.
- `screenshots/picture-44.png` … `picture-50.png`: template library surfaces.
- `screenshots/picture-70.png`: General settings with a landing-page selector.
- `src/model/settings.rs`, `src/ui/dialogs/settings_pages.rs`.

## Acceptance criteria

1. **Given** an empty workspace that has not completed onboarding, **when** the
   app starts, **then** the onboarding modal is shown; the seeded demo
   workspace does not show it.
2. **Given** a purpose choice, **when** it is clicked, **then**
   `onboarding_completed` and `workspace_purpose` persist and the matching
   starter template is applied.
3. **Given** Skip or the close control, **when** it is clicked, **then**
   onboarding is marked complete without adding a page.
4. **Given** the template library, **when** a template is used, **then** a page
   with that template's title, blocks and rows is added and opened.
5. **Given** the template catalog, **when** tests run, **then** ids are unique,
   pages build with stable ids, and purpose mapping is correct.
6. **Given** `NOT_NOTION_SHOW=onboarding|templates`, **when** the screenshot
   runs, **then** the corresponding modal is rendered.

## Test scenarios (written before implementation)

- `tests/templates.rs`:
  - unique/non-empty template ids;
  - `to_page` builds blocks/rows with stable ids;
  - onboarding purposes map to the expected templates.
- `tests/model.rs`: onboarding fields survive the `WorkspaceSettings` JSON
  round-trip.
- Onboarding and template screenshots are checked manually.

## Design

- `src/templates.rs`: `Template`, `catalog`, `find`, `template_for_purpose`,
  and `Template::to_page`.
- `WorkspaceSettings` gains serde-defaulted onboarding fields.
- `Command` gains `OpenOnboarding`, `CloseOnboarding`,
  `CompleteOnboarding(purpose)`, `OpenTemplates`, `CloseTemplates` and
  `ApplyTemplate(id)`.
- `apply_template` adds a page through `Workspace::add_page`, opens it in the
  active tab and lets the normal full-snapshot persistence path save it.
- `ui/dialogs/onboarding.rs` and `ui/dialogs/templates.rs` render the modals;
  Library → More → Templates opens the library.

## Plan / tasks

- [x] Spec + failing tests (red)
- [x] `templates.rs` catalog and page builder
- [x] Onboarding settings fields, commands and modal
- [x] Template library window and Library/Settings entry points
- [x] Docs, changelog, build/test/clippy/fmt/screenshots, commit
