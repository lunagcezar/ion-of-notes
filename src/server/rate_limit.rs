//! In-process sliding-window rate limiter for `/api/v2` (spec-037).
//!
//! The limiter is a policy adapter, not a domain rule. A production
//! deployment can replace it with a shared Redis/Postgres implementation
//! behind the same `check` call; the server only sees a `RateDecision`.

use crate::server::limits::Plan;
use crate::store::RateLimitStore;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// The connection's own budget is exhausted.
pub const REASON_CONNECTION: &str = "public_api_request_rate_limit";
/// The workspace's shared budget is exhausted.
pub const REASON_WORKSPACE: &str = "public_api_space_request_rate_limit";

/// One allow/deny decision with the wait the client should observe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateDecision {
    pub allowed: bool,
    /// Whole seconds until the current window resets; zero when allowed.
    pub retry_after: u64,
    /// `additional_data.rate_limit_reason` when denied.
    pub reason: Option<&'static str>,
}

impl RateDecision {
    fn allowed() -> Self {
        Self {
            allowed: true,
            retry_after: 0,
            reason: None,
        }
    }

    fn denied(retry_after: u64, reason: &'static str) -> Self {
        Self {
            allowed: false,
            retry_after: retry_after.max(1),
            reason: Some(reason),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct WindowCounter {
    window_start: Instant,
    count: u32,
}

/// Fixed-window limiter shared by every connection of one workspace.
#[derive(Debug, Clone)]
pub struct RateLimiter {
    window: Duration,
    per_connection: u32,
    per_workspace: u32,
    connections: BTreeMap<String, WindowCounter>,
    workspaces: BTreeMap<String, WindowCounter>,
}

impl RateLimiter {
    pub fn new(per_connection: u32, per_workspace: u32, window: Duration) -> Self {
        Self {
            window,
            per_connection,
            per_workspace,
            connections: BTreeMap::new(),
            workspaces: BTreeMap::new(),
        }
    }

    /// Default budgets for a plan: the documented per-minute connection limit
    /// and a workspace budget scaled from it.
    pub fn for_plan(plan: Plan, window: Duration) -> Self {
        let per_connection = plan.requests_per_minute();
        Self::new(per_connection, per_connection.saturating_mul(10), window)
    }

    pub fn window(&self) -> Duration {
        self.window
    }

    pub fn per_connection(&self) -> u32 {
        self.per_connection
    }

    pub fn per_workspace(&self) -> u32 {
        self.per_workspace
    }

    /// Account for one request. The connection budget is checked first, then
    /// the workspace budget shared by all connections.
    pub fn check(&mut self, connection_id: &str, workspace_id: &str, now: Instant) -> RateDecision {
        let (allowed, retry_after) = take(
            &mut self.connections,
            connection_id,
            now,
            self.window,
            self.per_connection,
        );
        if !allowed {
            return RateDecision::denied(retry_after, REASON_CONNECTION);
        }
        let (allowed, retry_after) = take(
            &mut self.workspaces,
            workspace_id,
            now,
            self.window,
            self.per_workspace,
        );
        if !allowed {
            return RateDecision::denied(retry_after, REASON_WORKSPACE);
        }
        RateDecision::allowed()
    }

    /// Clear every counter (used when the policy changes or in tests).
    pub fn reset(&mut self) {
        self.connections.clear();
        self.workspaces.clear();
    }
}

/// Rate-limit policy used by the server (in-process or shared).
pub trait RateLimitPolicy: Send + Sync {
    fn check(&mut self, connection_id: &str, workspace_id: &str, now: Instant) -> RateDecision;
    fn window(&self) -> Duration;
}

impl RateLimitPolicy for RateLimiter {
    fn check(&mut self, connection_id: &str, workspace_id: &str, now: Instant) -> RateDecision {
        RateLimiter::check(self, connection_id, workspace_id, now)
    }

    fn window(&self) -> Duration {
        RateLimiter::window(self)
    }
}

/// Shared fixed-window limiter backed by a `RateLimitStore` (SQLite today).
pub struct SqliteRateLimiter {
    store: Arc<dyn RateLimitStore>,
    window: Duration,
    per_connection: u32,
    per_workspace: u32,
}

impl SqliteRateLimiter {
    pub fn new(
        store: Arc<dyn RateLimitStore>,
        per_connection: u32,
        per_workspace: u32,
        window: Duration,
    ) -> Self {
        Self {
            store,
            window,
            per_connection,
            per_workspace,
        }
    }

    pub fn for_plan(store: Arc<dyn RateLimitStore>, plan: Plan, window: Duration) -> Self {
        let per_connection = plan.requests_per_minute();
        Self::new(
            store,
            per_connection,
            per_connection.saturating_mul(10),
            window,
        )
    }
}

impl RateLimitPolicy for SqliteRateLimiter {
    fn check(&mut self, connection_id: &str, workspace_id: &str, _now: Instant) -> RateDecision {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let window_secs = self.window.as_secs().max(1);
        match self.store.check(
            &format!("conn:{connection_id}"),
            window_secs,
            self.per_connection,
            now,
        ) {
            Ok(hit) if !hit.allowed => {
                return RateDecision::denied(hit.retry_after, REASON_CONNECTION);
            }
            Ok(_) => {}
            Err(error) => eprintln!("shared rate limit error: {error}"),
        }
        match self.store.check(
            &format!("ws:{workspace_id}"),
            window_secs,
            self.per_workspace,
            now,
        ) {
            Ok(hit) if !hit.allowed => RateDecision::denied(hit.retry_after, REASON_WORKSPACE),
            Ok(_) => RateDecision::allowed(),
            Err(error) => {
                eprintln!("shared rate limit error: {error}");
                RateDecision::allowed()
            }
        }
    }

    fn window(&self) -> Duration {
        self.window
    }
}

fn take(
    counters: &mut BTreeMap<String, WindowCounter>,
    key: &str,
    now: Instant,
    window: Duration,
    limit: u32,
) -> (bool, u64) {
    if limit == 0 {
        return (true, 0);
    }
    let elapsed = now.saturating_duration_since(
        counters
            .get(key)
            .map(|counter| counter.window_start)
            .unwrap_or(now),
    );
    let counter = counters.entry(key.to_string()).or_insert(WindowCounter {
        window_start: now,
        count: 0,
    });
    if elapsed >= window {
        counter.window_start = now;
        counter.count = 0;
    }
    if counter.count >= limit {
        let remaining = window.saturating_sub(elapsed);
        let seconds = remaining.as_millis().div_ceil(1000) as u64;
        return (false, seconds.max(1));
    }
    counter.count += 1;
    (true, 0)
}
