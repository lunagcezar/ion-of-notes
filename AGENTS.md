# AGENTS.md — ion-of-notes

## Project Overview

`ion-of-notes` is a clean-room desktop workspace application: pages made of editable
blocks, a page tree sidebar, quick search, and an inline table database. It is
rebuilt from a written request and descriptions only. Nothing inside the original
program, its files, or its folder is read, listed, run, or decompiled.

The system has two sides, as requested:

- **Client** — the desktop window (Rust + eframe/egui) and an API client that
  only talks through a documented request/response contract.
- **Server** — an in-process workspace service with a REST-shaped API, plus an
  optional Axum HTTP listener (`--serve`) that exposes the very same API.
  All state lives on the server; the client never mutates server state directly.
  The decided target is a **self-hosted** server with **accounts, document
  synchronization, sharing and real-time editing**; v1 keeps the same contract
  in-process behind the seam (see `docs/adr-003-self-hosted-collaborative-api.md`).

All human-readable documentation and all source code are written in **English**.
The working name of the application is **ion-of-notes**.

The project is **open source** (MIT, `LICENSE`). That is a license fact, not a
marketing feature: the person asked that "free & open for all" **not** be
emphasized in the application. The window therefore shows no free-edition card
or plan/feature-tier list. Because the product is now a self-hosted
client/server system **with accounts**, the earlier "no accounts" framing
(`spec-007`) is retired. The person also repeatedly emphasizes **DRY, Clean
Code, SOLID and modularization**: the standing guidelines are collected below
and in `docs/adr-002-architecture-guidelines.md`.

## Tech Stack

| Layer            | Technology                          | Notes                                              |
| ---------------- | ----------------------------------- | -------------------------------------------------- |
| Language         | Rust (stable, edition 2024)         | `cargo build`, `cargo test`                        |
| Window / UI      | eframe 0.37 + egui 0.37             | Immediate-mode UI; behind the default-on `desktop` feature |
| Data model       | `serde` + `serde_json`              | JSON documents; SQLite document store for `--database` |
| PNG output       | `image` (png feature only)          | `--screenshot`; behind the default-on `desktop` feature |
| Display fonts    | Segoe UI (Windows system fonts)     | Optional bold "Strong" family; egui built-ins are the fallback |
| Server transport | Axum 0.8 + Tokio + Tower/Tower-HTTP | Optional HTTP server for `--serve` (spec-038)        |
| Persistence      | JSON files + `rusqlite` (bundled)   | `SqliteStore` behind the existing store ports (spec-039) |
| Auth             | Argon2id + opaque sessions          | Hashed passwords and session tokens (spec-039)      |
| Real-time        | Axum WebSocket + Tokio broadcast    | Change feed, presence and resume (spec-040)          |
| Webhooks         | ureq + rustls, no redirects         | Signed delivery behind a `WebhookSender` port (spec-041) |
| Shared limits    | SQLite `rate_limits` + `RateLimitStore` | Multi-server workspace budgets (spec-041)        |
| CRDT             | Yrs per-page text documents         | Server merge (042) + client session/awareness (043) |
| Remote desktop   | ureq REST + tokio-tungstenite WS    | `--server`/`--token` remote mode (044)               |
| Deployment       | Docker multi-stage + Compose        | `ion-of-notes-server`, `ion-data` volume (spec-050) |
| IDs / time       | `uuid` (v4), `chrono`               | Stable page/block ids and display timestamps        |
| Tests            | `cargo test` (integration crates)   | One file per module in `tests/`, no external framework |

Dependencies are kept deliberately small. No web view and no browser engine.
The only runtime network access is the optional loopback API listener and the
optional Ollama AI service (`OLLAMA_HOST`, default `127.0.0.1:11434`).

## Development Methodology

### Spec-Driven Development (SDD)

Every feature starts with a spec. The order of work is:

1. `AGENTS.md` — architecture, technologies, conventions (this file).
2. `docs/requirements.md` — functional and non-functional requirements.
3. `docs/data-model.md` — entities and relationships.
4. `spec/spec-NNN-purpose.md` — one numbered spec per feature, with:
   - purpose and scope,
   - evidence used (what the reference pictures show),
   - acceptance criteria in **Given / When / Then** form,
   - test scenarios listed **before** implementation,
   - a short plan and task list.
5. Implementation following TDD (below).
6. `CHANGELOG.md` — updated after each implemented spec.
7. `AGENTS.md` and `docs/requirements.md` — revisited whenever the foundations
   change.

