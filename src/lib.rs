//! `ion-of-notes` — a clean-room, self-hosted workspace application.
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
//! The desktop stack (`ui`, `screenshot`, `eframe`, `egui`, `image`) lives
//! behind the default-on `desktop` feature. Core tests build with
//! `--no-default-features` so they do not compile or link the window.
//!
//! The project is open source (MIT). That is a license fact, not a marketing
//! feature: the window does not advertise a "free edition".

rust_i18n::i18n!("locales", fallback = "en");

pub mod app;
pub mod auth;
pub mod cli;
pub mod client;
pub mod collab;
pub mod i18n;
pub mod model;
pub mod ollama;
pub mod remote;
#[cfg(feature = "desktop")]
pub mod screenshot;
pub mod server;
pub mod shortcuts;
pub mod store;
pub mod templates;
#[cfg(feature = "desktop")]
pub mod ui;
pub mod webhooks;

pub use app::{
    DesktopApi, build_desktop, build_desktop_at, build_desktop_at_paths, build_remote_desktop,
    build_server, build_server_at, build_server_at_paths, build_server_sqlite,
    build_server_sqlite_at, serve,
};
pub use auth::{
    Account, AuthStore, DEFAULT_OWNER_PASSWORD, LoginInput, MemoryAuthStore, NoAuthStore,
    SESSION_DAYS, Session, SessionInfo, SessionResponse, hash_password, hash_token,
    new_session_token, read_password_file, session_expiry, session_is_expired,
    session_is_expired_at, verify_password,
};
pub use client::{Client, ClientError, RetryDecision, RetryPolicy};
pub use collab::{
    Access, Change, ChangeOp, CollabState, ExportFile, MAX_VERSIONS_PER_PAGE, Member, MemberDraft,
    PageVersion, RealtimeContract, RealtimeEvent, RealtimeKind, Role, Share, SyncBatch,
    export_tree, page_markdown, people_csv, people_markdown, realtime_contract,
};
pub use model::{
    Annotations, AttachmentInput, Block, BlockChildrenRequest, BlockDto, BlockInput, BlockKind,
    Bot, BotOwner, Comment, CommentAttachment, CommentDisplayName, CommentDisplayNameDto,
    CommentDto, CommentInput, DataSource, DataSourceInput, DataSourceRef, Database, DatabaseInput,
    DateValue, Equation, FileObject, FileRef, FileUpload, FileUploadInput, FormulaResult, Mention,
    OptionRef, Page, PageDiff, PageDraft, PagePropertiesInput, PageProperty, PageRef, PageSummary,
    ParentDto, Person, Property, PropertyConfig, PropertyType, PropertyValue, RichText,
    RollupResult, SearchResult, SelectOption, StatusGroup, TableRow, TextContent, TextLink,
    UploadStatus, UserDto, UserRef, UserType, Workspace, WorkspaceError, WorkspaceLimits,
};
pub use ollama::{
    DEFAULT_HOST, DEFAULT_MODEL, DEFAULT_PORT, OllamaConfig, OllamaStatus, build_generate_body,
    cli_version, decode_body, decode_chunked, default_endpoint, default_model_for, detect,
    generate, models, normalize_endpoint, parse_endpoint, parse_generate_response,
    parse_tags_response, service_reachable,
};
pub use remote::{
    RemoteApi, RemoteSession, clear_session, clear_session_at, load_session, load_session_at,
    save_session, save_session_at, session_path,
};
pub use server::{
    API_VERSION, ApiError, ApiRequest, ApiResponse, Capability, Connection, LimitViolation,
    ListResponse, Method, PageRequest, PaginationError, Plan, RateDecision, RateLimitPolicy,
    RateLimiter, RequestBudget, RequestLogEntry, SqliteRateLimiter, UreqWebhookSender,
    WebhookSender, WorkspaceApi, WorkspaceServer,
};
pub use shortcuts::{Action, Combo, SHORTCUT_TABLE, ShortcutDoc, lookup};
pub use store::{
    CollabStore, JsonCollabStore, JsonStore, NoCollabStore, NoStore, RateLimitHit, RateLimitStore,
    SqliteStore, WorkspaceStore,
};
pub use templates::{Template, catalog, find, template_for_purpose};
pub use webhooks::{
    ActorRef, EntityRef, SubscriptionStatus, WebhookDelivery, WebhookEvent, WebhookEventType,
    WebhookRequeueInput, WebhookStats, WebhookSubscription, WebhookSubscriptionInput,
    WebhookVerificationInput, aggregate_events, is_allowed_url, sign, verify,
};
