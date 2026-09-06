//! The Redis implementation of [`UsageCounter`].
//!
//! One key per person per period, holding a plain integer, expiring when the
//! period does. No history is kept: this answers "how much this month", and
//! keeping more would make a record of somebody's generation habits that
//! nothing needs.

use async_trait::async_trait;
use deadpool_redis::redis::AsyncCommands;
use deadpool_redis::Pool;
use std::sync::Arc;
use uuid::Uuid;

use crate::ai::application::ports::outgoing::{UsageCounter, UsageCounterError};

/// Counts generations in Redis.
#[derive(Clone)]
pub struct RedisUsageCounter {
    pool: Arc<Pool>,
}

impl RedisUsageCounter {
    /// Builds the counter from a Redis connection pool.
    pub fn new(pool: Arc<Pool>) -> Self {
        Self { pool }
    }

    fn key(owner: Uuid, period: &str) -> String {
        format!("ai:quota:{owner}:{period}")
    }
}

fn unavailable(e: impl std::fmt::Display) -> UsageCounterError {
    UsageCounterError::Unavailable(e.to_string())
}

#[async_trait]
impl UsageCounter for RedisUsageCounter {
    async fn used(&self, owner: Uuid, period: &str) -> Result<u32, UsageCounterError> {
        let mut conn = self.pool.get().await.map_err(unavailable)?;

        // A missing key is zero, not an error: nobody has a counter until
        // their first generation.
        let count: Option<u64> = conn
            .get(Self::key(owner, period))
            .await
            .map_err(unavailable)?;

        Ok(count.unwrap_or(0) as u32)
    }

    async fn record(
        &self,
        owner: Uuid,
        period: &str,
        expires_in_secs: u64,
    ) -> Result<u32, UsageCounterError> {
        let mut conn = self.pool.get().await.map_err(unavailable)?;
        let key = Self::key(owner, period);

        let count: u64 = conn.incr(&key, 1u64).await.map_err(unavailable)?;

        // The expiry is set whenever the key has none, not only on the write
        // that created it.
        //
        // `INCR` and `EXPIRE` are two commands, and the counter is created by
        // the first. Anything that stops the second — a dropped connection, a
        // restart, a Redis failover in the microseconds between them — leaves
        // a key that counts up and never expires. That person's allowance then
        // never resets: once they reach the limit they are refused
        // permanently, and no request can put it right, because the old code
        // only ever set the expiry when the count came back as 1.
        //
        // Asking for the TTL costs one cheap round trip beside a call that is
        // about to spend seconds in a model, and it repairs keys left without
        // an expiry by earlier versions rather than only avoiding new ones.
        // `-1` is Redis for "exists, no expiry"; `-2` is "gone", which a
        // concurrent expiry can produce between the two commands and which
        // setting a TTL on would resurrect.
        let ttl: i64 = conn.ttl(&key).await.map_err(unavailable)?;

        if ttl == -1 {
            // Refreshing an expiry that already exists would push the reset
            // further out each time somebody used the product, so a heavy
            // month would never end. Only a key with no expiry is touched.
            let _: () = conn
                .expire(&key, expires_in_secs as i64)
                .await
                .map_err(unavailable)?;
        }

        Ok(count as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use deadpool_redis::{Config, Runtime};

    /// Integration tests against a real Redis, gated the way the rate-limit
    /// store's are: skipped when `SKIP_REDIS_TESTS=1` or `REDIS_URL` is unset.
    /// The INCR / TTL / EXPIRE sequence cannot be checked any other way — a
    /// fake would reimplement the very commands under test.
    static TLS_INIT: std::sync::Once = std::sync::Once::new();

    /// rustls refuses to build a TLS connection until a crypto provider is
    /// installed, and Upstash requires TLS.
    fn init_tls() {
        TLS_INIT.call_once(|| {
            let _ = rustls::crypto::ring::default_provider().install_default();
        });
    }

    fn counter() -> Option<RedisUsageCounter> {
        if std::env::var("SKIP_REDIS_TESTS").is_ok_and(|v| v == "1") {
            eprintln!("SKIP_REDIS_TESTS=1; skipping Redis quota tests.");
            return None;
        }
        let url = std::env::var("REDIS_URL").ok()?;

        // Only once there is something to connect to. Installing a
        // process-wide provider on the skip path would change what every other
        // test module sees for no benefit.
        init_tls();

        let pool = Config::from_url(&url)
            .create_pool(Some(Runtime::Tokio1))
            .expect("redis pool");
        Some(RedisUsageCounter::new(Arc::new(pool)))
    }

    /// A fresh owner per run, so repeats and parallel CI never share counters.
    fn owner() -> Uuid {
        Uuid::new_v4()
    }

    async fn ttl_of(counter: &RedisUsageCounter, owner: Uuid, period: &str) -> i64 {
        let mut conn = counter.pool.get().await.expect("redis conn");
        conn.ttl(RedisUsageCounter::key(owner, period))
            .await
            .expect("ttl")
    }

    #[tokio::test]
    async fn the_first_generation_gives_the_counter_an_expiry() {
        let Some(counter) = counter() else { return };
        let (owner, period) = (owner(), "2099-01");

        assert_eq!(counter.record(owner, period, 3600).await.unwrap(), 1);

        let ttl = ttl_of(&counter, owner, period).await;
        assert!(ttl > 0, "a fresh counter must expire, got TTL {ttl}");
    }

    /// The regression this method exists to prevent.
    ///
    /// `INCR` creates the key and `EXPIRE` is a second command; anything that
    /// stops the second leaves a counter that never resets, and the person it
    /// belongs to is refused permanently once they reach the limit. A key found
    /// without an expiry must be repaired rather than counted on forever.
    #[tokio::test]
    async fn a_counter_left_without_an_expiry_is_repaired() {
        let Some(counter) = counter() else { return };
        let (owner, period) = (owner(), "2099-02");

        // Exactly what the old code left behind when it died after the INCR.
        {
            let mut conn = counter.pool.get().await.expect("redis conn");
            let _: i64 = conn
                .incr(RedisUsageCounter::key(owner, period), 1i64)
                .await
                .expect("incr");
        }
        assert_eq!(
            ttl_of(&counter, owner, period).await,
            -1,
            "the broken state this test is about"
        );

        counter.record(owner, period, 3600).await.unwrap();

        let ttl = ttl_of(&counter, owner, period).await;
        assert!(
            ttl > 0,
            "the missing expiry should have been set, got {ttl}"
        );
    }

    /// Refreshing an expiry on every generation would push the reset further
    /// out each time somebody used the product, so a heavy month never ends.
    #[tokio::test]
    async fn a_later_generation_does_not_push_the_reset_further_out() {
        let Some(counter) = counter() else { return };
        let (owner, period) = (owner(), "2099-03");

        counter.record(owner, period, 3600).await.unwrap();
        let first = ttl_of(&counter, owner, period).await;

        // A much longer window on the second call must not be adopted.
        counter.record(owner, period, 99_999).await.unwrap();
        let second = ttl_of(&counter, owner, period).await;

        assert!(
            second <= first,
            "the reset moved out: {first} then {second}"
        );
    }
}
