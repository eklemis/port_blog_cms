use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Deserializer, Serialize};

use crate::auth::application::ports::outgoing::token_provider::{TokenError, TokenProvider};

// ========================= Refresh Token Request =========================
/// Validated refresh token request
#[derive(Debug, Clone)]
pub struct RefreshTokenRequest {
    refresh_token: String, // Private - guaranteed non-empty
}

#[derive(Debug, Clone)]
pub enum RefreshTokenRequestError {
    EmptyToken,
}

impl std::fmt::Display for RefreshTokenRequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RefreshTokenRequestError::EmptyToken => write!(f, "Refresh token cannot be empty"),
        }
    }
}

impl std::error::Error for RefreshTokenRequestError {}

impl RefreshTokenRequest {
    /// Create a validated RefreshTokenRequest
    pub fn new(refresh_token: String) -> Result<Self, RefreshTokenRequestError> {
        // Validate token is not empty
        if refresh_token.trim().is_empty() {
            return Err(RefreshTokenRequestError::EmptyToken);
        }

        Ok(Self {
            refresh_token: refresh_token.trim().to_string(),
        })
    }

    /// Get refresh token (guaranteed to be non-empty)
    pub fn refresh_token(&self) -> &str {
        &self.refresh_token
    }
}

// Custom deserialization that validates during parsing
impl<'de> Deserialize<'de> for RefreshTokenRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RefreshTokenRequestHelper {
            refresh_token: String,
        }

        let helper = RefreshTokenRequestHelper::deserialize(deserializer)?;
        RefreshTokenRequest::new(helper.refresh_token).map_err(serde::de::Error::custom)
    }
}

// ====================== Refresh Token Error =============================
#[derive(Debug, Clone)]
pub enum RefreshTokenError {
    TokenExpired,
    TokenInvalid,
    TokenNotYetValid,
    InvalidTokenType,
    InvalidSignature,
    TokenGenerationFailed(String),
}

