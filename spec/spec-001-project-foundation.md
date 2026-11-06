---
title: Project foundation, documentation, and build skeleton
version: 1.0
date_created: 2026-11-05
tags: foundation, sdd, tdd
status: implemented
---

# Spec 001 — Project Foundation

## 1. Purpose & Scope

Establish the rules of the project before any feature: `AGENTS.md`, the
requirements document, the data-model document, a Cargo project that builds,
and a repeatable TDD command. This spec mirrors the workflow described in the
request (`AGENTS.md` → requirements → specs → implementation → changelog).

In scope: repository layout, toolchain, docs, Cargo manifest, empty module
files. Out of scope: any behavior.

## 2. Acceptance Criteria

- **AC-001** — Given a fresh checkout, when `cargo build` runs, then it exits
  successfully with no manual steps.
- **AC-002** — Given the docs folder, when it is read, then `AGENTS.md`,
  `docs/requirements.md`, `docs/data-model.md`, `spec/`, and `CHANGELOG.md`
  exist and are written in English.
- **AC-003** — Given any new feature, when work begins, then a numbered spec
  with Given/When/Then criteria and test scenarios exists first.
- **AC-004** — Given `cargo test`, when it runs, then it reports the test suite
  for every public module.

## 3. Test Scenarios

- TS-001: `cargo build` succeeds (build check, no unit test needed).
- TS-002: `cargo test` discovers at least one test in each of `model`,
  `store`, `server`, `client`, `screenshot` (added in later specs).
- TS-003: docs existence is checked by the builder at review time.

## 4. Plan & Tasks

1. Write `AGENTS.md` (architecture, stack, SDD + TDD loop, API contract).
2. Write `docs/requirements.md` and `docs/data-model.md`.
3. Create `Cargo.toml` with pinned dependency versions.
4. Create `src/lib.rs` and `src/main.rs` skeletons; confirm `cargo build`.
5. Record the red step in `CHANGELOG.md`.

## 5. Notes

The example project given by the person (`tcg-matchmaker`) is the reference
for document structure only; no code is borrowed from it.
