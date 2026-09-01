//! Refresh-token blacklist.
//!
//! JWTs are stateless, so a token stays cryptographically valid until it
//! expires. Logout therefore cannot invalidate one — it can only record that
//! the token must be refused. This port is that record.
//!
//! Tokens are stored as SHA-256 hashes, never in the clear: the store would
//! otherwise be a pile of working credentials. See
//! [`hash_token`](super::token_hasher::hash_token).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Why a blacklist operation failed.
#[derive(Debug, Clone)]
pub enum TokenRepositoryError {
    /// The backing store — Redis in production — could not be reached.
    ///
    /// Callers must decide deliberately how to treat this. Failing open lets a
    /// logged-out token be accepted during an outage; failing closed turns a
    /// cache outage into a total authentication outage.
    DatabaseError(String),

    /// No blacklist entry matched the hash. Only meaningful for removal.
    TokenNotFound,

    /// The value supplied was not a usable token hash.
    InvalidToken,
}

impl std::fmt::Display for TokenRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenRepositoryError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            TokenRepositoryError::TokenNotFound => write!(f, "Token not found"),
            TokenRepositoryError::InvalidToken => write!(f, "Invalid token"),
        }
    }
}

impl std::error::Error for TokenRepositoryError {}

/// Records which refresh tokens must no longer be accepted.
///
/// Every method takes a **hash** of a token, never the token itself.
#[async_trait]
pub trait TokenRepository: Send + Sync {
    /// Blacklists a token until `expires_at`.
    ///
    /// `expires_at` should be the token's own expiry: past that point the
    /// token is refused on its own merits and the entry is dead weight, which
    /// is what lets implementations expire entries rather than growing without
    /// bound. Blacklisting the same hash twice is not an error.
    async fn blacklist_token(
        &self,
        token_hash: String,
        user_id: Uuid,
        expires_at: DateTime<Utc>,
    ) -> Result<(), TokenRepositoryError>;

    /// Reports whether a token has been blacklisted.
    ///
    /// `Ok(false)` means "not blacklisted", which is the common path and not an
    /// error. This is called on every refresh, so implementations should keep
    /// it cheap.
    async fn is_token_blacklisted(&self, token_hash: &str) -> Result<bool, TokenRepositoryError>;

    /// Drops a single blacklist entry, letting the token be accepted again if
    /// it has not expired. Maintenance only — no request path calls this.
    async fn remove_blacklisted_token(&self, token_hash: &str) -> Result<(), TokenRepositoryError>;

    /// Revokes every outstanding token for one user.
    ///
    /// The lever for "sign out everywhere" and for locking out an account
    /// after a password change or compromise.
    async fn revoke_all_user_tokens(&self, user_id: Uuid) -> Result<(), TokenRepositoryError>;

    /// Drops entries whose tokens have expired, returning how many were
    /// removed.
    ///
    /// Only needed where the store does not expire entries itself. The Redis
    /// implementation sets a TTL per entry, so this is a no-op safety valve
    /// there rather than a required sweep.
    async fn cleanup_expired_tokens(&self) -> Result<u64, TokenRepositoryError>;
}