Specs live in `spec/` and are numbered sequentially (`001`, `002`, ... `009`).
A spec is the boundary of one working session: nothing outside its scope is
built in that session. Reference example for this workflow: `tcg-matchmaker`.
The visual evidence is the **98 pictures** in the workspace `reference/`
folder (`picture-1`…`picture-29` are the original set; `30`…`98` are the later
build) plus the **72 local `screenshots/` pictures**. `reference/` is the
primary visual source of truth; every spec that touches the window records
what was observed there. Historical specs may say "29" because that was the
set at the time.

### Test-Driven Development (TDD)

- **No production code without a failing test first.**
- Red → Green → Refactor for every feature and every bug fix.
- Tests exercise public interfaces (`model`, `store`, `server`, `client`,
  `screenshot`), not private implementation details.
- **Tests never live in implementation files.** Every test is an integration
  test in `tests/`, one file per module, following the Rust Book ch. 11. Shared
  fixtures live in `tests/common/mod.rs` so no helper is duplicated (DRY).
- Tests are deterministic: temporary files use unique names, no wall-clock
  assertions, no running listeners left behind.
- `cargo test` must be green before a spec is marked done in `CHANGELOG.md`.
- The red step is recorded: tests are written, `cargo test` is run and observed
  to fail (or fail to compile), then the implementation is added.

## Architecture

