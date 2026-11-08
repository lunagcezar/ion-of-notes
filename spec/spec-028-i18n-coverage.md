# spec-028 — Internationalization coverage

## Purpose and scope

`rust-i18n` was wired up, but only two Settings strings used it. This spec adds
a dynamic translation helper and translates the main navigation, Share,
Library, page-action, comment and Settings surfaces into English, Portuguese
(Brazil) and Esperanto.

Scope:

- `i18n::t(key)` for dynamic keys (settings tabs, Library tabs, action labels);
- expanded `en`, `pt-BR` and `eo` key tables with matching key sets;
- translated sidebar, Share, Library, page actions, comments and Settings
  labels/status messages;
- a locale completeness test and a locale-switching test.

Out of scope:

- translating long-form demo document copy;
- persisting the selected language across restarts (the current language
  selector changes the live locale only);
- translating protocol/log/error text that is not user-facing;
- a full audit of every remaining English string. The key table is the source
  of truth and coverage continues incrementally.

## Evidence

- `src/i18n.rs`: `set_locale`, `label_for`, `current_locale`; only two `t!`
  calls existed.
- `locales/en.yml`, `pt-BR.yml`, `eo.yml`: original small key set.
- `screenshots/`: Portuguese UI reference for labels and layout.
- `docs/requirements.md` FR-68 / spec-016 A6.

## Acceptance criteria

1. **Given** a key present in `en.yml`, **when** the locale completeness test
   runs, **then** the same key exists in `pt-BR.yml` and `eo.yml`.
2. **Given** the locale is `en`, `pt-BR` or `eo`, **when** `i18n::t("library")`
   is called, **then** it returns the translated label for that locale.
3. **Given** an unknown locale, **when** it is set, **then** it falls back to
   English.
4. **Given** the sidebar, Share dialog, Library, page actions, comments or
   Settings pages, **when** they render, **then** their labels/status messages
   use the key table rather than hard-coded English.
5. **Given** the default screenshot, **when** it renders, **then** no raw
   translation keys are visible.

## Test scenarios (written before implementation)

- `tests/i18n.rs`:
  - English key set is a subset of both other locales;
  - locale switching returns the expected labels;
  - unknown locale falls back to English.
- Existing tests, clippy, fmt and the default screenshot stay green.

## Design

- `i18n::t` reads the active locale, builds the locale-prefixed key expected by
  `rust-i18n`'s generated backend, and calls `_rust_i18n_translate`.
- The three YAML files use the same flat key names with translated values.
- UI surfaces call `i18n::t("key")`; dynamic tab labels map an English tab key
  to a translated label while the internal state keeps the stable English key.
- Status messages that include counts use `rust_i18n::t!` with `%{count}`.

## Plan / tasks

- [x] Spec + failing tests (red)
- [x] `i18n::t` dynamic helper
- [x] Expanded en / pt-BR / eo key tables
- [x] Sidebar, Share, Library, page actions, comments and Settings labels
- [x] Docs, changelog, build/test/clippy/fmt/screenshot, commit
