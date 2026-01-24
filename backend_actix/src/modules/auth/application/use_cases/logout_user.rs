use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Deserializer, Serialize};
use tracing::{info, warn};

use crate::auth::application::ports::{
    outgoing::token_hasher::hash_token,
    outgoing::token_provider::TokenProvider,
    outgoing::token_repository::{TokenRepository, TokenRepositoryError},
};

// ========================= Logout Request =========================
#[derive(Debug, Clone)]
pub struct LogoutRequest {
    refresh_token: Option<String>,
}

impl LogoutRequest {
    pub fn new(refresh_token: Option<String>) -> Result<Self, LogoutRequestError> {
        Ok(Self {
            refresh_token: refresh_token.map(|t| t.trim().to_string()),
        })
    }

    pub fn refresh_token(&self) -> Option<&str> {
        self.refresh_token.as_deref()
    }
}

#[derive(Debug, Clone)]
pub enum LogoutRequestError {
    // For future validation
}

impl std::fmt::Display for LogoutRequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Logout request error")
    }
}

impl std::error::Error for LogoutRequestError {}

impl<'de> Deserialize<'de> for LogoutRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct LogoutRequestHelper {
            #[serde(default)]
            refresh_token: Option<String>,
        }

        let helper = LogoutRequestHelper::deserialize(deserializer)?;
        LogoutRequest::new(helper.refresh_token).map_err(serde::de::Error::custom)
    }
}

// ====================== Logout Response =============================
#[derive(Debug, Clone, Serialize)]
pub struct LogoutResponse {
    pub message: String,
}

// ====================== Logout Error =============================
#[derive(Debug, Clone)]
pub enum LogoutError {
    TokenRevocationFailed(String),
    DatabaseError(String),
}

impl std::fmt::Display for LogoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogoutError::TokenRevocationFailed(msg) => {
                write!(f, "Token revocation failed: {}", msg)
            }
            LogoutError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
        }
    }
}

impl std::error::Error for LogoutError {}

impl From<TokenRepositoryError> for LogoutError {
    fn from(error: TokenRepositoryError) -> Self {
        match error {
            TokenRepositoryError::DatabaseError(msg) => LogoutError::DatabaseError(msg),
            _ => LogoutError::TokenRevocationFailed(error.to_string()),
        }
    }
}

// ============================ Logout Use Case =============================
#[async_trait]
pub trait ILogoutUseCase: Send + Sync {
    async fn execute(&self, request: LogoutRequest) -> Result<LogoutResponse, LogoutError>;
}

#[derive(Clone)]
pub struct LogoutUseCase<R>
where
    R: TokenRepository + Send + Sync,
{
    token_repository: R,
    token_provider: Arc<dyn TokenProvider>,
}

impl<R> LogoutUseCase<R>
where
    R: TokenRepository + Send + Sync,
{
    pub fn new(token_repository: R, token_provider: Arc<dyn TokenProvider>) -> Self {
        Self {
            token_repository,
            token_provider,
        }
    }
}