```
src/
  main.rs          CLI wiring only (desktop, --screenshot, --serve)
  bin/
    ion-of-notes-server.rs  server-only entry point (no desktop feature)
  app.rs           composition root (bootstrap): wires workspace + collaboration adapters to ports
  lib.rs           crate root and public re-exports
  model.rs         facade: module declarations, pub re-exports, time helpers
  model/
    rich_text.rs   annotations, mentions, equations, RichText
    block.rs       BlockKind, Block, block DTOs and tree helpers
    property.rs    typed properties, values and option/page refs
    database.rs    Database, DataSource, TableRow
    view.rs        views, filters, sorts and grouped queries
    file.rs        file objects, uploads and attachments
    comment.rs     page comments and comment DTOs
    user.rs        people, users, bots and partial user refs
    page.rs        Page, summaries, drafts and WorkspaceError
    workspace.rs   Workspace tree behavior and deserialization
    seed.rs        neutral demo workspace used on first start
    search.rs      ranked search + excerpt helper
    diff.rs        changed/added/removed/reordered page diff
    index.rs       derived page/children lookup index
    settings.rs    persisted workspace settings
  auth.rs          accounts, Argon2id passwords, opaque sessions, AuthStore port
  store.rs         WorkspaceStore + CollabStore + AuthStore ports, JSON/null adapters
  store/
    sqlite.rs      SQLite adapter for workspace, collaboration and auth (spec-039)
  server.rs        facade: contract DTOs, WorkspaceApi, WorkspaceServer state
                   (Persistence, Runtime, WebhookRuntime), constructors,
                   persistence helpers
  server/
    routes.rs      WorkspaceApi implementation and route facade
    routes/        v1 adapter, v2 pipeline, public route_capability table
    usecases/      pages, workspace and sharing use cases with typed results
    handlers/      one module per v2 resource family (auth, crdt, webhooks,
                   comments/users/files, views, databases, blocks, lists,
                   shared page/workspace/sharing adapter)
    response.rs    Envelope/json/error helpers
    http.rs        legacy loopback HTTP/1.1 adapter (parse/format/tests)
    url.rs         percent-encoding/decoding shared by client and server
    contract.rs    v2 versioning, ApiError, capabilities, request IDs
    pagination.rs  v2 list envelope, opaque cursors, page_size validation
    limits.rs      plan/request budgets, size checks, block-limit policy
    rate_limit.rs  in-process per-connection and workspace rate limiter
    realtime.rs    change-feed hub, protocol messages and presence (spec-040)
    crdt.rs        Yrs per-page text documents and projection (spec-042)
    web.rs         Axum/Tokio HTTP + WebSocket adapter for --serve
    webhook_sender.rs  WebhookSender port and ureq/rustls adapter (spec-041)
  remote.rs        remote WorkspaceApi: ureq REST + background WebSocket
  client.rs        Client<A: WorkspaceApi> — the only UI-facing data access
  collab.rs        accounts/roles, sharing, sync log, realtime DTOs, CollabState
                   (version history capped at 50 snapshots per page)
  collab/
    export.rs      Notion-style markdown/CSV export tree
  ollama.rs        AI assistant adapter: Ollama CLI in PATH or service at host:port
  shortcuts.rs     pure keyboard-shortcut table (Combo -> Action) + docs table
  screenshot.rs    ColorImage -> RGBA -> PNG (desktop feature)
  cli.rs           argument parsing
  i18n.rs          locale switching + dynamic key translation helper
  templates.rs     starter template catalog + page builder
  webhooks.rs      subscriptions, event types, HMAC-SHA256 signatures,
                   aggregation and delivery records
locales/           en.yml, pt-BR.yml, eo.yml key tables
  ui.rs            module root: declarations + public re-exports (facade,
                   desktop feature)
  ui/
    palette.rs     colors, fonts, tab-title helper, theming (NFR-06)
    icons.rs       original shape artwork (chrome, sidebar, book, controls,
                   page icon)
    commands.rs    Command + flush/lock classification, IconTab, FontStyle,
                   NewTabChoice and pure builders
    view.rs        ViewState snapshot passed to render functions
    input.rs       egui key events -> shortcut actions (pure input adapter)
    app.rs         NotionApp struct, new(), eframe::App shell, preview hook
    apply.rs       command application and state transitions
    render.rs      facade for the render submodules
    render/
      chrome.rs    tab strip and window controls
      sidebar.rs   search, Library/Settings/Inbox, favorites, tree, bottom group
      content.rs   breadcrumbs, page header, blocks, floating controls
      library.rs   Library table and tabs
      database.rs  inline table / board database
    dialogs.rs     facade for the dialog submodules
    dialogs/
      settings.rs       settings window, rail, search filtering
      settings_pages.rs Preferences / Workspace / Sidebar / Offline / People / AI
      sharing.rs        Share popover and Inbox
      page_actions.rs   page actions and version history
      menus.rs          app / workspace / grid / slash menus
      help.rs           keyboard shortcut sheet
      search_window.rs  shared search / new-tab windows
      onboarding.rs     “What is this space for?” first-run modal
      templates.rs      starter template library window
    widgets.rs     shared popover frame, menu row, toggle, section label
    edit_queue.rs  pure commit plan + debounce for per-page writes
    invite.rs      pure invite email parsing/validation/name derivation
    library_filter.rs pure Library tab filtering
    switcher.rs    one shared search / new-tab window (DRY)
    page_icon.rs   page-icon feature: catalog, picker and one renderer
    crdt.rs        client Yrs session, local diff and projection (spec-043)
    dialogs/settings.rs  Settings → Server login form (spec-045)
    search_cache.rs pure query/version cache so redraws do not re-query
    settings_search.rs pure settings section/row matcher and row table
tests/
  common/mod.rs    shared test helpers (DRY; not a test target)
  model.rs         one integration-test crate per module (named after it)
  store.rs
  server.rs
  client.rs
  collab.rs
  cli.rs
  screenshot.rs
  ui.rs
  app.rs
  ollama.rs
  shortcuts.rs
  fidelity.rs
  store_port.rs
  collab_store.rs  collaboration persistence and failing-store behavior
  i18n.rs          locale key completeness and fallback
  templates.rs     starter template catalog and page builder
  contract.rs      v2 versioning, errors, capabilities, request IDs
  pagination.rs    v2 cursor pagination and list envelopes
  blocks.rs        rich text, block metadata, children and block endpoints
  properties.rs    property schemas/values, data sources and property items
  views.rs         view CRUD, filters/sorts, board/calendar grouping
  comments.rs      comment CRUD, users, attachments and file uploads
  webhooks.rs      subscriptions, signatures, aggregation, delivery and retry
  limits.rs        rate/size/block limits, Retry-After, 409/503/529
  web.rs           Axum HTTP adapter contract suite
  realtime.rs      WebSocket auth, ready, backlog, live change, presence
  crdt.rs          Yrs convergence, projection and WebSocket CRDT sync
  ui_crdt.rs       client Yrs diff/merge/projection
  remote.rs        remote REST/WebSocket transport and login
  shared_limits.rs SQLite shared rate limits and window reset
  auth.rs          password hashing, login/logout/session, SQLite restart
  sqlite_store.rs  SQLite round-trip, accounts/sessions, JSON migration
  integration.rs   end-to-end client/server/store scenario
Dockerfile         multi-stage server-only image
compose.yml        self-hosted server stack with the ion-data volume
.env.example       Compose password and port template
.dockerignore      build-context exclusions
```

Dependency direction: `ui -> client -> server -> model`, `server -> store`,
and `app` (the composition root) wires the concrete adapters into the core.
`model` and `store` have no UI dependencies. The `WorkspaceApi` trait is the
seam that makes the client testable and allows a future remote client; the
`WorkspaceStore` trait is the seam that makes persistence swappable. The
domain (`model`) knows nothing about egui, eframe, the JSON file or any
framework; adapters implement ports, never the reverse (see the layer table
and dependency direction below).

