use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};

use std::fmt;
use tracing;
use uuid::Uuid;

use crate::auth::application::ports::outgoing::token_provider::{
    TokenClaims, TokenError, TokenProvider,
};

use super::jwt_config::JwtConfig;

#[derive(Clone)]
pub struct JwtTokenService {
    config: JwtConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

#[cfg(not(tarpaulin_include))]
impl fmt::Debug for JwtTokenService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JwtService")
            .field("config", &"JwtConfig")
            .finish()
    }
}
impl JwtTokenService {
    /// Initialize the service with config
    pub fn new(config: JwtConfig) -> Self {
        let encoding_key = EncodingKey::from_secret(config.secret_key.as_bytes());
        let decoding_key = DecodingKey::from_secret(config.secret_key.as_bytes());

        Self {
            config,
            encoding_key,
            decoding_key,
        }
    }

    fn generate_token(
        &self,
        user_id: Uuid,
        is_verified: bool,
        token_type: &str,
        expiry_seconds: i64,
    ) -> Result<String, TokenError> {
        let now = Utc::now();
        let expiration = now + Duration::seconds(expiry_seconds);

        let claims = TokenClaims {
            sub: user_id,
            exp: expiration.timestamp(),
            iat: now.timestamp(),
            nbf: now.timestamp(),
            token_type: token_type.to_string(),
            is_verified,
        };

        encode(&Header::new(Algorithm::HS256), &claims, &self.encoding_key)
            .map_err(|e| TokenError::EncodingError(e.to_string()))
    }
}
impl TokenProvider for JwtTokenService {
    /// Generate an access token
    fn generate_access_token(
        &self,
        user_id: Uuid,
        is_verified: bool,
    ) -> Result<String, TokenError> {
        let expiry_seconds = self.config.access_token_expiry;
        self.generate_token(user_id, is_verified, "access", expiry_seconds)
    }

    /// Generate a refresh token
    fn generate_refresh_token(
        &self,
        user_id: Uuid,
        is_verified: bool,
    ) -> Result<String, TokenError> {
        let expiry_seconds = self.config.refresh_token_expiry;
        self.generate_token(user_id, is_verified, "refresh", expiry_seconds)
    }

    /// Verify and decode a token
    fn verify_token(&self, token: &str) -> Result<TokenClaims, TokenError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.leeway = 30;
        validation.validate_nbf = true;

        let decoded =
            decode::<TokenClaims>(token, &self.decoding_key, &validation).map_err(|e| {
                use jsonwebtoken::errors::ErrorKind;

                let error = match e.kind() {
                    ErrorKind::ExpiredSignature => {
                        tracing::debug!("Token verification failed: Token expired");
                        TokenError::TokenExpired
                    }
                    ErrorKind::ImmatureSignature => {
                        tracing::warn!("Token verification failed: Token not yet valid");
                        TokenError::TokenNotYetValid
                    }
                    ErrorKind::InvalidSignature => {
                        tracing::error!("Security alert: Invalid token signature detected");
                        TokenError::InvalidSignature
                    }
                    ErrorKind::InvalidToken | ErrorKind::InvalidAlgorithm => {
                        tracing::error!("Security alert: Malformed or invalid algorithm token");
                        TokenError::MalformedToken
                    }
                    ErrorKind::Base64(_) | ErrorKind::Json(_) | ErrorKind::Utf8(_) => {
                        tracing::warn!("Token verification failed: Malformed token");
                        TokenError::MalformedToken
                    }
                    _ => {
                        tracing::warn!("Token verification failed: Unknown error");
                        TokenError::MalformedToken
                    }
                };

                error
            })?;

