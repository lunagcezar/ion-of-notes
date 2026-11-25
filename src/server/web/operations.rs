use super::*;

pub(super) async fn health() -> Response {
    let mut response = r#"{"data":"ok","error":null}"#.into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/json; charset=utf-8"),
    );
    response
}

/// Readiness probe: the process is up and the router is serving.
pub(super) async fn ready() -> Response {
    let mut response = r#"{"data":"ready","error":null}"#.into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/json; charset=utf-8"),
    );
    response
}

/// Minimal Prometheus text exposition for uptime, request volume, workspace
/// size, sync cursor and webhook outbox state.
pub(super) async fn metrics(State(state): State<AppState>) -> Response {
    let uptime = state.started.elapsed().as_secs();
    let requests = state.requests.load(Ordering::Relaxed);
    let snapshot = {
        let server = state.server.lock_or_log();
        let stats = server.webhook_stats();
        Some((
            stats.pending,
            stats.dead_letters,
            stats.deliveries,
            server.workspace.pages.len(),
            server.workspace.block_count(),
            server.collab.members.len(),
            server.workspace.version,
            server.collab.next_seq.saturating_sub(1),
            server.request_log().len(),
            server.upload_usage(),
            server.upload_quota_remaining(),
        ))
    };
    let (
        pending,
        dead_letters,
        deliveries,
        pages,
        blocks,
        members,
        version,
        sync_seq,
        request_log,
        upload_bytes,
        upload_free,
    ) = snapshot.unwrap_or((0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0));
    let body = format!(
        "# HELP ion_of_notes_up Whether the server process is running.\n\
         # TYPE ion_of_notes_up gauge\n\
         ion_of_notes_up 1\n\
         # HELP ion_of_notes_uptime_seconds Process uptime in seconds.\n\
         # TYPE ion_of_notes_uptime_seconds gauge\n\
         ion_of_notes_uptime_seconds {uptime}\n\
         # HELP ion_of_notes_http_requests_total API requests handled since start.\n\
         # TYPE ion_of_notes_http_requests_total counter\n\
         ion_of_notes_http_requests_total {requests}\n\
         # HELP ion_of_notes_webhook_pending Aggregated webhook events waiting to flush.\n\
         # TYPE ion_of_notes_webhook_pending gauge\n\
         ion_of_notes_webhook_pending {pending}\n\
         # HELP ion_of_notes_webhook_dead_letters Deliveries that exhausted retries.\n\
         # TYPE ion_of_notes_webhook_dead_letters gauge\n\
         ion_of_notes_webhook_dead_letters {dead_letters}\n\
         # HELP ion_of_notes_webhook_deliveries_total Delivery records created.\n\
         # TYPE ion_of_notes_webhook_deliveries_total counter\n\
         ion_of_notes_webhook_deliveries_total {deliveries}\n\
         # HELP ion_of_notes_pages Current page count.\n\
         # TYPE ion_of_notes_pages gauge\n\
         ion_of_notes_pages {pages}\n\
         # HELP ion_of_notes_blocks Current block count (pages count as one).\n\
         # TYPE ion_of_notes_blocks gauge\n\
         ion_of_notes_blocks {blocks}\n\
         # HELP ion_of_notes_members Current workspace member count.\n\
         # TYPE ion_of_notes_members gauge\n\
         ion_of_notes_members {members}\n\
         # HELP ion_of_notes_workspace_version Current workspace version.\n\
         # TYPE ion_of_notes_workspace_version gauge\n\
         ion_of_notes_workspace_version {version}\n\
         # HELP ion_of_notes_sync_seq Latest synchronization-log sequence.\n\
         # TYPE ion_of_notes_sync_seq gauge\n\
         ion_of_notes_sync_seq {sync_seq}\n\
         # HELP ion_of_notes_request_log_entries In-memory v2 request-log entries.\n\
         # TYPE ion_of_notes_request_log_entries gauge\n\
         ion_of_notes_request_log_entries {request_log}\n\
         # HELP ion_of_notes_upload_bytes Bytes stored in the workspace upload directory.\n\
         # TYPE ion_of_notes_upload_bytes gauge\n\
         ion_of_notes_upload_bytes {upload_bytes}\n\
         # HELP ion_of_notes_upload_quota_remaining Remaining upload bytes before the quota.\n\
         # TYPE ion_of_notes_upload_quota_remaining gauge\n\
         ion_of_notes_upload_quota_remaining {upload_free}\n\
         # HELP ion_of_notes_poisoned_locks_total Mutexes found poisoned and recovered.\n\
         # TYPE ion_of_notes_poisoned_locks_total counter\n\
         ion_of_notes_poisoned_locks_total {poisoned}\n",
        poisoned = crate::lock::POISONED_LOCKS.load(Ordering::Relaxed)
    );
    let mut response = body.into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("text/plain; version=0.0.4"),
    );
    response
}
