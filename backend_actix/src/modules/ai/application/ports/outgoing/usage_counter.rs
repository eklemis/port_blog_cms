//! Counting generations, per person, per period.

use async_trait::async_trait;
use uuid::Uuid;

/// Why a count could not be read or written.
#[derive(Debug, Clone, thiserror::Error)]
pub enum UsageCounterError {
    /// The counter store could not be reached.
    #[error("Usage counter unavailable: {0}")]
    Unavailable(String),
}

/// A per-person counter that resets each period.
///
/// Deliberately narrow: this only counts. Whether a count is over a limit is
/// policy, and lives in the service, so the policy can be tested without Redis.
#[async_trait]
pub trait UsageCounter: Send + Sync {
    /// Generations used so far this period. Zero when nothing is recorded.
    async fn used(&self, owner: Uuid, period: &str) -> Result<u32, UsageCounterError>;

    /// Records one generation and returns the new total.
    ///
    /// `expires_in_secs` sets the counter's lifetime on first write, so a
    /// period's key disappears on its own rather than accumulating a row per
    /// user per month forever.
    async fn record(
        &self,
        owner: Uuid,
        period: &str,
        expires_in_secs: u64,
    ) -> Result<u32, UsageCounterError>;
}
