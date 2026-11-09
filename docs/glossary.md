# Glossary — ion-of-notes

Shared vocabulary for contributors and agents. When a term has a precise
meaning in this project, this table is the source of truth. Terms are grouped
by concern; the "Where" column points at the code or document that owns the
definition.

## 1. Process and documentation

| Term | Meaning | Where |
| ---- | ------- | ----- |
| **SDD** | Spec-Driven Development: every feature starts with a numbered spec before code. | `AGENTS.md`, `spec/` |
| **TDD** | Test-Driven Development: red → green → refactor for every feature and bug fix. | `AGENTS.md`, `docs/testing.md` |
| **Spec** | One working session's boundary: purpose, evidence, acceptance criteria, test scenarios, plan. | `spec/spec-NNN-*.md` |
| **ADR** | Architecture Decision Record: a durable decision with context, alternatives and consequences. | `docs/adr-NNN-*.md` |
| **FR / NFR** | Functional requirement / non-functional requirement. | `docs/requirements.md` |
| **Given / When / Then** | Acceptance-criteria form: precondition, action, observable result. | specs |
| **Red / Green / Refactor** | Observe a failing test, make it pass minimally, then improve without behavior change. | `docs/testing.md` |
| **Definition of done** | Tests, lint, format, screenshot, docs and commit all complete. | `docs/testing.md` §10 |
| **Preview hook** | `ION_OF_NOTES_SHOW=<state>` opens one pictured state for `--screenshot`. | `AGENTS.md`, `ui/app.rs` |
| **Clean room** | Reconstruct only from public behavior; never read, run or decompile the original. | `docs/clean-room.md` |
| **Evidence level** | `UI-observed`, `Inferred`, `Decided` or `Unknown`; prevents an inference from becoming a false fact. | `docs/api-observations.md` |
| **B1 … B8** | The clean-room observation phases in `spec-016` (scope gate, inventory, capture, contract, mock, tooling, log, stop). | `spec/spec-016-*` |

## 2. Architecture

| Term | Meaning | Where |
| ---- | ------- | ----- |
| **Modular Monolith** | One executable organized into cohesive modules, not microservices. | ADR-002 |
| **Clean Architecture** | Domain rules independent of GUI, storage and frameworks. | ADR-002 |
| **Selective DDD** | Use DDD only where the domain has real invariants; not a file-splitting rule. | ADR-002, ADR-004 |
| **Layer** | Domain, application, infrastructure, presentation or bootstrap. | `docs/architecture.md` §2 |
| **Port** | A trait the core declares for something it needs (`WorkspaceApi`, `WorkspaceStore`, `CollabStore`). | `src/server.rs`, `src/store.rs` |
| **Adapter** | A concrete implementation of a port (`JsonStore`, `JsonCollabStore`, loopback HTTP). | `src/store.rs`, `src/server/http.rs` |
| **Composition root / bootstrap** | The one place that wires concrete adapters into the core. | `src/app.rs` |
| **Facade** | A module root that declares private submodules and re-exports a small public surface. | `src/ui.rs`, `src/server.rs` |
| **DTO** | Data-transfer shape at a boundary, separate from a domain entity. | `src/collab.rs`, `PageDraft`, `PageSummary` |
| **Invariant** | A rule that must always hold (unique ids, valid parents, monotonic version). | `docs/data-model.md` |
| **Dependency inversion** | High-level code depends on abstractions, not concrete I/O. | ADR-002 |
| **Open/Closed** | Add a new adapter without editing the server. | ADR-002 |
| **Null object** | `NoStore` / `NoCollabStore` implement a port by doing nothing. | `src/store.rs` |

## 3. Domain

| Term | Meaning | Where |
| ---- | ------- | ----- |
| **Workspace** | The whole document: name, settings, pages, version. | `src/model.rs` |
| **Page** | A titled node with an icon, optional parent, blocks, optional table rows, flags and comments. | `src/model.rs` |
| **Block** | One editable unit of page content (paragraph, heading, list, to-do, quote, callout, code, divider). | `src/model.rs` |
| **BlockKind** | The enum of block variants. | `src/model.rs` |
| **TableRow** | One row of the inline database (name, status, assignee, due). | `src/model.rs` |
| **Comment** | A page-level discussion message. | `src/model.rs` |
| **WorkspaceSettings** | Persisted workspace-level preferences (icon, landing page, domains, toggles). | `src/model/settings.rs` |
| **PageDraft** | The create-page request body (`title`, `icon`, `parent`). | `src/model.rs` |
| **PageSummary** | The sidebar/list view of a page. | `src/model.rs` |
| **PageDiff** | Pure difference between two workspaces: changed, added, removed, reordered, settings changed. | `src/model/diff.rs` |
| **WorkspaceIndex** | Derived id/parent lookup structure; skipped by serde and rebuilt on mutation. | `src/model/index.rs` |
| **Seed** | The neutral demo workspace created on first start. | `src/model/seed.rs` |
| **Archive** | A page flag that hides a page from normal views while keeping it in the Library Archive view. | `src/model.rs` |
| **Lock** | A page flag that disables editing. | `src/model.rs` |
| **Offline** | A page flag that marks a page for the offline list. | `src/model.rs` |

