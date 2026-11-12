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

/// Minimal Prometheus text exposition for uptime and request volume.
pub(super) async fn metrics(State(state): State<AppState>) -> Response {
    let uptime = state.started.elapsed().as_secs();
    let requests = state.requests.load(Ordering::Relaxed);
    let (pending, dead_letters, deliveries) = state
        .server
        .lock()
        .ok()
        .map(|server| {
            let stats = server.webhook_stats();
            (stats.pending, stats.dead_letters, stats.deliveries)
        })
        .unwrap_or((0, 0, 0));
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
         ion_of_notes_webhook_deliveries_total {deliveries}\n"
    );
    let mut response = body.into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("text/plain; version=0.0.4"),
    );
    response
}
