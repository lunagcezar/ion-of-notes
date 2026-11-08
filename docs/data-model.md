# Data Model — not-notion

All entities are plain Rust structs deriving `Serialize`/`Deserialize`. The
workspace is one JSON document (`workspace.json`); collaboration state is a
second document (`collab.json`) so a corrupt collaboration file cannot destroy
pages (spec-019). No egui types appear here.

## Entity: Workspace

| Field     | Type         | Description                                  |
| --------- | ------------ | -------------------------------------------- |
| `name`    | `String`     | Display name, default `not-notion`           |
| `pages`   | `Vec<Page>`  | All pages, order is the sidebar order        |
| `version` | `u64`        | Increments on every successful write         |

## Entity: Page

| Field               | Type                | Description                                      |
| ------------------- | ------------------- | ------------------------------------------------ |
| `id`                | `String`            | UUID v4, unique inside the workspace             |
| `title`             | `String`            | Editable page title                              |
| `icon`              | `String`            | Glyph/emoji or image reference                   |
| `parent`            | `Option<String>`    | Parent page id, `None` for a root page           |
| `favorite`          | `bool`              | Shown in the Favorites sidebar section           |
| `locked`            | `bool`              | Prevents editing when true (serde default false) |
| `available_offline` | `bool`              | Marks page for offline listing (serde default)   |
| `blocks`            | `Vec<Block>`        | Ordered editable content                         |
| `rows`              | `Vec<TableRow>`     | Inline table database (empty for normal pages)   |
| `updated_at`        | `String`            | ISO-8601-like display timestamp                  |

## Entity: Block

| Field       | Type        | Description                                     |
| ----------- | ----------- | ----------------------------------------------- |
| `id`        | `String`    | UUID v4                                         |
| `kind`      | `BlockKind` | Rendering and behavior variant                  |
| `text`      | `String`    | Editable text (empty for a divider)             |
| `checked`   | `bool`      | Only meaningful for `Todo`                      |
| `collapsed` | `bool`      | Reserved for a future toggle variant            |

`BlockKind` variants: `Paragraph`, `Heading1`, `Heading2`, `Heading3`,
`Bulleted`, `Numbered`, `Todo`, `Quote`, `Callout`, `Code`, `Divider`.

## Entity: TableRow

| Field      | Type     | Description                                    |
| ---------- | -------- | ---------------------------------------------- |
| `id`       | `String` | UUID v4                                        |
| `name`     | `String` | Row title                                      |
| `status`   | `String` | One of `Not started`, `In progress`, `Done`    |
| `assignee` | `String` | Short person label                             |
| `due`      | `String` | Short date label                               |

## Value objects

- `PageDraft { title, icon, parent }` — the create-page request body.
- `PageSummary { id, title, icon, parent, favorite }` — sidebar/list view.
- `SearchResult { page_id, page_title, snippet, score }` — search response.
- `ApiRequest { method, path, body }` / `ApiResponse { status, body }` — the
  client/server contract.
- `Envelope<T> { data: Option<T>, error: Option<String> }` — JSON response body.

## Relationships

```
Workspace 1 --- * Page
Page       0..1 --- * Page        (self-referencing parent tree)
Page       1 --- * Block
Page       1 --- * TableRow
```

Deleting a page removes the whole subtree (all descendants), so no orphan
pages can remain. Ids are never reused.

## Invariants

- I1: `Workspace.pages` contains no duplicate ids.
- I2: Every non-null `parent` refers to an existing page id.
- I3: `version` only increases.
- I4: Search ranking is deterministic: title prefix > title contains > body
  contains, then by page order.

## Collaboration entities (spec-011 / ADR-003)

All in `src/collab.rs`; plain serde structs/enums with no egui, socket or file
types. They are the DTOs of the decided API.

| Entity | Fields | Purpose |
| ------ | ------ | ------- |
| `Member` | `id, name, email, role: Role` | An account / People-directory row |
| `Role` | `Owner \| Member \| Guest` | Membership type ("Workspace owner") |
| `MemberDraft` | `name, email, role` | `POST /api/members` invite body |
| `Share` | `page_id, subject, access: Access` | Permission grant; `page_id == "*"` is general |
| `Access` | `Full \| Edit \| View \| None` | Share popover levels; `can_write` for Full/Edit |
| `Change` | `seq, page_id, op, actor` | One versioned sync-log entry |
| `PageVersion` | `version, page_id, title, actor, updated_at, snapshot` | Stored page snapshot for version history |
| `ChangeOp` | `Create \| Update \| Delete \| Replace` | Mutation kind |
| `SyncBatch` | `cursor, changes: Vec<Change>` | `GET /api/sync?since=` response |
| `RealtimeEvent` | `kind, page_id, actor, payload` | WebSocket frame payload |
| `RealtimeKind` | `Presence \| Edit \| Awareness \| Sync` | Frame category |
| `RealtimeContract` | `transport, channel, events, conflict_resolution` | `GET /api/realtime` discovery |
| `ExportFile` | `path, content` | One file of the in-memory export tree |
| `CollabState` | `members, shares, changes, next_seq, versions` | Server-owned state + pure methods |

`CollabState` methods: `invite`, `set_access`, `shares_for`, `access_for`
(most-specific-first resolution), `can_write`, `record`, `changes_since`,
`record_version`, `versions_for`. It is persisted through the `CollabStore`
port (`JsonCollabStore` writes `collab.json`) and restored on startup. The
version snapshots are read by
`Client::page_versions` and shown in the page-actions version-history window
(spec-015); restoring one writes it back through `Client::update_page`.

## Export tree (takes the described workspace export into account)

`export_tree(workspace, members) -> Vec<ExportFile>` reproduces the structure
the person supplied:

- one markdown file per page, named `Title <id>.md`;
- a page with children is a **folder**: its own markdown lists links to the
  child markdown files (URL-encoded `%20` paths) and the children live in a
  same-named directory;
- the workspace root is itself a folder page linking to top-level pages;
- a `People` folder holds `People.md` (links + `filters:`/`sort:` view
  definition), `People_all.csv`, and one person file per member containing
  `Membership Type: <role>` and `Person: <name>`;
- inline table pages emit a companion `_all.csv`.

The export is pure (no filesystem I/O); writing the ZIP/files is a future
infrastructure adapter.