## 4. Collaboration

| Term | Meaning | Where |
| ---- | ------- | ----- |
| **Member** | An account / row of the People directory. | `src/collab.rs` |
| **Role** | `Owner`, `Member` or `Guest`. | `src/collab.rs` |
| **Share** | A permission grant keyed by page and subject; `page_id == "*"` is workspace-general. | `src/collab.rs` |
| **Access** | `Full`, `Edit`, `View` or `None`; `Full`/`Edit` can write. | `src/collab.rs` |
| **General access** | The "Everyone at workspace" rule, resolved after a page-specific rule. | `src/collab.rs` |
| **Change** | One entry of the versioned sync log (`seq`, `page_id`, `op`, `actor`). | `src/collab.rs` |
| **ChangeOp** | `Create`, `Update`, `Delete` or `Replace`. | `src/collab.rs` |
| **SyncBatch** | Changes after a cursor plus the new cursor. | `src/collab.rs` |
| **PageVersion** | A stored snapshot of a page for version history. | `src/collab.rs` |
| **Retention** | At most 50 newest snapshots per page; older ones are pruned. | `src/collab.rs` |
| **RealtimeContract** | The decided WebSocket channel and CRDT conflict model, exposed as data. | `src/collab.rs` |
| **CRDT** | Conflict-free replicated data type; the decided per-block merge strategy. | ADR-003 |
| **Export tree** | In-memory markdown/CSV files: page files, folder pages, `People` folder. | `src/collab/export.rs` |
| **CollabState** | Server-owned members, shares, changes and versions. | `src/collab.rs` |

## 5. Client, server and persistence

| Term | Meaning | Where |
| ---- | ------- | ----- |
| **WorkspaceApi** | The transport seam: `handle(ApiRequest) -> ApiResponse`. | `src/server.rs` |
| **Client** | The only data access the UI may use; typed methods over `WorkspaceApi`. | `src/client.rs` |
| **Envelope** | JSON response shape `{ data, error }`. | `src/server.rs` |
| **ApiRequest / ApiResponse** | Transport-neutral request (`method`, `path`, `body`) and response (`status`, `body`). | `src/server.rs` |
| **Loopback adapter** | The optional `--serve` HTTP/1.1 listener bound to `127.0.0.1`. | `src/server/http.rs` |
| **PATCH path** | Single-page edit persistence (`PATCH /api/pages/{id}`). | `src/client.rs` |
| **Full replace** | `PUT /api/workspace` for structural or settings changes. | `src/server.rs` |
| **Debounce** | Wait 250 ms after the last edit before sending, with force-flush points. | `src/ui/edit_queue.rs` |
| **Commit plan** | Pure decision: `None`, patch changed pages, or full replace. | `src/ui/edit_queue.rs` |
| **Cursor** | The `seq` watermark used by `GET /api/sync?since=`. | `src/collab.rs` |
| **Atomic save** | Write a temporary file, then move it into place. | `src/store.rs` |
| **Composition root** | `build_server` / `build_desktop` wire stores, server and client. | `src/app.rs` |

## 6. UI

| Term | Meaning | Where |
| ---- | ------- | ----- |
| **Command** | A user intention collected by the UI and applied through `Client`. | `src/ui/commands.rs` |
| **ViewState** | A read-only snapshot passed to render functions. | `src/ui/view.rs` |
| **Apply** | The single place that turns a `Command` into a state transition and client call. | `src/ui/apply.rs` |
| **Palette** | The single source of colors, fonts and spacing. | `src/ui/palette.rs` |
| **Switcher** | The shared search / "Open in new tab…" window. | `src/ui/switcher.rs` |
| **Page icon** | The page-icon catalog, picker and renderer. | `src/ui/page_icon.rs` |
| **Tab title** | The truncated label shown in the tab strip. | `src/ui/palette.rs` |
| **Library view** | Recents, Favorites, Shared, Private or Archive filtering of pages. | `src/ui/library_filter.rs` |
| **Settings matcher** | Pure title/description matcher for settings search. | `src/ui/settings_search.rs` |
| **SettingsRow** | One searchable settings row (`id`, `section`, `title`, `description`). | spec-016 A9 |
| **Search cache** | Query/version cache so an unchanged frame does not re-query. | `src/ui/search_cache.rs` |
| **Onboarding** | The first-run "What is this space for?" choice. | `src/ui/dialogs/onboarding.rs` |
| **Template** | A starter page blueprint (work, personal, school, meeting, blank). | `src/templates.rs` |
| **Locale** | `en`, `pt-BR` or `eo`; the key table is the source of truth. | `locales/`, `src/i18n.rs` |

## 7. Clean room and evidence

