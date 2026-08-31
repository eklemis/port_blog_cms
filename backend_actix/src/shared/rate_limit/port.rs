use async_trait::async_trait;

/// Outcome of consuming one unit of quota.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimitDecision {
    pub allowed: bool,
    /// Requests still available in the current window.
    pub remaining: u32,
    /// Seconds until the window resets, for the `Retry-After` header.
    pub retry_after_secs: u64,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum RateLimitError {
    #[error("Rate limit store unavailable: {0}")]
    Unavailable(String),
}

/// A counter keyed by caller and route.
///
/// Deliberately narrow: the middleware decides *what* to count and the store
/// only counts, so the policy can be tested without Redis and the store can be
/// swapped for an in-memory one in tests.
#[async_trait]
pub trait RateLimitStore: Send + Sync {
    /// Records one request against `key` and reports whether it is allowed.
    ///
    /// Implementations must treat the first request in a window as the one that
    /// starts the clock, so a caller cannot keep a window alive indefinitely by
    /// continuing to hit a limit they have already exceeded.
    async fn consume(
        &self,
        key: &str,
        limit: u32,
        window_secs: u64,
    ) -> Result<RateLimitDecision, RateLimitError>;
}
