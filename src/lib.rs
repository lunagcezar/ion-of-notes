//! `not-notion` — a clean-room, self-hosted workspace application.
//!
//! The crate is split at hard seams, following ports-and-adapters:
//! `model` is pure data, `collab` holds the accounts/sharing/sync/export
//! domain, `store` owns the persistence port and its JSON adapter, `server`
//! owns all state behind the `WorkspaceApi` contract, `client` is the only
//! data access the UI is allowed to use, and `ui` draws the window. `shortcuts`
//! holds the keyboard table and `ollama` is the AI-assistant adapter (CLI in
//! PATH or service on a network port). High-level modules depend on traits,
//! never on concrete I/O.
//!
//! The project is open source (MIT). That is a license fact, not a marketing
//! feature: the window does not advertise a "free edition".

rust_i18n::i18n!("locales", fallback = "en");

pub mod app;
pub mod cli;
pub mod client;
pub mod collab;
pub mod i18n;
pub mod model;
pub mod ollama;
pub mod screenshot;
pub mod server;
pub mod shortcuts;
pub mod store;
pub mod ui;

pub use app::{
    build_desktop, build_desktop_at, build_desktop_at_paths, build_server, build_server_at,
    build_server_at_paths,
};
pub use client::{Client, ClientError};
pub use collab::{
    Access, Change, ChangeOp, CollabState, ExportFile, MAX_VERSIONS_PER_PAGE, Member, MemberDraft,
    PageVersion, RealtimeContract, RealtimeEvent, RealtimeKind, Role, Share, SyncBatch,
    export_tree, page_markdown, people_csv, people_markdown, realtime_contract,
};
pub use model::{
    Block, BlockKind, Comment, Page, PageDiff, PageDraft, PageSummary, SearchResult, TableRow,
    Workspace, WorkspaceError,
};
pub use ollama::{
    DEFAULT_HOST, DEFAULT_MODEL, DEFAULT_PORT, OllamaConfig, OllamaStatus, build_generate_body,
    cli_version, decode_body, decode_chunked, default_endpoint, default_model_for, detect,
    generate, models, normalize_endpoint, parse_endpoint, parse_generate_response,
    parse_tags_response, service_reachable,
};
pub use server::{ApiRequest, ApiResponse, Method, WorkspaceApi, WorkspaceServer};
pub use shortcuts::{Action, Combo, SHORTCUT_TABLE, ShortcutDoc, lookup};
pub use store::{CollabStore, JsonCollabStore, JsonStore, NoCollabStore, NoStore, WorkspaceStore};