        Ok(decoded.claims)
    }

    /// Refresh an access token using a valid refresh token
    fn refresh_access_token(&self, refresh_token: &str) -> Result<String, TokenError> {
        let claims = self.verify_token(refresh_token)?;

        if claims.token_type != "refresh" {
            tracing::warn!(
                "Token type mismatch: expected 'refresh', got '{}'",
                claims.token_type
            );
            return Err(TokenError::InvalidTokenType("refresh".to_string()));
        }

        tracing::debug!(
            "Refresh token validated, issuing new access token for user: {}",
            claims.sub
        );
        self.generate_access_token(claims.sub, claims.is_verified)
    }

    /// Verify an email verification token and extract the user ID
    fn verify_verification_token(&self, token: &str) -> Result<Uuid, TokenError> {
        let claims = self.verify_token(token)?;

        if claims.token_type != "verification" {
            tracing::warn!(
                "Token type mismatch: expected 'verification', got '{}'",
                claims.token_type
            );
            return Err(TokenError::InvalidTokenType("verification".to_string()));
        }

        tracing::debug!(
            "Verification token validated successfully for user: {}",
            claims.sub
        );
        Ok(claims.sub)
    }

    fn generate_verification_token(&self, user_id: Uuid) -> Result<String, TokenError> {
        let token_expiry = self.config.verification_token_expiry;
        self.generate_token(user_id, false, "verification", token_expiry)
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::support::load_test_env;

    use super::*;

    // Helper function to create a test JwtService
    fn create_test_jwt_service() -> JwtTokenService {
        let config = JwtConfig {
            secret_key: std::env::var("TEST_JWT_SECRET")
                .unwrap_or_else(|_| "FAKE_JWT_SECRET_DO_NOT_USE".to_string()),
            issuer: "test_issuer".to_string(),
            access_token_expiry: 3600,        // 1 hour
            refresh_token_expiry: 86400,      // 24 hours
            verification_token_expiry: 86400, // 24 hours
        };
        JwtTokenService::new(config)
    }

    #[test]
    fn test_generate_and_verify_access_token() {
        let config = JwtConfig {
            secret_key: std::env::var("TEST_JWT_SECRET")
                .unwrap_or_else(|_| "FAKE_JWT_SECRET_DO_NOT_USE".to_string()),

            issuer: "myapp".to_string(),
            access_token_expiry: 3600,
            refresh_token_expiry: 86400,
            verification_token_expiry: 86400,
        };

        let jwt_service = JwtTokenService::new(config);
        let user_id = Uuid::new_v4();

        // Generate token
        let token = jwt_service
            .generate_access_token(user_id, true)
            .expect("Token should be generated");

        // Verify token
        let claims = jwt_service.verify_token(&token);
        assert!(claims.is_ok(), "Token should be valid");
        let claims = claims.unwrap();
        assert_eq!(claims.sub, user_id, "User ID should match");
        assert_eq!(claims.token_type, "access");
        assert_eq!(claims.is_verified, true);
    }

    #[test]
    fn test_generate_and_verify_access_token_unverified_user() {
        let service = create_test_jwt_service();
        let user_id = Uuid::new_v4();

        // Generate token for unverified user
        let token = service
            .generate_access_token(user_id, false)
            .expect("Token should be generated");

        // Verify token
        let claims = service.verify_token(&token).unwrap();
        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.is_verified, false);
        assert_eq!(claims.token_type, "access");
    }

    #[test]
    fn test_invalid_token_verification() {
        let service = create_test_jwt_service();

        // Invalid token
        let invalid_token = "invalid.jwt.token";
        let result = service.verify_token(invalid_token);

        assert!(result.is_err(), "Invalid token should fail verification");
        assert!(matches!(result.unwrap_err(), TokenError::MalformedToken));
    }

    #[test]
    fn test_malformed_token_base64_error() {
        let service = create_test_jwt_service();

        // Token with invalid base64
        let result = service.verify_token("not.a.valid@base64.token!");

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TokenError::MalformedToken));
    }

    #[test]
    fn test_token_with_invalid_json() {
        use base64::{engine::general_purpose, Engine as _};
        let service = create_test_jwt_service();

        // Create a token with invalid JSON in payload
        let header = general_purpose::STANDARD.encode(r#"{"alg":"HS256","typ":"JWT"}"#);
        let payload = general_purpose::STANDARD.encode("not valid json");
        let invalid_token = format!("{}.{}.fakesignature", header, payload);

        let result = service.verify_token(&invalid_token);
        assert!(result.is_err());
    }

    #[test]
    fn test_expired_token() {
        let config = JwtConfig {
            secret_key: std::env::var("TEST_JWT_SECRET")
                .unwrap_or_else(|_| "FAKE_JWT_SECRET_DO_NOT_USE".to_string()),
            issuer: "myapp".to_string(),
            access_token_expiry: -35, // Already expired (beyond leeway)
            refresh_token_expiry: 86400,
            verification_token_expiry: 86400,
        };

        let jwt_service = JwtTokenService::new(config);
        let user_id = Uuid::new_v4();

        // Generate token (will be immediately expired)
        let token = jwt_service
            .generate_access_token(user_id, true)
            .expect("Token should be generated");

        // Verify expired token (no sleep needed)
        let result = jwt_service.verify_token(&token);

        assert!(result.is_err(), "Expired token should be invalid");
        assert!(matches!(result.unwrap_err(), TokenError::TokenExpired));
    }

    #[test]
    fn test_token_not_yet_valid() {
        // This test would require manually crafting a token with nbf in the future
        // Or modifying the generate_token to accept a custom nbf for testing
        // For now, we'll skip this as our implementation sets nbf to now
    }

    #[test]
    fn test_invalid_signature() {
        load_test_env();

        // service secret (bisa dari env, bisa fallback)
        let service = create_test_jwt_service();
        let user_id = Uuid::new_v4();

        let token = service.generate_access_token(user_id, true).unwrap();

        // pastikan berbeda dari secret service (apa pun nilai env-nya)
        let different_secret = format!("{}_DIFFERENT", service.config.secret_key);

        let different_config = JwtConfig {
            secret_key: different_secret,
            issuer: "test".to_string(),
            access_token_expiry: 3600,
            refresh_token_expiry: 86400,
            verification_token_expiry: 86400,
        };

        let different_service = JwtTokenService::new(different_config);

        let result = different_service.verify_token(&token);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TokenError::InvalidSignature));
    }

    #[test]
    fn test_generate_refresh_token() {
        let service = create_test_jwt_service();
        let user_id = Uuid::new_v4();

        // Generate refresh token
        let token = service
            .generate_refresh_token(user_id, true)
            .expect("Refresh token should be generated");

        // Verify refresh token
        let claims = service.verify_token(&token);
        assert!(claims.is_ok(), "Refresh token should be valid");
        let claims = claims.unwrap();
        assert_eq!(claims.sub, user_id, "User ID should match in refresh token");
        assert_eq!(claims.token_type, "refresh");
        assert_eq!(claims.is_verified, true);
    }

    #[test]
    fn test_generate_refresh_token_unverified() {
        let service = create_test_jwt_service();
        let user_id = Uuid::new_v4();

        let token = service.generate_refresh_token(user_id, false).unwrap();
        let claims = service.verify_token(&token).unwrap();

        assert_eq!(claims.is_verified, false);
        assert_eq!(claims.token_type, "refresh");
    }

    #[test]
    fn test_generate_and_verify_verification_token() {
        let service = create_test_jwt_service();
        let user_id = Uuid::new_v4();

        let token = service
            .generate_verification_token(user_id)
            .expect("Should generate verification token");

        // Verify using verify_token
        let claims = service.verify_token(&token).unwrap();
        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.token_type, "verification");
        assert_eq!(claims.is_verified, false);

        // Verify using verify_verification_token
        let result = service.verify_verification_token(&token);
        assert!(result.is_ok(), "Token should be valid");
        assert_eq!(result.unwrap(), user_id, "User ID should match");
    }

    #[test]
    fn test_verify_verification_token_with_wrong_type() {
        let service = create_test_jwt_service();
        let user_id = Uuid::new_v4();

        // Generate an access token
        let access_token = service.generate_access_token(user_id, true).unwrap();

        // Try to verify it as verification token
        let result = service.verify_verification_token(&access_token);

        assert!(result.is_err());
        match result.unwrap_err() {
            TokenError::InvalidTokenType(expected) => {
                assert_eq!(expected, "verification");
            }
            _ => panic!("Expected InvalidTokenType error"),
        }
    }

    #[test]
    fn test_verify_verification_token_with_refresh_token() {
        let service = create_test_jwt_service();
        let user_id = Uuid::new_v4();

        let refresh_token = service.generate_refresh_token(user_id, true).unwrap();
        let result = service.verify_verification_token(&refresh_token);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TokenError::InvalidTokenType(_)
        ));
    }

    #[test]
    fn test_refresh_access_token_success() {
        let service = create_test_jwt_service();
        let user_id = Uuid::new_v4();

        // Generate a valid refresh token
        let refresh_token = service.generate_refresh_token(user_id, true).unwrap();

        // Refresh the access token
        let result = service.refresh_access_token(&refresh_token);

        assert!(result.is_ok());
        let new_access_token = result.unwrap();

        // Verify the new access token is valid
        let claims = service.verify_token(&new_access_token).unwrap();
        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.token_type, "access");
        assert_eq!(claims.is_verified, true);
    }

    #[test]
    fn test_refresh_access_token_with_access_token_fails() {
        let service = create_test_jwt_service();
        let user_id = Uuid::new_v4();

        // Generate an access token (not a refresh token)
        let access_token = service.generate_access_token(user_id, true).unwrap();

        // Try to refresh using an access token (should fail)
        let result = service.refresh_access_token(&access_token);

        assert!(result.is_err());
        match result.unwrap_err() {
            TokenError::InvalidTokenType(expected) => {
                assert_eq!(expected, "refresh");
            }
            _ => panic!("Expected InvalidTokenType error"),
        }
    }

    #[test]
    fn test_refresh_access_token_with_verification_token_fails() {
        let service = create_test_jwt_service();
        let user_id = Uuid::new_v4();

        // Generate a verification token
        let verification_token = service.generate_verification_token(user_id).unwrap();

        // Try to refresh using a verification token (should fail)
        let result = service.refresh_access_token(&verification_token);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TokenError::InvalidTokenType(_)
        ));
    }

    #[test]
    fn test_refresh_access_token_with_invalid_token() {
        let service = create_test_jwt_service();

        // Try to refresh with an invalid token string
        let result = service.refresh_access_token("invalid_token_string");

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TokenError::MalformedToken));
    }

    #[test]
    fn test_refresh_access_token_with_expired_refresh_token() {
        // Create a service with very short expiry
        let config = JwtConfig {
            secret_key: std::env::var("TEST_JWT_SECRET")
                .unwrap_or_else(|_| "FAKE_JWT_SECRET_DO_NOT_USE".to_string()),
            issuer: "test_issuer".to_string(),
            access_token_expiry: 3600,
            refresh_token_expiry: -32, // 1 second
            verification_token_expiry: 86400,
        };
        let service = JwtTokenService::new(config);
        let user_id = Uuid::new_v4();

        // Generate a refresh token
        let refresh_token = service.generate_refresh_token(user_id, true).unwrap();

        // Try to refresh with expired token
        let result = service.refresh_access_token(&refresh_token);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TokenError::TokenExpired));
    }

    #[test]
    fn test_refresh_access_token_with_tampered_token() {
        let service = create_test_jwt_service();
        let user_id = Uuid::new_v4();

        // Generate a valid refresh token
        let mut refresh_token = service.generate_refresh_token(user_id, true).unwrap();

        // Tamper with the token (change a character)
        refresh_token.push('x');

        // Try to refresh with tampered token
        let result = service.refresh_access_token(&refresh_token);

        assert!(result.is_err());
    }

    #[test]
    fn test_refresh_access_token_preserves_user_id() {
        let service = create_test_jwt_service();
        let user_id = Uuid::new_v4();

        // Generate a refresh token
        let refresh_token = service.generate_refresh_token(user_id, true).unwrap();

        // Refresh to get new access token
        let new_access_token = service.refresh_access_token(&refresh_token).unwrap();

        // Verify the user ID is preserved
        let claims = service.verify_token(&new_access_token).unwrap();
        assert_eq!(claims.sub, user_id);
    }

    #[test]
    fn test_refresh_access_token_preserves_verification_status() {
        let service = create_test_jwt_service();
        let user_id = Uuid::new_v4();

        // Test with verified user
        let refresh_token_verified = service.generate_refresh_token(user_id, true).unwrap();
        let access_token = service
            .refresh_access_token(&refresh_token_verified)
            .unwrap();
        let claims = service.verify_token(&access_token).unwrap();
        assert_eq!(claims.is_verified, true);

        // Test with unverified user
        let refresh_token_unverified = service.generate_refresh_token(user_id, false).unwrap();
        let access_token = service
            .refresh_access_token(&refresh_token_unverified)
            .unwrap();
        let claims = service.verify_token(&access_token).unwrap();
        assert_eq!(claims.is_verified, false);
    }

    #[test]
    fn test_refresh_access_token_multiple_times() {
        let service = create_test_jwt_service();
        let user_id = Uuid::new_v4();

        // Generate initial refresh token
        let refresh_token = service.generate_refresh_token(user_id, true).unwrap();

        // Refresh multiple times with the same refresh token
        let access_token_1 = service.refresh_access_token(&refresh_token).unwrap();
        let access_token_2 = service.refresh_access_token(&refresh_token).unwrap();

        // Both should be valid (though different tokens)
        let claims_1 = service.verify_token(&access_token_1).unwrap();
        let claims_2 = service.verify_token(&access_token_2).unwrap();

        assert_eq!(claims_1.sub, user_id);
        assert_eq!(claims_2.sub, user_id);
        assert_eq!(claims_1.token_type, "access");
        assert_eq!(claims_2.token_type, "access");
    }

    #[test]
    fn test_jwt_claims_has_required_fields() {
        let service = create_test_jwt_service();
        let user_id = Uuid::new_v4();

        let token = service.generate_access_token(user_id, true).unwrap();
        let claims = service.verify_token(&token).unwrap();

        // Verify all required fields are present
        assert_eq!(claims.sub, user_id);
        assert!(claims.exp > 0);
        assert!(claims.iat > 0);
        assert!(claims.nbf > 0);
        assert!(!claims.token_type.is_empty());
    }

    #[test]
    fn test_token_expiry_is_in_future() {
        let service = create_test_jwt_service();
        let user_id = Uuid::new_v4();

        let token = service.generate_access_token(user_id, true).unwrap();
        let claims = service.verify_token(&token).unwrap();

        let now = Utc::now().timestamp();
        assert!(claims.exp > now, "Expiry should be in the future");
        assert!(claims.iat <= now, "Issued at should be now or in the past");
        assert!(claims.nbf <= now, "Not before should be now or in the past");
    }

    #[test]
    fn test_jwt_error_display() {
        assert_eq!(format!("{}", TokenError::TokenExpired), "Token has expired");
        assert_eq!(
            format!("{}", TokenError::TokenNotYetValid),
            "Token is not yet valid"
        );
        assert_eq!(
            format!("{}", TokenError::InvalidTokenType("refresh".to_string())),
            "Invalid token type, expected: refresh"
        );
        assert_eq!(
            format!("{}", TokenError::InvalidSignature),
            "Invalid token signature"
        );
        assert_eq!(format!("{}", TokenError::MalformedToken), "Malformed token");
        assert_eq!(
            format!("{}", TokenError::EncodingError("test error".to_string())),
            "Token encoding error: test error"
        );
    }

    #[test]
    fn test_jwt_error_is_error_trait() {
        let error: Box<dyn std::error::Error> = Box::new(TokenError::TokenExpired);
        assert_eq!(error.to_string(), "Token has expired");
    }

    #[test]
    fn test_jwt_service_debug() {
        let service = create_test_jwt_service();
        let debug_str = format!("{:?}", service);
        assert!(debug_str.contains("JwtService"));
    }

    #[test]
    fn test_jwt_claims_debug() {
        let claims = TokenClaims {
            sub: Uuid::new_v4(),
            exp: 12345,
            iat: 12340,
            nbf: 12340,
            token_type: "access".to_string(),
            is_verified: true,
        };
        let debug_str = format!("{:?}", claims);
        assert!(debug_str.contains("TokenClaims"));
        assert!(debug_str.contains("access"));
    }

    #[test]
    fn test_jwt_service_clone() {
        let service = create_test_jwt_service();
        let cloned_service = service.clone();

        let user_id = Uuid::new_v4();
        let token1 = service.generate_access_token(user_id, true).unwrap();
        let token2 = cloned_service.generate_access_token(user_id, true).unwrap();

        // Both services should produce valid tokens
        assert!(service.verify_token(&token1).is_ok());
        assert!(cloned_service.verify_token(&token2).is_ok());
    }
}
