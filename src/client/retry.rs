//! Client-side retry policy for `429`/`529` and idempotent server errors
//! (spec-037). The transport owns the sleep; this type owns the decision, so
//! it is pure and testable without timers.

use crate::server::Method;

/// Client-side retry policy for `429`/`529` and idempotent server errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base_delay_secs: u64,
    pub max_delay_secs: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            base_delay_secs: 1,
            max_delay_secs: 30,
        }
    }
}

/// One retry decision: whether to retry and how long to wait.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryDecision {
    pub retry: bool,
    pub delay_millis: u64,
}

impl RetryDecision {
    fn stop() -> Self {
        Self {
            retry: false,
            delay_millis: 0,
        }
    }
}

impl RetryPolicy {
    /// Decide whether `status` should be retried and how long to wait.
    ///
    /// `retry_after` is the integer seconds from the response header; when it
    /// is absent the fallback is exponential backoff capped at
    /// `max_delay_secs`. `jitter_millis` is supplied by the caller so the
    /// policy stays deterministic in tests.
    pub fn decide(
        &self,
        method: Method,
        status: u16,
        code: Option<&str>,
        retry_after: Option<u64>,
        attempt: u32,
        jitter_millis: u64,
    ) -> RetryDecision {
        if attempt >= self.max_attempts {
            return RetryDecision::stop();
        }
        let idempotent = matches!(method, Method::Get | Method::Delete);
        let retryable = match status {
            429 => code != Some("public_api_request_blocked"),
            529 => true,
            500 | 502 | 503 | 504 => idempotent,
            _ => false,
        };
        if !retryable {
            return RetryDecision::stop();
        }
        let delay_millis = match retry_after {
            Some(seconds) => seconds.saturating_mul(1000),
            None => {
                let exponential = self
                    .base_delay_secs
                    .saturating_mul(1u64 << attempt.min(20))
                    .min(self.max_delay_secs);
                exponential.saturating_mul(1000)
            }
        };
        RetryDecision {
            retry: true,
            delay_millis: delay_millis.saturating_add(jitter_millis),
        }
    }
}
