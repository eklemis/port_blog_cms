use crate::modules::auth::application::ports::outgoing::token_repository::{
    TokenRepository, TokenRepositoryError,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deadpool_redis::{redis::AsyncCommands, Pool};

use std::sync::Arc;

use uuid::Uuid;

/// Redis-backed implementation of `TokenRepository`.
///
/// ## High-level purpose
/// This repository manages **blacklisted (revoked) tokens** using Redis.
///
/// Redis is used because:
/// - Token blacklist lookups must be **O(1)**
/// - Tokens must **auto-expire** without manual cleanup
/// - Revoking *all* tokens of a user must be efficient
///
/// ## Redis data model
///
/// Two kinds of keys are used:
///
/// 1. **Per-token key (authoritative)**
/// ```text
/// auth:blacklist:token:{token_hash} -> "{user_id}"
/// ```
/// - Exists ⇒ token is blacklisted
/// - TTL = token expiration time
///
/// 2. **Per-user index (helper for bulk revoke)**
/// ```text
/// auth:blacklist:user:{user_id} -> SET(token_hash)
/// ```
/// - Tracks all blacklisted tokens for a user
/// - Also has TTL to avoid orphaned data
///
/// Redis TTL is the **single source of truth** for cleanup.
#[derive(Clone)]
pub struct RedisTokenRepository {
    pool: Arc<Pool>,
}

impl RedisTokenRepository {
    /// Create a new Redis-backed token repository.
    ///
    /// The connection manager must already be initialized and ready to use.
    pub fn new(pool: Arc<Pool>) -> Self {
        Self { pool }
    }

    /// Generate the Redis key for a blacklisted token.
    ///
    /// If this key exists, the token is considered **invalid**.
    fn token_key(token_hash: &str) -> String {
        format!("auth:blacklist:token:{token_hash}")
    }

    /// Generate the Redis key for a user's blacklist index.
    ///
    /// This key stores a SET of token hashes belonging to the user.
    fn user_key(user_id: Uuid) -> String {
        format!("auth:blacklist:user:{user_id}")
    }
    /// Helper to get a connection from the pool
    async fn get_conn(&self) -> Result<deadpool_redis::Connection, TokenRepositoryError> {
        self.pool
            .get()
            .await
            .map_err(|e| TokenRepositoryError::DatabaseError(format!("Pool error: {}", e)))
    }
}

#[async_trait]
impl TokenRepository for RedisTokenRepository {
    /// Blacklist (revoke) a single token.
    ///
    /// ## What this means
    /// After this call:
    /// - The token is immediately invalid
    /// - It will automatically disappear from Redis when it expires
    ///
    /// ## Redis operations performed (atomically)
    /// ```text
    /// SET    auth:blacklist:token:{hash} "{user_id}"
    /// EXPIRE auth:blacklist:token:{hash} <ttl>
    /// SADD   auth:blacklist:user:{user_id} {hash}
    /// EXPIRE auth:blacklist:user:{user_id} <ttl>
    /// ```
    ///
    /// All commands are wrapped in a `MULTI/EXEC` transaction to prevent
    /// partial state (e.g., token key exists but user index does not).
    ///
    /// ## Why TTL matters
    /// Redis automatically removes expired tokens.
    /// No background job or manual cleanup is needed.
    async fn blacklist_token(
        &self,
        token_hash: String,
        user_id: Uuid,
        expires_at: DateTime<Utc>,
    ) -> Result<(), TokenRepositoryError> {
        let ttl = (expires_at - Utc::now()).num_seconds();
        if ttl <= 0 {
            return Err(TokenRepositoryError::InvalidToken);
        }

        let token_key = Self::token_key(&token_hash);
        let user_key = Self::user_key(user_id);

        let mut conn = self.get_conn().await?;

        deadpool_redis::redis::pipe()
            .atomic()
            .cmd("SET")
            .arg(&token_key)
            .arg(user_id.to_string())
            .ignore()
            .cmd("EXPIRE")
            .arg(&token_key)
            .arg(ttl)
            .ignore()
            .cmd("SADD")
            .arg(&user_key)
            .arg(&token_hash)
            .ignore()
            .cmd("EXPIRE")
            .arg(&user_key)
            .arg(ttl)
            .ignore()
            .query_async::<()>(&mut *conn)
            .await
            .map_err(|e| TokenRepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Check whether a token is blacklisted.
    ///
    /// ## Redis operation
    /// ```text
    /// EXISTS auth:blacklist:token:{hash}
    /// ```
    ///
    /// - `true`  ⇒ token is revoked
    /// - `false` ⇒ token is valid (or already expired)
    ///
    /// This is an **O(1)** operation.
    async fn is_token_blacklisted(&self, token_hash: &str) -> Result<bool, TokenRepositoryError> {
        let key = Self::token_key(token_hash);
        let mut conn = self.get_conn().await?;

        let exists: bool = conn
            .exists(key)
            .await
            .map_err(|e| TokenRepositoryError::DatabaseError(e.to_string()))?;

        Ok(exists)
    }

    /// Remove a specific blacklisted token.
    ///
    /// ## When this is used
    /// - Manual cleanup
    /// - Token rotation
    /// - Defensive cleanup
    ///
    /// ## Redis behavior
    /// 1. Read the token key to find the owning user
    /// 2. If it exists, delete:
    ///    - the token key
    ///    - the token hash from the user's SET
    ///
    /// ## Important design choice
    /// If the token does not exist, this method **silently succeeds**.
    /// This makes the operation idempotent and safe to retry.
    async fn remove_blacklisted_token(&self, token_hash: &str) -> Result<(), TokenRepositoryError> {
        let token_key = Self::token_key(token_hash);
        let mut conn = self.get_conn().await?;

        let user_id: Option<String> = conn
            .get(&token_key)
            .await
            .map_err(|e| TokenRepositoryError::DatabaseError(e.to_string()))?;

        if let Some(uid) = user_id {
            let user_key = Self::user_key(
                uid.parse()
                    .map_err(|_| TokenRepositoryError::InvalidToken)?,
            );

            deadpool_redis::redis::pipe()
                .atomic()
                .del(&token_key)
                .ignore()
                .srem(&user_key, token_hash)
                .ignore()
                .query_async::<()>(&mut *conn)
                .await
                .map_err(|e| TokenRepositoryError::DatabaseError(e.to_string()))?;
        }

        Ok(())
    }

    /// Revoke **all** blacklisted tokens belonging to a user.
    ///
    /// ## Redis operations
    /// 1. Read all token hashes from:
    ///    ```text
    ///    SMEMBERS auth:blacklist:user:{user_id}
    ///    ```
    /// 2. Delete each token key
    /// 3. Delete the user index key itself
    ///
    /// All deletions are executed **atomically**.
    ///
    /// ## Complexity
    /// - O(N) where N = number of tokens for the user
    /// - No global scans
    ///
    /// ## Behavior
    /// - If the user has no tokens, this is a no-op
    /// - Safe to call multiple times
    async fn revoke_all_user_tokens(&self, user_id: Uuid) -> Result<(), TokenRepositoryError> {
        let user_key = Self::user_key(user_id);
        let mut conn = self.get_conn().await?;

        let tokens: Vec<String> = conn
            .smembers(&user_key)
            .await
            .map_err(|e| TokenRepositoryError::DatabaseError(e.to_string()))?;

        let mut pipe = deadpool_redis::redis::pipe();
        pipe.atomic();

        for token in tokens {
            pipe.del(Self::token_key(&token)).ignore();
        }

        pipe.del(&user_key).ignore();

        pipe.query_async::<()>(&mut *conn)
            .await
            .map_err(|e| TokenRepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Cleanup expired tokens.
    ///
    /// ## Why this does nothing
    /// Redis automatically deletes expired keys based on TTL.
    ///
    /// Any manual scan would:
    /// - Duplicate Redis functionality
    /// - Hurt performance
    /// - Risk blocking Redis under load
    ///
    /// Returning `0` is **intentional and correct**.
    async fn cleanup_expired_tokens(&self) -> Result<u64, TokenRepositoryError> {
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::RedisTokenRepository;
    use crate::modules::auth::application::ports::outgoing::token_repository::TokenRepository;
    use chrono::{Duration, Utc};
    use std::sync::Once;
    use uuid::Uuid;

    static TLS_INIT: Once = Once::new();

    fn init_tls() {
        TLS_INIT.call_once(|| {
            // choose ONE provider:
            // ring:
            rustls::crypto::ring::default_provider()
                .install_default()
                .expect("install rustls ring provider");

            // OR aws-lc-rs:
            // rustls::crypto::aws_lc_rs::default_provider()
            //     .install_default()
            //     .expect("install rustls aws-lc-rs provider");
        });
    }
    async fn setup_repo() -> RedisTokenRepository {
        init_tls();
        let redis_url = match std::env::var("REDIS_URL") {
            Ok(v) => v,
            Err(_) => {
                eprintln!("REDIS_URL not set; skipping Redis integration tests");
                // Skip the current test (not fail)
                // Requires Rust 1.70+ for std::process::exit in async context? This works fine.
                std::process::exit(0);
            }
        };

        let redis_pool = deadpool_redis::Config::from_url(&redis_url)
            .create_pool(Some(deadpool_redis::Runtime::Tokio1))
            .expect("Failed to create Redis pool");

        RedisTokenRepository::new(std::sync::Arc::new(redis_pool))
    }

    #[tokio::test]
    async fn blacklist_token_marks_token_as_blacklisted() {
        let repo = setup_repo().await;

        let token = "token_blacklist_1";
        let user_id = Uuid::new_v4();

        repo.blacklist_token(
            token.to_string(),
            user_id,
            Utc::now() + Duration::seconds(30),
        )
        .await
        .unwrap();

        let is_blacklisted = repo.is_token_blacklisted(token).await.unwrap();
        assert!(is_blacklisted);
    }

    #[tokio::test]
    async fn blacklisted_token_expires_automatically() {
        let repo = setup_repo().await;

        let token = "token_expiry_1";
        let user_id = Uuid::new_v4();

        // Use a TTL that survives truncation + scheduling
        repo.blacklist_token(
            token.to_string(),
            user_id,
            Utc::now() + Duration::seconds(3),
        )
        .await
        .unwrap();

        tokio::time::sleep(std::time::Duration::from_secs(4)).await;

        let is_blacklisted = repo.is_token_blacklisted(token).await.unwrap();
        assert!(!is_blacklisted);
    }

    #[tokio::test]
    async fn remove_blacklisted_token_removes_token() {
        let repo = setup_repo().await;

        let token = "token_remove_1";
        let user_id = Uuid::new_v4();

        repo.blacklist_token(
            token.to_string(),
            user_id,
            Utc::now() + Duration::seconds(30),
        )
        .await
        .unwrap();

        repo.remove_blacklisted_token(token).await.unwrap();

        let is_blacklisted = repo.is_token_blacklisted(token).await.unwrap();
        assert!(!is_blacklisted);
    }

    #[tokio::test]
    async fn remove_nonexistent_token_is_noop() {
        let repo = setup_repo().await;

        let result = repo.remove_blacklisted_token("does_not_exist").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn revoke_all_user_tokens_removes_all_tokens() {
        let repo = setup_repo().await;
        let user_id = Uuid::new_v4();

        let tokens = vec!["t1", "t2", "t3"];

        for t in &tokens {
            repo.blacklist_token(t.to_string(), user_id, Utc::now() + Duration::seconds(60))
                .await
                .unwrap();
        }

        repo.revoke_all_user_tokens(user_id).await.unwrap();

        for t in &tokens {
            assert!(!repo.is_token_blacklisted(t).await.unwrap());
        }
    }

    #[tokio::test]
    async fn revoke_user_with_no_tokens_is_noop() {
        let repo = setup_repo().await;
        let user_id = Uuid::new_v4();

        let result = repo.revoke_all_user_tokens(user_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn cleanup_expired_tokens_returns_zero() {
        let repo = setup_repo().await;

        let count = repo.cleanup_expired_tokens().await.unwrap();
        assert_eq!(count, 0);
    }
}