### Architectural patterns (DRY + SOLID)

| Pattern | Where it is applied |
| ------- | ------------------- |
| Ports and adapters (hexagonal) | `WorkspaceApi` (transport) and `WorkspaceStore` (persistence) are traits declared by the core; the JSON file and the Axum/loopback HTTP listeners are adapters. |
| Single Responsibility | One module per concern; `ui/` splits state, theming, icons, rendering, and dialogs. Each function has one job. |
| Open / Closed | New transports or stores are added as new trait implementations; `WorkspaceServer` is not edited. |
| Liskov Substitution | Any `WorkspaceApi` or `WorkspaceStore` implementation works everywhere the trait is expected (including test stubs). |
| Interface Segregation | Traits are small: `WorkspaceApi::handle`, `WorkspaceStore::persist`. |
| Dependency Inversion | `Client<A: WorkspaceApi>` and `WorkspaceServer` depend on abstractions, not on concrete transports or files. |
| DRY | Shared test helpers in `tests/common/mod.rs`; colors in `ui::Palette`; one JSON envelope helper in `server`. |

References consulted: The Rust Programming Language ch. 7 "Managing Growing
Projects with Packages, Crates, and Modules" (module tree, visibility,
encapsulation), ch. 11 "Test Organization" (integration tests,
`tests/common/mod.rs`), and ch. 12 "Refactoring to Improve Modularity and
Error Handling" (`main.rs` wiring, `lib.rs` logic, one task per function); the
[Rust API Guidelines](https://github.com/rust-lang/api-guidelines) (traits for
flexibility, small documented public interfaces, future-proofing); and the
2026-11-05 architecture review (recorded in
`docs/adr-002-architecture-guidelines.md` and
`spec/spec-010-architecture-guidelines.md`). The evaluation of a suggested
external library list is in `docs/adr-001-stack-evaluation.md`.

### Improved architecture guidelines (review, 2026-11-05)

The person supplied an architecture review for Rust desktop applications, the
Rust API Guidelines, and Rust Book ch. 7. The conventions below are the
**project guidelines** distilled from them and are the standing reference for
this file. DRY, SOLID, Clean Architecture and DDD are **tools, not literal
patterns to implement everywhere**. What actually matters is low coupling,
high cohesion, explicit dependencies, encapsulation, ease of testing and ease
of evolving a feature without breaking others — not the number of files.

#### Two different decisions

1. **Code modularization** — how files, modules, responsibilities and
   dependencies are divided.
2. **Business architecture** — how rules, entities, use cases and processes
   are represented.

DRY and SOLID help but do not settle either on their own. We do not start with
microservices, distributed architecture, or a surplus of abstractions.

#### Chosen style: pragmatic Modular Monolith

- **Modular Monolith**: one executable, organized into modules with clear
  responsibilities (one crate today).
- **Clean Architecture, pragmatic**: business rules independent of database,
  GUI and frameworks.
- **DDD, selective**: entities, value objects and invariants only where the
  domain has real complexity. Being a desktop app is not itself a reason to
  use or avoid DDD, and DDD is not "a class for every noun".
- **SOLID**: above all Single Responsibility and Dependency Inversion.
- **DRY**: avoid duplicating *knowledge and rules*, not any code that merely
  looks similar; do not force a shared abstraction onto similar-looking code.

#### Layers, responsibilities and dependency direction

| Layer | Responsibility | In this crate |
| ----- | -------------- | ------------- |
| domain | business rules, entities, invariants | `model.rs` |
| application | use cases, orchestration, the contracts it needs | `server.rs` (routes / use cases, `WorkspaceApi`), `client.rs` (typed calls) |
| infrastructure | database, filesystem, APIs, technical adapters | `store.rs` (`JsonStore`, `NoStore`, `data_path`), `server.rs` HTTP listener |
| presentation | UI, commands, DTOs, input/output adapters | `ui/`; `PageDraft` / `PageSummary` are the DTO-ish shapes |
| app (bootstrap) | startup, composition root, application state | `app.rs` (`build_server`, `build_desktop`), `main.rs` (CLI wiring only) |

Fundamental rule: **the domain must not know the infrastructure or the GUI.**
`model.rs` knows nothing about SQLite, egui, eframe or Tauri; no egui type
appears in `model`, `store`, `server` or `client`.

Direction of dependencies: `domain <- application <- infrastructure /
presentation`. **The bootstrap connects the concrete implementations to the
contracts the application needs** — adapters implement ports, never the
reverse. Concretely: `ui -> client -> server -> model`, `server -> store`, and
`app` wires `JsonStore` into `WorkspaceServer` and wraps it in `Client`;
neither `main.rs` nor the window assembles that graph itself any more.

