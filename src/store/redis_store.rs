//! Redis shared rate-limit adapter (spec-137).
//!
//! Enabled with `--features redis` and selected through `ION_OF_NOTES_REDIS_URL`.
//! The limiter uses one Redis counter per key and fixed window; INCR/EXPIRE run
//! in a small Lua script so the first hit always sets the TTL atomically.

use crate::store::{RateLimitHit, RateLimitStore};
use redis::Client;
use std::io;
use std::sync::{Arc, Mutex};

const WINDOW_SCRIPT: &str = r"
local count = redis.call('INCR', KEYS[1])
if count == 1 then
  redis.call('EXPIRE', KEYS[1], ARGV[1])
end
return count
";

/// Redis-backed fixed-window rate limiter.
pub struct RedisRateLimitStore {
    client: Client,
    connection: Arc<Mutex<redis::Connection>>,
}

impl RedisRateLimitStore {
    /// Connect to Redis.
    pub fn open(url: &str) -> io::Result<Self> {
        let client = Client::open(url).map_err(io::Error::other)?;
        let connection = client.get_connection().map_err(io::Error::other)?;
        Ok(Self {
            client,
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    fn connection(&self) -> std::sync::MutexGuard<'_, redis::Connection> {
        self.connection
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl RateLimitStore for RedisRateLimitStore {
    fn check(&self, key: &str, window_secs: u64, limit: u32, now: u64) -> io::Result<RateLimitHit> {
        if limit == 0 {
            return Ok(RateLimitHit {
                allowed: true,
                retry_after: 0,
            });
        }
        let window_secs = window_secs.max(1);
        let window_start = now - (now % window_secs);
        let redis_key = format!("ion:rate:{key}:{window_start}");
        let mut connection = self.connection();
        let count: u64 = redis::Script::new(WINDOW_SCRIPT)
            .key(redis_key)
            .arg(window_secs)
            .invoke(&mut *connection)
            .map_err(io::Error::other)?;
        if count > u64::from(limit) {
            Ok(RateLimitHit {
                allowed: false,
                retry_after: window_secs.saturating_sub(now - window_start).max(1),
            })
        } else {
            Ok(RateLimitHit {
                allowed: true,
                retry_after: 0,
            })
        }
    }
}

impl std::fmt::Debug for RedisRateLimitStore {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RedisRateLimitStore")
            .field("client", &self.client)
            .finish_non_exhaustive()
    }
}
