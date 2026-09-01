use async_trait::async_trait;
use deadpool_redis::redis::AsyncCommands;
use deadpool_redis::Pool;
use std::sync::Arc;

use super::port::{RateLimitDecision, RateLimitError, RateLimitStore};

/// Fixed-window counter in Redis.
///
/// One `INCR` per request; the first increment in a window also sets the TTL.
/// A fixed window admits up to 2x the limit across a boundary — a caller can
/// spend a full window at the end of one and again at the start of the next.
/// That is acceptable here: the goal is to make credential stuffing and
/// repeated Argon2 hashing expensive, not to enforce an exact rate.
#[derive(Clone)]
pub struct RedisRateLimitStore {
    pool: Arc<Pool>,
}

impl RedisRateLimitStore {
    pub fn new(pool: Arc<Pool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RateLimitStore for RedisRateLimitStore {
    async fn consume(
        &self,
        key: &str,
        limit: u32,
        window_secs: u64,
    ) -> Result<RateLimitDecision, RateLimitError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| RateLimitError::Unavailable(e.to_string()))?;

        let redis_key = format!("ratelimit:{key}");

        let count: u64 = conn
            .incr(&redis_key, 1u64)
            .await
            .map_err(|e| RateLimitError::Unavailable(e.to_string()))?;

        // Only the request that created the key sets the expiry. Refreshing it
        // on every request would let a caller who keeps hitting a limit hold the
        // window open forever and never recover.
        if count == 1 {
            let _: () = conn
                .expire(&redis_key, window_secs as i64)
                .await
                .map_err(|e| RateLimitError::Unavailable(e.to_string()))?;
        }

        let ttl: i64 = conn
            .ttl(&redis_key)
            .await
            .map_err(|e| RateLimitError::Unavailable(e.to_string()))?;

        // TTL is -1 when the key has no expiry and -2 when it is gone; both mean
        // we cannot say when the window ends, so fall back to a full window.
        let retry_after_secs = if ttl > 0 { ttl as u64 } else { window_secs };

        Ok(RateLimitDecision {
            allowed: count <= limit as u64,
            remaining: (limit as u64).saturating_sub(count) as u32,
            retry_after_secs,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::rate_limit::port::RateLimitStore;
    use deadpool_redis::{Config, Runtime};
    use uuid::Uuid;

    /// Integration tests against a real Redis.
    ///
    /// Gated the same way as `token_repository_redis`: skipped when
    /// `SKIP_REDIS_TESTS=1` or `REDIS_URL` is unset. These are the only tests
    /// that exercise the INCR / EXPIRE / TTL sequence — the in-memory store used
    /// by the middleware tests reimplements the counting, so it cannot catch a
    /// mistake in the Redis commands themselves.
    static TLS_INIT: std::sync::Once = std::sync::Once::new();

    /// rustls 0.23 refuses to build a TLS connection until a crypto provider is
    /// installed, and Upstash requires TLS. `main.rs` does this at startup;
    /// tests have to do it themselves or every rediss:// connection panics
    /// inside rustls rather than failing as a connection error.
    fn init_tls() {
        TLS_INIT.call_once(|| {
            let _ = rustls::crypto::ring::default_provider().install_default();
        });
    }

    fn store() -> Option<RedisRateLimitStore> {
        init_tls();

        if std::env::var("SKIP_REDIS_TESTS").is_ok_and(|v| v == "1") {
            eprintln!("SKIP_REDIS_TESTS=1; skipping Redis rate-limit tests.");
            return None;
        }
        let url = match std::env::var("REDIS_URL") {
            Ok(v) => v,
            Err(_) => {
                eprintln!("REDIS_URL not set; skipping Redis rate-limit tests.");
                return None;
            }
        };
        let pool = Config::from_url(&url)
            .create_pool(Some(Runtime::Tokio1))
            .expect("redis pool");
        Some(RedisRateLimitStore::new(Arc::new(pool)))
    }

    /// A fresh key per test run, so repeated runs and parallel CI do not share
    /// counters.
    fn key(name: &str) -> String {
        format!("test:{name}:{}", Uuid::new_v4())
    }

    #[tokio::test]
    async fn counts_requests_and_blocks_past_the_limit() {
        let Some(s) = store() else { return };
        let k = key("limit");

        for i in 1..=3 {
            let d = s.consume(&k, 3, 60).await.unwrap();
            assert!(d.allowed, "request {i} should be allowed");
            assert_eq!(d.remaining, 3 - i);
        }

        let d = s.consume(&k, 3, 60).await.unwrap();
        assert!(!d.allowed);
        assert_eq!(d.remaining, 0, "remaining saturates rather than underflowing");
    }

    /// The TTL is set only by the request that creates the key. If it were
    /// refreshed on every request, a caller who keeps hitting the limit would
    /// hold their own window open forever and never recover.
    ///
    /// The assertion has to be strict and separated in time. An earlier version
    /// compared `<=` with no delay, which passes either way: with the bug the
    /// TTL is reset to the full window each call, so it stays *equal* rather
    /// than growing. Sleeping first means a correct implementation shows a
    /// decayed TTL, while a refreshing one snaps back to the maximum.
    #[tokio::test]
    async fn the_window_does_not_slide_when_the_limit_is_exceeded() {
        let Some(s) = store() else { return };
        let k = key("ttl");
        let window = 30;

        let first = s.consume(&k, 1, window).await.unwrap();
        assert!(first.allowed);

        tokio::time::sleep(std::time::Duration::from_millis(3_000)).await;

        // Exceed the limit; the window must keep counting down from the first
        // request, not restart.
        let after = s.consume(&k, 1, window).await.unwrap();
        assert!(!after.allowed);
        assert!(
            after.retry_after_secs < window,
            "TTL is still {} of {window}s after a 3s wait — the window slid",
            after.retry_after_secs
        );
    }

    #[tokio::test]
    async fn separate_keys_hold_separate_counters() {
        let Some(s) = store() else { return };
        let (a, b) = (key("a"), key("b"));

        for _ in 0..5 {
            s.consume(&a, 2, 60).await.unwrap();
        }
        assert!(!s.consume(&a, 2, 60).await.unwrap().allowed);

        // A different caller is unaffected.
        assert!(s.consume(&b, 2, 60).await.unwrap().allowed);
    }

    /// A one-second window lets the reset be observed without a slow test.
    #[tokio::test]
    async fn the_counter_resets_after_the_window_expires() {
        let Some(s) = store() else { return };
        let k = key("expiry");

        assert!(s.consume(&k, 1, 1).await.unwrap().allowed);
        assert!(!s.consume(&k, 1, 1).await.unwrap().allowed);

        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

        assert!(
            s.consume(&k, 1, 1).await.unwrap().allowed,
            "the key should have expired and the counter restarted"
        );
    }

    #[tokio::test]
    async fn an_unreachable_redis_reports_unavailable_rather_than_panicking() {
        // Points at a port nothing listens on, so this needs no live Redis and
        // runs even when the others skip.
        let pool = Config::from_url("redis://127.0.0.1:1")
            .create_pool(Some(Runtime::Tokio1))
            .expect("pool config is valid even when the server is absent");
        let s = RedisRateLimitStore::new(Arc::new(pool));

        let err = s.consume("k", 1, 60).await.unwrap_err();
        assert!(matches!(err, RateLimitError::Unavailable(_)));
    }
}