#### Desktop presentation is an input adapter

The GUI must never own business rules. A healthy flow is:

```
Frontend -> command (input adapter) -> use case -> domain
                                          |
                                          v
                              repository / store contract
                                          ^
                                          |
                              JSON / SQLite adapter
```

A Tauri command (or an egui / iced handler) only adapts data in and out and
delegates to a use case; it implements no rule. This keeps the app testable
without launching a window. In this crate `ui/app.rs` collects `Command`s and
applies every change through `Client<A: WorkspaceApi>`.

#### Modules, visibility and encapsulation (Rust Book ch. 7)

Modules are not just files: they control **visibility and encapsulation**.

- **Private by default.** Use `pub` only when there is a real need to expose;
  avoid `pub` indiscriminately.
- Use `pub(crate)` for functionality shared inside the crate but not part of
  the public API; keep implementation details private.
- Use `pub use` in a module root to build one clean public facade:

  ```rust
  // domain/project/mod.rs
  mod entity;
  mod errors;
  mod repository;

  pub use entity::Project;
  pub use errors::ProjectError;
  pub(crate) use repository::ProjectRepository;
  ```

- In-repo examples: `ui.rs` declares five private submodules and re-exports
  only `NotionApp` and the palette helpers; `lib.rs` re-exports the crate's
  public facade; inner `Command` / `ViewState` are `pub(super)`.

#### When to create a file or module

A file represents **one coherent unit of responsibility** — not "one struct
per file" or "one class per file".

| Situation | Decision |
| --------- | -------- |
| A file with many responsibilities | split it |
| Related concepts in one module | keep them together |
| A small struct tightly coupled to another | the same file is fine |
| A large infrastructure implementation | give it its own file |
| A module exposing too many internals | revisit encapsulation |

There is no magic line count: one cohesive 400-line file can be better than
four artificially separated 100-line files. Avoid `invoice_id.rs`-style
fragmentation of tiny definitions. Do not create empty
`domain/ application/ infrastructure/` folders for features that do not need
them (e.g. settings that only store a few preferences), and do not create
abstractions in advance — a single concrete implementation is enough until
there is a real need to swap, isolate or reuse it.

#### Pragmatic DDD and domain invariants

DDD is chosen by **domain complexity**: complex rules, invariants, multi-step
processes, a specific vocabulary, several bounded contexts, rules that change
often. A file manager needs simple modularization; a finance or engineering
tool may need pragmatic DDD. In Rust the domain is expressed with `struct`,
`enum`, functions, modules and traits — not class-per-noun OOP. Protect
invariants with private fields and validating constructors returning
`Result`, and give each context its own error type:

```rust
pub struct Project { id: ProjectId, name: String }  // fields private

impl Project {
    pub fn create(id: ProjectId, name: String) -> Result<Self, ProjectError> {
        if name.trim().is_empty() { return Err(ProjectError::InvalidName); }
        Ok(Self { id, name })
    }
    pub fn name(&self) -> &str { &self.name }
}
```

Pragmatic tradeoff in this crate: `model` structs are serde-friendly data
shapes with public fields (needed for JSON persistence and immediate-mode
editing), so invariants are enforced at the boundaries instead — `server`
rejects empty page titles, `Workspace::add_page` rejects duplicate ids and
unknown parents (`WorkspaceError`). If the domain grows real behavior, add
private-field value objects and validated constructors then, not before.

#### Growth path: feature-based modules

The review prefers, for large projects, organizing first by functional
context and using layers **inside** each feature:

```
src/
  shared/            cross-cutting errors / helpers
  modules/
    users/{domain, application, infrastructure}
    projects/{domain, application, infrastructure}
    settings/        (only the layers it actually needs)
  presentation/
  app/               bootstrap / composition root
```

Advantages: each feature owns its code, responsibilities are easy to locate,
modules evolve independently, dependencies are visible, and a module can
later be extracted into its own crate. Decision: keep today's layered,
single-crate layout while there is one bounded context (the workspace); when
a second clearly separate context appears, migrate one feature at a time into
`src/modules/<feature>/`. Split into multiple crates only with a real need for
independent versioning/build, not pre-emptively. Architecture is not measured
by file count.

#### Team conventions (quick reference)

