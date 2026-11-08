# ADR-004 — Code modularization: DDD, subfiles, and better alternatives

- Status: Accepted (2026-11-06)
- Supersedes: nothing; refines `docs/adr-002-architecture-guidelines.md`
- Related: `spec/spec-013-interaction-polish-and-modularity.md`, `AGENTS.md`
  ("Improved architecture guidelines", "Modules, visibility and
  encapsulation")

## Context

The person pointed at the length of the `src` files and asked whether it is
better to adopt DDD and divide everything into subfiles, and asked for better
alternatives. They also supplied the tree of the `tcg-matchmaker` worker:

```
auth/{index,repository,router,service}.ts
events/{index,repository,router,service}.ts
stores/{index,repository,router,service}.ts
moderation/, notifications/, tcgs/, tournaments/, geocoding/
db/, lib/, middleware/, types/, test-utils/
```

That tree is **feature-first modularization**: one folder per feature
(`auth`, `events`, `stores`, …), and inside each feature the **layers that the
feature actually needs** (`repository` = persistence, `router` = transport,
`service` = use case, `index` = facade). `middleware/`, `lib/`, `db/` and
`test-utils/` are cross-cutting concerns. It is not DDD: there is no
`domain/`, no aggregates/value objects, no bounded-context language. It is a
pragmatic *vertical slice* / modular-monolith layout, which is precisely the
better alternative ADR-002 already recommends as the growth path.

Two decisions must not be conflated (ADR-002): **code modularization** (how
files and modules are divided) and **business architecture** (how rules,
entities and use cases are represented). File length is a symptom, not a
rule.

References consulted:

- The Rust Programming Language, ch. 7 "Managing Growing Projects with
  Packages, Crates, and Modules" — modules control **visibility and
  encapsulation**; group related functionality and separate distinct
  features, expose a small public interface and keep the rest private. It
  describes modules, not "one struct per file".
- The Rust Programming Language, ch. 14 (Cargo workspaces) — split into
  multiple crates only when packages need to evolve/version independently.
- [Rust API Guidelines](https://github.com/rust-lang/api-guidelines) — small
  documented public interfaces, traits at boundaries that need abstraction.
- `docs/adr-002-architecture-guidelines.md` — pragmatic Modular Monolith,
  selective DDD, `pub(crate)`, `pub use` facade, feature-based growth path.

## Alternatives considered

| Alternative | Verdict | Why |
| ----------- | ------- | --- |
| **Strict DDD layering everywhere** (`domain/ application/ infrastructure/` per feature, aggregates, repositories for every noun) | Reject for now | DDD is chosen by **domain complexity** (real invariants, several bounded contexts, a rich vocabulary), not by file length. This crate has one bounded context (the workspace) and serde-friendly data shapes; invariants are enforced at the `server` boundary. Adding layers now would be speculative abstraction and empty folders. |
| **Divide by a line-count threshold** (e.g. cap every file at 300 lines) | Reject | Produces `invoice_id.rs`-style fragmentation. A cohesive 500-line file is better than five artificially separated 100-line files that must import each other's private details. |
| **One struct / class per file** | Reject | OOP habit, not Rust. Rust Book ch. 7 says a module is a unit of visibility and encapsulation; small tightly-coupled types belong together. |
| **Split into multiple crates / a Cargo workspace now** | Defer | Only justified by independent versioning/build (Rust Book ch. 14). The crate is one executable with one bounded context. |
| **Feature-based vertical slices** (`src/modules/<feature>/` with only the layers each feature needs) | Accept **as the growth path** | This is what the `tcg-matchmaker` worker does well. Keep today's layered single-crate layout while there is one bounded context; migrate one feature at a time when a second clearly separate context appears. |
| **Pragmatic Modular Monolith + Clean Architecture + selective DDD** (ADR-002) | **Accept as the standing style** | Low coupling, high cohesion, explicit dependencies, testability. Ports (`WorkspaceApi`, `WorkspaceStore`) only at real boundaries; domain independent of egui/eframe/JSON. |
| **Split a file only when it holds more than one coherent responsibility** | **Accept as the operative rule** | The actionable answer to "pay attention to the length": measure *responsibilities*, not lines. |

## Decision

1. Keep the pragmatic Modular Monolith and the layer/dependency direction of
   ADR-002. No `domain/`, `application/`, `infrastructure/` scaffold, no
   aggregate-per-noun, no new crate.
2. Do **not** adopt DDD as a blanket file-splitting strategy. Use selective
   DDD (private-field value objects, validated constructors, per-context
   errors) only when a feature grows real invariants.
3. Split by responsibility. This session applied that rule instead of
   pre-emptive scaffolding:
   - `ui/dialogs.rs` had two unrelated floating windows plus the settings,
     share, inbox, shortcut-sheet and slash-menu code. The **shared
     search / new-tab window** was extracted into `ui/switcher.rs` (one
     cohesive unit, and DRY: both states now use one renderer).
   - **Page-icon customization** (catalog, picker dialog, and the single
     renderer used in header/sidebar/breadcrumbs/tabs/preview) was extracted
     into `ui/page_icon.rs`, with `icons::draw_page_icon` as the one place
     that paints a page icon.
   - `ui.rs` stays a small facade of private modules + `pub use`.
4. When a second bounded context appears, move one feature at a time into
   `src/modules/<feature>/` following the `tcg-matchmaker` shape (facade +
   only the layers it needs), keeping `ui`, `app`, `shared` outside.

## Consequences

- At the time of this decision, `src/ui` had seven cohesive modules instead of
  five. Later refactors applied the same rule: `spec-017` split the three large
  UI files into `commands`, `view`, `input`, `apply`, `render/*` and
  `dialogs/*`; `spec-018` split `model`, `collab` and `server`. The current
  module map is in `AGENTS.md` and `docs/architecture.md` §3 — do not use the
  count in this historical paragraph as a current fact.
- No new dependency, no new crate, no behavior change to the API contract.
- The `tcg-matchmaker` tree is treated as a **reference for the future
  feature-based layout**, not as a template to copy into a single-context
  desktop app today.
- Visual fidelity work (spec-013) and this decision are committed together.

## Follow-up (2026-11-08)

The operative rule still holds: **split by responsibility, not by line count**.
The largest cohesive files after the later refactors are `ui/apply.rs` (one
command-application switch), `ui/dialogs/settings_pages.rs` (the settings
pages) and `ui/render/content.rs` (the page column). They are left alone until
a genuinely separate responsibility appears; see `docs/architecture.md` §10
for the current constraints.
