use crate::modules::auth::application::ports::outgoing::token_blacklist_repository::TokenBlacklistRepository;
use async_trait::async_trait;
use redis::{AsyncCommands, Client};
use std::sync::Arc;

pub struct RedisTokenBlacklistRepository {
    client: Arc<Client>,
    expiration_seconds: u64,
}

impl RedisTokenBlacklistRepository {
    pub fn new(redis_url: &str, expiration_seconds: u64) -> Result<Self, String> {
        let client =
            Client::open(redis_url).map_err(|e| format!("Redis connection error: {}", e))?;

        Ok(Self {
            client: Arc::new(client),
            expiration_seconds,
        })
    }
}

#[async_trait]
impl TokenBlacklistRepository for RedisTokenBlacklistRepository {
    async fn blacklist_token(&self, token: &str) -> Result<(), String> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| format!("Redis connection error: {}", e))?;

        let key = format!("blacklisted_token:{}", token);
        let _: () = conn
            .set_ex(key, "1", self.expiration_seconds)
            .await
            .map_err(|e| format!("Failed to blacklist token: {}", e))?;

        Ok(())
    }

    async fn is_token_blacklisted(&self, token: &str) -> Result<bool, String> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| format!("Redis connection error: {}", e))?;

        let key = format!("blacklisted_token:{}", token);
        let exists: bool = conn
            .exists(key)
            .await
            .map_err(|e| format!("Failed to check token status: {}", e))?;

        Ok(exists)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    // UNIT TESTS

    // Create a simple mock implementation for unit testing
    #[derive(Clone)]
    struct MockRedisClient {
        should_fail: bool,
        blacklisted_tokens: Arc<std::sync::Mutex<Vec<String>>>,
    }

    impl MockRedisClient {
        fn new(should_fail: bool) -> Self {
            Self {
                should_fail,
                blacklisted_tokens: Arc::new(std::sync::Mutex::new(Vec::new())),
            }
        }

        // Simulate Redis connection
        async fn get_connection(&self) -> Result<MockRedisConnection, String> {
            if self.should_fail {
                return Err("Redis connection error: connection refused".to_string());
            }
            Ok(MockRedisConnection {
                blacklisted_tokens: self.blacklisted_tokens.clone(),
            })
        }
    }

    #[derive(Clone)]
    struct MockRedisConnection {
        blacklisted_tokens: Arc<std::sync::Mutex<Vec<String>>>,
    }

    impl MockRedisConnection {
        async fn set_ex(&mut self, key: String, _value: &str, _expiry: u64) -> Result<(), String> {
            // Extract token from key format "blacklisted_token:{token}"
            if let Some(token) = key.strip_prefix("blacklisted_token:") {
                let mut tokens = self.blacklisted_tokens.lock().unwrap();
                tokens.push(token.to_string());
            }
            Ok(())
        }

        async fn exists(&mut self, key: String) -> Result<bool, String> {
            // Extract token from key format "blacklisted_token:{token}"
            if let Some(token) = key.strip_prefix("blacklisted_token:") {
                let tokens = self.blacklisted_tokens.lock().unwrap();
                return Ok(tokens.contains(&token.to_string()));
            }
            Ok(false)
        }
    }

    // Create a testable repository that uses our mock
    struct TestRedisTokenRepository {
        client: MockRedisClient,
        expiration_seconds: u64,
    }

    #[async_trait]
    impl TokenBlacklistRepository for TestRedisTokenRepository {
        async fn blacklist_token(&self, token: &str) -> Result<(), String> {
            let mut conn = self.client.get_connection().await?;

            let key = format!("blacklisted_token:{}", token);
            conn.set_ex(key, "1", self.expiration_seconds).await?;

            Ok(())
        }

        async fn is_token_blacklisted(&self, token: &str) -> Result<bool, String> {
            let mut conn = self.client.get_connection().await?;

            let key = format!("blacklisted_token:{}", token);
            let exists = conn.exists(key).await?;

            Ok(exists)
        }
    }

    #[tokio::test]
    async fn test_unit_blacklist_token_success() {
        // Arrange
        let client = MockRedisClient::new(false); // No failure
        let repo = TestRedisTokenRepository {
            client,
            expiration_seconds: 3600,
        };

        // Act
        let result = repo.blacklist_token("test_token").await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_unit_blacklist_token_connection_error() {
        // Arrange
        let client = MockRedisClient::new(true); // Simulate failure
        let repo = TestRedisTokenRepository {
            client,
            expiration_seconds: 3600,
        };

        // Act
        let result = repo.blacklist_token("test_token").await;

        // Assert
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Redis connection error"));
    }

    #[tokio::test]
    async fn test_unit_is_token_blacklisted_true() {
        // Arrange
        let client = MockRedisClient::new(false); // No failure
        let repo = TestRedisTokenRepository {
            client: client.clone(),
            expiration_seconds: 3600,
        };

        // First blacklist a token
        let _ = repo.blacklist_token("test_token").await;

        // Act
        let result = repo.is_token_blacklisted("test_token").await;

        // Assert
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_unit_is_token_blacklisted_false() {
        // Arrange
        let client = MockRedisClient::new(false); // No failure
        let repo = TestRedisTokenRepository {
            client,
            expiration_seconds: 3600,
        };

        // Act - check for a token that wasn't blacklisted
        let result = repo.is_token_blacklisted("unknown_token").await;

        // Assert
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_unit_is_token_blacklisted_connection_error() {
        // Arrange
        let client = MockRedisClient::new(true); // Simulate failure
        let repo = TestRedisTokenRepository {
            client,
            expiration_seconds: 3600,
        };

        // Act
        let result = repo.is_token_blacklisted("test_token").await;

        // Assert
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Redis connection error"));
    }

    // INTEGRATION TESTS

    const TEST_REDIS_URL: &str = "redis://127.0.0.1/";

    // Helper function to create a test repository
    async fn create_test_repository() -> Result<RedisTokenBlacklistRepository, String> {
        let repo = RedisTokenBlacklistRepository::new(TEST_REDIS_URL, 3600)?;
        Ok(repo)
    }

    // Helper function to clean up Redis before tests
    async fn clean_redis(repo: &RedisTokenBlacklistRepository) -> Result<(), String> {
        match repo.client.get_multiplexed_async_connection().await {
            Ok(mut conn) => {
                let _: Result<(), redis::RedisError> =
                    redis::cmd("FLUSHDB").query_async(&mut conn).await;
                Ok(())
            }
            Err(e) => Err(format!("Redis connection error: {}", e)),
        }
    }

    #[tokio::test]
    async fn test_integration_constructor_with_valid_url() {
        let result = RedisTokenBlacklistRepository::new(TEST_REDIS_URL, 3600);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_integration_constructor_with_invalid_url() {
        let result = RedisTokenBlacklistRepository::new("invalid://url", 3600);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_integration_blacklist_token() {
        // This test requires a running Redis instance
        // If Redis is not available, the test will be skipped
        match create_test_repository().await {
            Ok(repo) => {
                if let Err(_) = clean_redis(&repo).await {
                    eprintln!("Could not clean Redis, skipping test");
                    return;
                }

                let token = "test_token";
                let result = repo.blacklist_token(token).await;
                assert!(result.is_ok());

                // Verify token is blacklisted
                let result = repo.is_token_blacklisted(token).await;
                assert!(result.is_ok());
                assert!(result.unwrap());
            }
            Err(e) => {
                eprintln!("Could not connect to Redis, skipping test: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_integration_is_token_blacklisted_when_not_blacklisted() {
        match create_test_repository().await {
            Ok(repo) => {
                if let Err(_) = clean_redis(&repo).await {
                    eprintln!("Could not clean Redis, skipping test");
                    return;
                }

                let token = "non_blacklisted_token";
                let result = repo.is_token_blacklisted(token).await;

                assert!(result.is_ok());
                assert!(!result.unwrap());
            }
            Err(e) => {
                eprintln!("Could not connect to Redis, skipping test: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_integration_token_expiration() {
        match RedisTokenBlacklistRepository::new(TEST_REDIS_URL, 1) {
            Ok(repo) => {
                if let Err(_) = clean_redis(&repo).await {
                    eprintln!("Could not clean Redis, skipping test");
                    return;
                }

                let token = "expiring_token";

                // Blacklist the token
                let _ = repo.blacklist_token(token).await;

                // Verify it's blacklisted initially
                let result = repo.is_token_blacklisted(token).await;
                assert!(result.is_ok());
                assert!(result.unwrap());

                // Wait for expiration (plus buffer)
                sleep(Duration::from_millis(1500)).await;

                // Verify it's no longer blacklisted
                let result = repo.is_token_blacklisted(token).await;
                assert!(result.is_ok());
                assert!(!result.unwrap());
            }
            Err(e) => {
                eprintln!("Could not connect to Redis, skipping test: {}", e);
            }
        }
    }
}