| Principle | Convention |
| --------- | ---------- |
| Modularization | by feature/context as the project grows; a layered modular monolith today |
| Architecture | pragmatic Clean Architecture |
| Domain | independent of frameworks and UI |
| Use cases | one business operation per unit of implementation |
| Persistence | repository/store contracts + adapters |
| Visibility | private by default (`pub(crate)`, `pub use` facade) |
| Traits | only at boundaries that need abstraction |
| Errors | specific types per context |
| DTOs | separate from domain entities |
| Tests | unit tests in the domain, integration tests at the adapters |
| Crates | split only with a real need for independence |

### Answer to the "long src files / DDD / subfiles" question (2026-11-06)

The `tcg-matchmaker` worker tree (`auth/`, `events/`, `stores/`, … each with
`index/repository/router/service`) is **feature-first modularization**
(vertical slices), not DDD: there is no `domain/`, no aggregates and no
bounded-context language. It is a good *code-modularization* alternative for a
many-feature backend and is kept as the growth path. Full decision, all
alternatives and references: `docs/adr-004-modularization-decision.md`.

- **Do not adopt DDD as a file-splitting strategy.** DDD is chosen by domain
  complexity (real invariants, several bounded contexts), not by file length;
  this crate has one bounded context. Do not create empty
  `domain/ application/ infrastructure/` folders or a repository per noun.
- **Do not split by a line-count threshold or one-struct-per-file.** A
  cohesive 500-line file is better than five artificially separated files.
  Rust Book ch. 7: modules control visibility and encapsulation; group related
  functionality and expose a small `pub use` facade.
- **Split a file when it holds more than one coherent responsibility.** This
  session extracted `ui/switcher.rs` (one shared search/new-tab window) and
  `ui/page_icon.rs` (page-icon feature) instead of a layer scaffold;
  `ui.rs` stays the facade.
- **Defer multiple crates / a Cargo workspace** until there is a real need for
  independent versioning (Rust Book ch. 14). When a second bounded context
  appears, migrate one feature at a time into `src/modules/<feature>/`
  following the `tcg-matchmaker` shape, keeping only the layers it needs.

### API contract (server)

| Method   | Path                    | Purpose                              |
| -------- | ----------------------- | ------------------------------------ |
| `GET`    | `/health`               | Liveness probe                       |
| `GET`    | `/api/workspace`        | Full workspace snapshot              |
| `PUT`    | `/api/workspace`        | Replace workspace (used by the UI)   |
| `GET`    | `/api/pages`            | Page summaries                       |
| `GET`    | `/api/pages/{id}`       | One page                             |
| `POST`   | `/api/pages`            | Create page from a draft             |
| `PATCH`  | `/api/pages/{id}`       | Update one page                      |
| `DELETE` | `/api/pages/{id}`       | Delete page and its descendants      |
| `GET`    | `/api/pages/{id}/versions` | Version history (newest first)    |
| `GET`    | `/api/search?q=`        | Ranked search results                |
| `GET`    | `/api/members`          | People directory (accounts / roles)  |
| `POST`   | `/api/members`          | Invite a member (`MemberDraft`)       |
| `GET`    | `/api/shares?page=`     | Permission grants for a page          |
| `PUT`    | `/api/shares`           | Set a permission (`Share`)            |
| `GET`    | `/api/sync?since=`      | Versioned changes after a cursor      |
| `GET`    | `/api/export`           | Notion-style markdown + People export |
| `GET`    | `/api/realtime`         | Decided WebSocket / CRDT contract     |

Responses are `ApiResponse { status: u16, body: String }`. JSON bodies are
enveloped as `{ "data": ..., "error": null | "message" }`. Status codes follow
standard REST (`200`, `201`, `400`, `404`, `500` on a storage failure). The
loopback server exposes these unversioned routes today; the full production
contract (`/api/v1`, sessions, `WS /ws/workspace/{id}`) is **decided** in
`docs/adr-003-self-hosted-collaborative-api.md` and scoped by
`spec/spec-011-collaborative-server.md`. The DTOs live in `src/collab.rs`.
The complete implemented-vs-decided reference is `docs/api-contract.md`.

**Public API alignment (B4, 2026-11-08):** the public Notion developer docs were
reviewed and compared with our contract. ADR-006 decides to adopt compatible
patterns behind our own `/api/v2`, not to clone Notion. The roadmap is
`spec-030` (versioning/errors/capabilities), `031` (pagination), `032` (rich
text/blocks), `033` (properties/data sources), `034` (views), `035`
(comments/users/files), `036` (webhooks), `037` (limits) and `038` (Axum web
server). Specs 030–038 are implemented: `/api/v2`, required
`Ion-Of-Notes-Version`, Bearer auth, `ApiError`,
capabilities, request IDs, `ListResponse` and opaque cursors, rich text/blocks,
typed properties/data sources, views, comments/users/files, signed webhook
delivery and the operational rate/size/block contract with retry guidance. v1
routes stay stable. Full analysis: `docs/public-api-analysis.md`.

