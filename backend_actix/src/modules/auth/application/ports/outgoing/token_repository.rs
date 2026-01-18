use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Errors that can occur in token repository operations
#[derive(Debug, Clone)]
pub enum TokenRepositoryError {
    DatabaseError(String),
    TokenNotFound,
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

/// Token repository port (interface)
#[async_trait]
pub trait TokenRepository: Send + Sync {
    /// Add a token to the blacklist
    async fn blacklist_token(
        &self,
        token_hash: String,
        user_id: Uuid,
        expires_at: DateTime<Utc>,
    ) -> Result<(), TokenRepositoryError>;

    /// Check if a token is blacklisted
    async fn is_token_blacklisted(&self, token_hash: &str) -> Result<bool, TokenRepositoryError>;

    /// Remove a specific blacklisted token (for cleanup)
    async fn remove_blacklisted_token(&self, token_hash: &str) -> Result<(), TokenRepositoryError>;

    /// Revoke all tokens for a user
    async fn revoke_all_user_tokens(&self, user_id: Uuid) -> Result<(), TokenRepositoryError>;

    /// Clean up expired tokens from blacklist
    async fn cleanup_expired_tokens(&self) -> Result<u64, TokenRepositoryError>;
}
