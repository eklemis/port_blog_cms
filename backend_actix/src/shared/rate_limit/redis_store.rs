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