## Code Conventions

- Rust idioms: `Result` over panics in library code, no `unwrap()` on user input.
- Public functions get short doc comments; module-level docs explain the seam.
- Pure logic goes in `model.rs`; I/O goes in `store.rs`; protocol logic goes in
  `server.rs`. No egui types leak into `model`, `server`, `client`, or `store`.
- **No `#[cfg(test)]` modules in `src/`.** Tests live in `tests/` as separate
  crates and use only the public API; shared helpers go in `tests/common/mod.rs`.
- Persistence is reached only through the `WorkspaceStore` port; the JSON
  adapter is the only code that touches the data file.
- UI copy is short, neutral sample text. No brand names, logos, photos, or long
  passages copied from anywhere.
- Colors and text styles are centralized in `ui::Palette`.
- Every user-visible command is also reachable through the client API.

## Commands

| Command                                  | Purpose                                  |
| ---------------------------------------- | ---------------------------------------- |
| `cargo build`                            | Build the desktop app (default `desktop` feature) |
| `cargo build --no-default-features`      | Build the core library without eframe/egui/wgpu |
| `cargo build --release --no-default-features --bin ion-of-notes-server` | Build the headless server binary |
| `cargo test --all-targets --no-default-features` | Core integration tests (229, no desktop stack) |
| `cargo test --features desktop --test ui --test fidelity --test ui_crdt --test screenshot` | Desktop UI tests (26) |
| `cargo test --all-targets`               | All default-feature tests (254; the core-only seam test is skipped) |
| `cargo clippy --all-targets -- -D warnings` | Lint gate                             |
| `cargo fmt --check`                      | Formatting gate                          |
| `pwsh -File scripts/quality.ps1`         | Full gate: fmt, clippy, core + desktop tests, docs |
| `pwsh -File scripts/check-docs.ps1`      | Links, Mermaid fences and table columns  |
| `cargo run -- --screenshot shots/now.png`| Render ~120 frames, save PNG, exit       |
| `cargo run -- --serve 127.0.0.1:8787`   | Run the Axum API server                  |
| `cargo run -- --serve 127.0.0.1:8787 --database data.db` | Run with SQLite persistence |
| `docker compose up --build`              | Build and run the self-hosted server stack |
| `docker compose down`                    | Stop the stack; the `ion-data` volume stays |

Build profiles keep the full gate usable on memory-constrained machines:
`[profile.dev] debug = "line-tables-only"` and `[profile.test] debug = 0`
remove the ~250 MB PDB per test binary, while `incremental = true` and Cargo's
default job count stay unchanged (spec-048). The `desktop` feature is on by
default; core tests opt out with `--no-default-features`, so they never
compile or link eframe/egui/wgpu.

The headless `ion-of-notes-server` binary is built with
`--no-default-features` and shares `app::serve` with the desktop `--serve`
path. `Dockerfile` produces a slim Debian runtime image, and `compose.yml`
runs it with the `ion-data` volume, a required owner password and the
`/health` healthcheck (spec-050).

The window opens at 1322x861 logical points, the size of the reference
pictures. OS decorations are disabled (`with_decorations(false)`): the Rust
tab strip is the only window frame and doubles as a drag handle.
`ION_OF_NOTES_SHOW=library|share|inbox|settings|settings-general|settings-sidebar|settings-people|settings-offline|settings-server|search|tasks|newtab|ai|shortcuts|iconpicker|pageactions|discussions|onboarding|templates`
is a harmless preview hook: it opens one pictured state on startup so it can be
captured with `--screenshot`. Normal runs (and the rebuild check) never set it.

Do not start long-running watchers or dev servers during a build session.

### Commits (automation rule)

**Always commit after completing a spec.** Finishing a spec means: build green,
`cargo test` green, a fresh `--screenshot` compared with the 98 `reference/`
pictures (and the 72 `screenshots/` pictures when relevant), and the docs above
updated (`CHANGELOG.md`, the relevant spec,
`AGENTS.md` when foundations change, and any ADR). Then stage the whole
`app/` tree and create one git commit whose message names the spec. This is
registered here so the step is automated rather than remembered.

## Docs Maintenance

The documentation hub is `docs/index.md`; it owns the reading order and the
source-of-truth rules. `spec/README.md` owns the spec index and status.

