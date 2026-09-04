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

        // Only the write that created the key sets the expiry. Refreshing it
        // on every generation would push the reset further out each time
        // somebody used the product, so a heavy month would never end.
        if count == 1 {
            let _: () = conn
                .expire(&key, expires_in_secs as i64)
                .await
                .map_err(unavailable)?;
        }

        Ok(count as u32)
    }
}