#[async_trait]
impl<R> ILogoutUseCase for LogoutUseCase<R>
where
    R: TokenRepository + Send + Sync,
{
    async fn execute(&self, request: LogoutRequest) -> Result<LogoutResponse, LogoutError> {
        // If refresh token provided, blacklist it
        if let Some(refresh_token) = request.refresh_token() {
            match self.token_provider.verify_token(refresh_token) {
                Ok(claims) => {
                    // Hash the token before storing (NEVER store raw tokens!)
                    let token_hash = hash_token(refresh_token);

                    // Get token expiration from claims
                    let expires_at = chrono::DateTime::from_timestamp(claims.exp, 0)
                        .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::days(7));

                    // Blacklist the token
                    self.token_repository
                        .blacklist_token(token_hash, claims.sub, expires_at)
                        .await?;

                    info!("Token blacklisted for user: {}", claims.sub);
                }
                Err(e) => {
                    // Token invalid or expired - still return success
                    // Better UX: logout always succeeds from user perspective
                    warn!("Failed to verify token during logout: {}", e);
                }
            }
        }

        Ok(LogoutResponse {
            message: "Logged out successfully".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::auth::adapter::outgoing::jwt::{JwtConfig, JwtTokenService};
    use chrono::{DateTime, Utc};
    use uuid::Uuid;

    // Mock Token Repository
    #[derive(Default, Clone)]
    struct MockTokenRepository {
        blacklisted_tokens: std::sync::Arc<tokio::sync::Mutex<Vec<String>>>,
        should_fail: bool,
    }

    impl MockTokenRepository {
        fn new() -> Self {
            Self::default()
        }

        fn with_failure() -> Self {
            Self {
                blacklisted_tokens: std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new())),
                should_fail: true,
            }
        }

        async fn is_blacklisted(&self, token_hash: &str) -> bool {
            self.blacklisted_tokens
                .lock()
                .await
                .contains(&token_hash.to_string())
        }
    }

    #[async_trait]
    impl TokenRepository for MockTokenRepository {
        async fn blacklist_token(
            &self,
            token_hash: String,
            _user_id: Uuid,
            _expires_at: DateTime<Utc>,
        ) -> Result<(), TokenRepositoryError> {
            if self.should_fail {
                return Err(TokenRepositoryError::DatabaseError(
                    "Connection failed".to_string(),
                ));
            }

            self.blacklisted_tokens.lock().await.push(token_hash);
            Ok(())
        }

        async fn is_token_blacklisted(
            &self,
            token_hash: &str,
        ) -> Result<bool, TokenRepositoryError> {
            Ok(self.is_blacklisted(token_hash).await)
        }

        async fn remove_blacklisted_token(
            &self,
            _token_hash: &str,
        ) -> Result<(), TokenRepositoryError> {
            Ok(())
        }

        async fn revoke_all_user_tokens(&self, _user_id: Uuid) -> Result<(), TokenRepositoryError> {
            Ok(())
        }

        async fn cleanup_expired_tokens(&self) -> Result<u64, TokenRepositoryError> {
            Ok(0)
        }
    }

    fn create_jwt_service() -> JwtTokenService {
        JwtTokenService::new(JwtConfig {
            secret_key: "test_secret_key_min_32_characters_long".to_string(),
            issuer: "testapp".to_string(),
            access_token_expiry: 3600,
            refresh_token_expiry: 86400,
            verification_token_expiry: 86400,
        })
    }

    #[tokio::test]
    async fn test_logout_with_token_blacklisting() {
        let repository = MockTokenRepository::new();
        let jwt_service = create_jwt_service();
        let user_id = Uuid::new_v4();

        // Generate a valid refresh token
        let refresh_token = jwt_service.generate_refresh_token(user_id, true).unwrap();

        let use_case = LogoutUseCase::new(repository.clone(), Arc::new(jwt_service));
        let request = LogoutRequest::new(Some(refresh_token.clone())).unwrap();

        let result = use_case.execute(request).await;

        assert!(result.is_ok());

        // Verify token was blacklisted
        let token_hash = hash_token(&refresh_token);
        assert!(repository.is_blacklisted(&token_hash).await);
    }

    #[tokio::test]
    async fn test_logout_without_token() {
        let repository = MockTokenRepository::new();
        let jwt_service = create_jwt_service();

        let use_case = LogoutUseCase::new(repository, Arc::new(jwt_service));
        let request = LogoutRequest::new(None).unwrap();

        let result = use_case.execute(request).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_logout_with_invalid_token() {
        let repository = MockTokenRepository::new();
        let jwt_service = create_jwt_service();

        let use_case = LogoutUseCase::new(repository, Arc::new(jwt_service));
        let request = LogoutRequest::new(Some("invalid.token.here".to_string())).unwrap();

        // Should still succeed - logout always succeeds from user perspective
        let result = use_case.execute(request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_logout_repository_failure() {
        let repository = MockTokenRepository::with_failure();
        let jwt_service = create_jwt_service();
        let user_id = Uuid::new_v4();

        let refresh_token = jwt_service.generate_refresh_token(user_id, true).unwrap();

        let use_case = LogoutUseCase::new(repository, Arc::new(jwt_service));
        let request = LogoutRequest::new(Some(refresh_token)).unwrap();

        let result = use_case.execute(request).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LogoutError::DatabaseError(_)));
    }
}