- Behavior change → update `docs/requirements.md` and the relevant spec.
- New entity/field → update `docs/data-model.md`.
- New route/DTO/status → update `docs/api-contract.md`.
- New module or dependency-direction change → update `AGENTS.md` and
  `docs/architecture.md`.
- New term → update `docs/glossary.md`.
- Each implemented spec → add a `CHANGELOG.md` entry and update
  `spec/README.md`.
- Foundation change → update `AGENTS.md`.
- Architecture review / improved guidelines → **always add the improved
  project guidelines in `AGENTS.md`** (this file) and record the decision as
  an ADR under `docs/adr-NNN-*.md`.
- Stack/library decision → add an ADR under `docs/adr-NNN-*.md`.
- Clean-room observation → update `docs/api-observations.md` and append a dated
  entry to `docs/reverse-engineering-log.md`; never add a capture under the
  current B1 scope.
- Public API review → update `docs/public-api-analysis.md`,
  `docs/api-observations.md` (mark **Observed (public docs)**), the relevant
  ADR, and the specs; do not claim wire compatibility with Notion.
- Keep `docs/unknowns.md` in sync with open questions about the reconstruction.
- Keep `docs/review-findings.md` current when a review finds drift; resolve it
  in a spec rather than silently editing history.
- Retired or historical specs keep a dated status note; do not rewrite them as
  if the old decision had never happened.

## External References

- eframe / egui: https://docs.rs/eframe/latest/eframe/ and https://docs.rs/egui/latest/egui/
- Rust API Guidelines: https://github.com/rust-lang/api-guidelines
- The Rust Programming Language, ch. 7 "Managing Growing Projects with
  Packages, Crates, and Modules":
  https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html
- 2026-11-05 architecture review (Modular Monolith + Clean Architecture +
  selective DDD, feature-based growth path, visibility rules, bootstrap layer):
  recorded in `docs/adr-002-architecture-guidelines.md` and
  `spec/spec-010-architecture-guidelines.md`.
- 2026-11-06 modularization decision (the `tcg-matchmaker` worker tree is
  feature-first, not DDD; split by responsibility, not line count; Rust Book
  ch. 7 / ch. 14, Rust API Guidelines): `docs/adr-004-modularization-decision.md`
  and `spec/spec-013-interaction-polish-and-modularity.md`.
- 2026-11-07 clean-room server observation (public docs + own-account HAR +
  mock contract; Wireshark only for traffic we control; redaction and a
  clean-room log): `docs/adr-005-clean-room-server-observation.md` and
  `spec/spec-016-architecture-follow-up-and-observation.md`.
- Spec Driven Development: https://www.ibm.com/think/topics/spec-driven-development
- Self-hosted collaborative API decision (accounts, sync, sharing, real-time,
  and the workspace export tree): `docs/adr-003-self-hosted-collaborative-api.md`
  and `spec/spec-011-collaborative-server.md`.
- Keyboard shortcuts: the app menu in `reference/picture-6.png` ..
  `reference/picture-11.png` and
  <https://www.notion.com/help/keyboard-shortcuts> (implemented in
  `shortcuts.rs`, `spec/spec-012-interaction-polish.md`).
- AI assistant transport: Ollama — the `ollama` CLI in `PATH` or the service at
  a network `host:port` (`OLLAMA_HOST`, default `127.0.0.1:11434`). See
  `src/ollama.rs` and the Settings → AI assistant page.
- The earlier free/paid feature references (`/product/features`, `/pricing`)
  informed the now-retired `spec-007` framing. The app no longer emphasizes a
  free edition; visual scope is the 98 pictures in `reference/` plus the 72
  `screenshots/` pictures. Mail & Calendar (`reference/picture-22.png`) is
  deliberately not implemented.
- Documentation hub, architecture, testing, glossary, review findings, API
  contract and clean-room guide: `docs/index.md`, `docs/architecture.md`,
  `docs/testing.md`, `docs/glossary.md`, `docs/review-findings.md`,
  `docs/api-contract.md`, `docs/clean-room.md` (spec-029).
- Clean-room evidence levels and phases: `docs/api-observations.md`,
  `docs/reverse-engineering-log.md`, `docs/unknowns.md`.
- Public Notion API alignment (B4): `docs/public-api-analysis.md`,
  `docs/adr-006-public-api-alignment.md`, and specs 030–038. The public
  developer docs are allowed evidence; captures are not.
- Public API reference: <https://developers.notion.com/reference/intro>,
  <https://developers.notion.com/reference/pagination>,
  <https://developers.notion.com/reference/capabilities>,
  <https://developers.notion.com/reference/webhooks>,
  <https://developers.notion.com/reference/request-limits>,
  <https://developers.notion.com/reference/versioning>.