| Term | Meaning | Where |
| ---- | ------- | ----- |
| **UI-observed** | Visible in the reference pictures or normal product behavior. | `docs/api-observations.md` |
| **Inferred** | A reasonable server implication not confirmed by public docs. | `docs/api-observations.md` |
| **Decided** | Our own contract (ADR-003); not a claim about the original. | `docs/api-observations.md` |
| **Unknown** | Needs public documentation or a future authorized source. | `docs/api-observations.md` |
| **HAR** | HTTP Archive; **not authorized** under the strict B1 scope. | `docs/clean-room.md` |
| **Interception proxy** | `mitmproxy` / Charles; only for traffic we control. | ADR-005 |
| **Wireshark** | Packet analyzer; not the primary discovery tool. | ADR-005 |
| **Stop condition** | A source or action that would cross the clean-room line; stop and ask. | `docs/clean-room.md` |

## 8. Public API alignment (planned v2)

| Term | Meaning | Where |
| ---- | ------- | ----- |
| **B4** | The public-documentation review phase; public developer docs are allowed. | `docs/clean-room.md` |
| **Observed (public docs)** | A fact stated by a public Notion developer page. | `docs/api-observations.md` |
| **Ion-Of-Notes-Version** | Our own required version header for `/api/v2`; not `Notion-Version`. | spec-030 |
| **ApiError** | Structured error object with `status`, `code`, `message`, `request_id`, `additional_data`. | spec-030 |
| **Capability** | A connection right: content read/update/insert, comment read/insert, user-info level. | spec-030 |
| **Connection** | A token + member + workspace + capability set. | spec-030 |
| **RequestLogEntry** | In-memory record of a v2 request ID, method, path and status. | spec-030 |
| **owner-token** | Local demo owner bearer token; replaced by sessions in production. | spec-030 |
| **Cursor / start_cursor / next_cursor** | Opaque pagination tokens; pass back unchanged. | spec-031 |
| **ListResponse** | `{ object:"list", type, results, has_more, next_cursor }`. | spec-031 |
| **RichText** | Text, mention or equation with annotations, `plain_text` and `href`. | spec-032 |
| **Block metadata** | `object`, `id`, `parent`, timestamps, users, `in_trash`, `has_children`. | spec-032 |
| **BlockDto / BlockInput** | Read/write block payloads for the v2 block endpoints. | spec-032 |
| **ParentDto** | Page/block/workspace parent object in the v2 block API. | spec-032 |
| **Property** | A data source schema entry: id, name, type and type-specific settings. | spec-033 |
| **PropertyValue** | A typed page value for a property. | spec-033 |
| **PageProperty** | A property value with its stable id and type. | spec-033 |
| **PropertyItem** | A single value or a paginated item of a property. | spec-033 |
| **Data source** | A table under a database, owning its schema and rows. | spec-033 |
| **Database** | A container of one or more data sources. | spec-033 |
| **View** | A saved table/board/calendar/list layout with filter, sorts and config. | spec-034 |
| **Filter** | Typed predicate tree (and/or, equals, contains, empty, range, checked). | spec-034 |
| **Sort** | `{ property, direction }` applied to a view query. | spec-034 |
| **ViewQueryResult** | View id/type, grouped pages and a paginated page list. | spec-034 |
| **CommentDto** | A v2 comment with page/block parent, rich text and attachments. | spec-035 |
| **CommentAttachment** | A file attached to a comment; max 3 per comment. | spec-035 |
| **CommentDisplayName** | `connection`, `user` or `custom` author label. | spec-035 |
| **FileObject** | `file` (hosted), `file_upload` (API upload) or `external` URL. | spec-035 |
| **FileUpload** | Upload lifecycle: pending → uploaded, or expired/failed. | spec-035 |
| **UserDto** | Person/bot user object with capability-filtered profile fields. | spec-035 |
| **WebhookSubscription** | A verified HTTPS endpoint with selected event types. | spec-036 |
| **HMAC-SHA256 signature** | `X-Ion-Of-Notes-Signature` over the raw webhook body. | spec-036 |
| **Aggregated event** | A batched event such as `page.content_updated`. | spec-036 |
| **Plan** | `free`/`business`/`enterprise`, used only for operational limits. | spec-037 |
| **RequestBudget** | `{ plan, members, blocks }` inputs that derive the limits. | spec-037 |
| **Block limit** | Free multi-member workspace lifetime block limit (1000). | spec-037 |
| **Retry-After** | Seconds to wait after a 429/529 before retrying. | spec-037 |
| **RetryPolicy** | Client decision: retryable status, delay and attempt cap. | spec-037 |
| **Axum adapter** | Tokio/Tower HTTP transport for `--serve`; calls `WorkspaceApi`. | spec-038 |
| **Account** | A local login record with an Argon2id password hash. | spec-039 |
| **Session** | An opaque login token stored only as a SHA-256 hash with an expiry. | spec-039 |
| **SqliteStore** | SQLite adapter for workspace, collaboration and auth state. | spec-039 |

## 9. Naming rules

- Specs: `spec-NNN-kebab-case-title.md`, numbered sequentially.
- ADRs: `adr-NNN-kebab-case-title.md`, numbered sequentially.
- Test files: named after the module they exercise.
- Commands and DTOs: `PascalCase`; functions and fields: `snake_case`.
- UI state keys (for example Library tab names) stay stable English even when
  the display label is translated.
- No product brand names, logos, photos or copied long text anywhere.
