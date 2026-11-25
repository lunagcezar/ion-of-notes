//! Redis shared rate limiter (opt-in, spec-137).
//!
//! Set `ION_OF_NOTES_REDIS_TEST_URL` to a disposable Redis instance. Without it
//! the test is a no-op.

#![cfg(feature = "redis")]

use ion_of_notes::store::{RateLimitStore, RedisRateLimitStore};

#[test]
fn redis_rate_limit_when_configured() {
    let Ok(url) = std::env::var("ION_OF_NOTES_REDIS_TEST_URL") else {
        return;
    };
    let store = RedisRateLimitStore::open(&url).expect("redis");
    let key = format!("test-{}", uuid::Uuid::new_v4());
    let window = 60u64;
    let now = 1_000u64;
    assert!(store.check(&key, window, 2, now).expect("first").allowed);
    assert!(store.check(&key, window, 2, now).expect("second").allowed);
    let denied = store.check(&key, window, 2, now).expect("third");
    assert!(!denied.allowed);
    assert!(denied.retry_after > 0);

    let client = redis::Client::open(url).expect("client");
    let mut connection = client.get_connection().expect("connection");
    let _: i64 = redis::cmd("DEL")
        .arg(format!("ion:rate:{key}:{now}"))
        .query(&mut connection)
        .expect("del");
}