impl std::fmt::Display for RefreshTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RefreshTokenError::TokenExpired => write!(f, "Refresh token has expired"),
            RefreshTokenError::TokenInvalid => write!(f, "Invalid refresh token"),
            RefreshTokenError::TokenNotYetValid => write!(f, "Token is not yet valid"),
            RefreshTokenError::InvalidTokenType => write!(f, "Invalid token type"),
            RefreshTokenError::InvalidSignature => write!(f, "Invalid token signature"),
            RefreshTokenError::TokenGenerationFailed(msg) => {
                write!(f, "Token generation failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for RefreshTokenError {}

// Convert JwtError to RefreshTokenError
impl From<TokenError> for RefreshTokenError {
    fn from(error: TokenError) -> Self {
        match error {
            TokenError::TokenExpired => RefreshTokenError::TokenExpired,
            TokenError::TokenNotYetValid => RefreshTokenError::TokenNotYetValid,
            TokenError::InvalidTokenType(_) => RefreshTokenError::InvalidTokenType,
            TokenError::InvalidSignature => RefreshTokenError::InvalidSignature,
            TokenError::MalformedToken => RefreshTokenError::TokenInvalid,
            TokenError::EncodingError(msg) => RefreshTokenError::TokenGenerationFailed(msg),
        }
    }
}

// ============================ Refresh Token Response =========================
#[derive(Debug, Clone, Serialize)]
pub struct RefreshTokenResponse {
    pub access_token: String,
    pub refresh_token: String, // Optional: return new refresh token (token rotation)
}

// ============================ Refresh Token Use Case =============================
/// Interface for Refresh Token use case
#[async_trait]
pub trait IRefreshTokenUseCase: Send + Sync {
    async fn execute(
        &self,
        request: RefreshTokenRequest,
    ) -> Result<RefreshTokenResponse, RefreshTokenError>;
}

/// Implementation of Refresh Token use case
#[derive(Clone)]
pub struct RefreshTokenUseCase {
    token_provider: Arc<dyn TokenProvider>,
    enable_token_rotation: bool, // Feature flag for token rotation
}

impl RefreshTokenUseCase {
    pub fn new(token_provider: Arc<dyn TokenProvider>) -> Self {
        Self {
            token_provider,
            enable_token_rotation: true, // Enable token rotation by default
        }
    }

    pub fn with_token_rotation(mut self, enable: bool) -> Self {
        self.enable_token_rotation = enable;
        self
    }
}

#[async_trait]
impl IRefreshTokenUseCase for RefreshTokenUseCase {
    async fn execute(
        &self,
        request: RefreshTokenRequest,
    ) -> Result<RefreshTokenResponse, RefreshTokenError> {
        // 1️⃣ Verify and decode refresh token
        let claims = self
            .token_provider
            .verify_token(request.refresh_token())
            .map_err(RefreshTokenError::from)?;

        // 2️⃣ Ensure it's a refresh token (not access or verification token)
        if claims.token_type != "refresh" {
            return Err(RefreshTokenError::InvalidTokenType);
        }

        // 3️⃣ Generate new access token
        let access_token = self
            .token_provider
            .generate_access_token(claims.sub, claims.is_verified)
            .map_err(|e| RefreshTokenError::TokenGenerationFailed(e.to_string()))?;

        // 4️⃣ Optionally generate new refresh token (token rotation)
        let refresh_token = if self.enable_token_rotation {
            self.token_provider
                .generate_refresh_token(claims.sub, claims.is_verified)
                .map_err(|e| RefreshTokenError::TokenGenerationFailed(e.to_string()))?
        } else {
            // Return the same refresh token
            request.refresh_token().to_string()
        };

        // 5️⃣ Return response
        Ok(RefreshTokenResponse {
            access_token,
            refresh_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::auth::adapter::outgoing::jwt::{JwtConfig, JwtTokenService};
    use serde_json::json;
    use uuid::Uuid;

    // Helper to create JWT service
    fn create_jwt_service() -> JwtTokenService {
        JwtTokenService::new(JwtConfig {
            secret_key: "test_secret_key_min_32_characters_long".to_string(),
            issuer: "testapp".to_string(),
            access_token_expiry: 3600,
            refresh_token_expiry: 86400,
            verification_token_expiry: 86400,
        })
    }

    // ==================== RefreshTokenRequest Tests ====================
    #[test]
    fn test_refresh_token_request_valid() {
        let request = RefreshTokenRequest::new("valid_token_123".to_string());
        assert!(request.is_ok());
        assert_eq!(request.unwrap().refresh_token(), "valid_token_123");
    }

    #[test]
    fn test_refresh_token_request_trimmed() {
        let request = RefreshTokenRequest::new("  token_123  ".to_string()).unwrap();
        assert_eq!(request.refresh_token(), "token_123");
    }

    #[test]
    fn test_refresh_token_request_empty() {
        let result = RefreshTokenRequest::new("".to_string());
        assert!(matches!(result, Err(RefreshTokenRequestError::EmptyToken)));
    }

    #[test]
    fn test_refresh_token_request_whitespace_only() {
        let result = RefreshTokenRequest::new("   ".to_string());
        assert!(matches!(result, Err(RefreshTokenRequestError::EmptyToken)));
    }

    #[test]
    fn test_refresh_token_request_deserialize_valid() {
        let json = json!({
            "refresh_token": "valid_token_123"
        });

        let request: RefreshTokenRequest = serde_json::from_value(json).unwrap();
        assert_eq!(request.refresh_token(), "valid_token_123");
    }

    #[test]
    fn test_refresh_token_request_deserialize_empty() {
        let json = json!({
            "refresh_token": ""
        });

        let result: Result<RefreshTokenRequest, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_refresh_token_request_error_display() {
        assert_eq!(
            RefreshTokenRequestError::EmptyToken.to_string(),
            "Refresh token cannot be empty"
        );
    }

    // ==================== RefreshTokenError Tests ====================
    #[test]
    fn test_refresh_token_error_display() {
        assert_eq!(
            RefreshTokenError::TokenExpired.to_string(),
            "Refresh token has expired"
        );
        assert_eq!(
            RefreshTokenError::TokenInvalid.to_string(),
            "Invalid refresh token"
        );
        assert_eq!(
            RefreshTokenError::InvalidTokenType.to_string(),
            "Invalid token type"
        );
    }

    #[test]
    fn test_jwt_error_conversion() {
        let jwt_error = TokenError::TokenExpired;
        let refresh_error: RefreshTokenError = jwt_error.into();
        assert!(matches!(refresh_error, RefreshTokenError::TokenExpired));
    }

    // ==================== RefreshTokenUseCase Tests ====================
    #[tokio::test]
    async fn test_refresh_token_success() {
        let jwt_service = create_jwt_service();
        let user_id = Uuid::new_v4();

        // Generate a valid refresh token
        let refresh_token = jwt_service.generate_refresh_token(user_id, true).unwrap();

        let use_case = RefreshTokenUseCase::new(Arc::new(jwt_service));
        let request = RefreshTokenRequest::new(refresh_token).unwrap();

        let result = use_case.execute(request).await;

        assert!(result.is_ok(), "Expected successful token refresh");
        let response = result.unwrap();
        assert!(!response.access_token.is_empty());
        assert!(!response.refresh_token.is_empty());
    }

    #[tokio::test]
    async fn test_refresh_token_with_rotation() {
        let jwt_service = create_jwt_service();
        let user_id = Uuid::new_v4();

        let original_refresh_token = jwt_service.generate_refresh_token(user_id, true).unwrap();

        // Add a 32 second delay to ensure different timestamps follow the `validation.leeway = 30;` in JWT Service
        tokio::time::sleep(tokio::time::Duration::from_millis(1920)).await;

        let use_case = RefreshTokenUseCase::new(Arc::new(jwt_service)).with_token_rotation(true);

        let request = RefreshTokenRequest::new(original_refresh_token.clone()).unwrap();
        let result = use_case.execute(request).await;

        assert!(result.is_ok());
        let response = result.unwrap();

        // With rotation, new refresh token should be different
        assert_ne!(response.refresh_token, original_refresh_token);
    }

    #[tokio::test]
    async fn test_refresh_token_without_rotation() {
        let jwt_service = create_jwt_service();
        let user_id = Uuid::new_v4();

        let original_refresh_token = jwt_service.generate_refresh_token(user_id, true).unwrap();

        let use_case = RefreshTokenUseCase::new(Arc::new(jwt_service)).with_token_rotation(false);

        let request = RefreshTokenRequest::new(original_refresh_token.clone()).unwrap();
        let result = use_case.execute(request).await;

        assert!(result.is_ok());
        let response = result.unwrap();

        // Without rotation, refresh token should be the same
        assert_eq!(response.refresh_token, original_refresh_token);
    }

    #[tokio::test]
    async fn test_refresh_token_expired() {
        let jwt_service = JwtTokenService::new(JwtConfig {
            secret_key: "test_secret_key_min_32_characters_long".to_string(),
            issuer: "testapp".to_string(),
            access_token_expiry: 3600,
            refresh_token_expiry: -60, // Expired token
            verification_token_expiry: 86400,
        });

        let user_id = Uuid::new_v4();
        let expired_token = jwt_service.generate_refresh_token(user_id, true).unwrap();

        let use_case = RefreshTokenUseCase::new(Arc::new(create_jwt_service()));
        let request = RefreshTokenRequest::new(expired_token).unwrap();
        let result = use_case.execute(request).await;

        assert!(
            matches!(result, Err(RefreshTokenError::TokenExpired)),
            "Expected TokenExpired, got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_refresh_token_invalid_token() {
        let use_case = RefreshTokenUseCase::new(Arc::new(create_jwt_service()));
        let request = RefreshTokenRequest::new("invalid.token.here".to_string()).unwrap();

        let result = use_case.execute(request).await;

        assert!(
            matches!(result, Err(RefreshTokenError::TokenInvalid)),
            "Expected TokenInvalid, got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_refresh_token_wrong_token_type() {
        let jwt_service = create_jwt_service();
        let user_id = Uuid::new_v4();

        // Generate an access token instead of refresh token
        let access_token = jwt_service.generate_access_token(user_id, true).unwrap();

        let use_case = RefreshTokenUseCase::new(Arc::new(jwt_service));
        let request = RefreshTokenRequest::new(access_token).unwrap();
        let result = use_case.execute(request).await;

        assert!(
            matches!(result, Err(RefreshTokenError::InvalidTokenType)),
            "Expected InvalidTokenType, got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_refresh_token_verification_token() {
        let jwt_service = create_jwt_service();
        let user_id = Uuid::new_v4();

        // Generate a verification token
        let verification_token = jwt_service.generate_verification_token(user_id).unwrap();

        let use_case = RefreshTokenUseCase::new(Arc::new(jwt_service));
        let request = RefreshTokenRequest::new(verification_token).unwrap();
        let result = use_case.execute(request).await;

        assert!(
            matches!(result, Err(RefreshTokenError::InvalidTokenType)),
            "Expected InvalidTokenType, got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_refresh_token_invalid_signature() {
        let jwt_service1 = JwtTokenService::new(JwtConfig {
            secret_key: "secret_one_min_32_characters_long_key".to_string(),
            issuer: "testapp".to_string(),
            access_token_expiry: 3600,
            refresh_token_expiry: 86400,
            verification_token_expiry: 86400,
        });

        let jwt_service2 = create_jwt_service(); // Different secret

        let user_id = Uuid::new_v4();
        let token = jwt_service1.generate_refresh_token(user_id, true).unwrap();

        let use_case = RefreshTokenUseCase::new(Arc::new(jwt_service2));
        let request = RefreshTokenRequest::new(token).unwrap();
        let result = use_case.execute(request).await;

        assert!(
            matches!(result, Err(RefreshTokenError::InvalidSignature)),
            "Expected InvalidSignature, got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_refresh_token_preserves_user_verification_status() {
        let jwt_service = create_jwt_service();
        let user_id = Uuid::new_v4();

        // Test with verified user
        let refresh_token_verified = jwt_service.generate_refresh_token(user_id, true).unwrap();

        let use_case = RefreshTokenUseCase::new(Arc::new(jwt_service.clone()));
        let request = RefreshTokenRequest::new(refresh_token_verified).unwrap();
        let result = use_case.execute(request).await;

        assert!(result.is_ok());
        let access_token = result.unwrap().access_token;
        let claims = jwt_service.verify_token(&access_token).unwrap();
        assert_eq!(claims.is_verified, true);

        // Test with unverified user
        let refresh_token_unverified = jwt_service.generate_refresh_token(user_id, false).unwrap();

        let request = RefreshTokenRequest::new(refresh_token_unverified).unwrap();
        let result = use_case.execute(request).await;

        assert!(result.is_ok());
        let access_token = result.unwrap().access_token;
        let claims = jwt_service.verify_token(&access_token).unwrap();
        assert_eq!(claims.is_verified, false);
    }
}
