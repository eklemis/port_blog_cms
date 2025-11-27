use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use super::jwt_config::JwtConfig;

/// Structure for JWT Claims
#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: Uuid,          // User ID
    pub exp: i64,           // Expiration timestamp
    pub token_type: String, // Either "access" or "refresh"
}

#[derive(Clone)]
pub struct JwtService {
    config: JwtConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

#[cfg(not(tarpaulin_include))]
impl fmt::Debug for JwtService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JwtService")
            .field("config", &"JwtConfig")
            .finish()
    }
}
impl JwtService {
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

    /// Generate an access token
    pub fn generate_access_token(&self, user_id: Uuid) -> Result<String, String> {
        let expiration = Utc::now() + Duration::seconds(self.config.access_token_expiry);
        let claims = JwtClaims {
            sub: user_id,
            exp: expiration.timestamp(),
            token_type: "access".to_string(),
        };

        encode(&Header::new(Algorithm::HS256), &claims, &self.encoding_key)
            .map_err(|e| e.to_string())
    }

    /// Generate a refresh token
    pub fn generate_refresh_token(&self, user_id: Uuid) -> Result<String, String> {
        let expiration = Utc::now() + Duration::seconds(self.config.refresh_token_expiry);
        let claims = JwtClaims {
            sub: user_id,
            exp: expiration.timestamp(),
            token_type: "refresh".to_string(),
        };

        encode(&Header::new(Algorithm::HS256), &claims, &self.encoding_key)
            .map_err(|e| e.to_string())
    }

    /// Verify and decode a token
    pub fn verify_token(&self, token: &str) -> Result<JwtClaims, String> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = false; // We will enforce manually

        let decoded = decode::<JwtClaims>(token, &self.decoding_key, &validation)
            .map_err(|e| e.to_string())?;

        let now = Utc::now().timestamp();
        if decoded.claims.exp < now {
            return Err("TokenExpired".to_string());
        }

        Ok(decoded.claims)
    }

    /// Refresh an access token using a valid refresh token
    pub fn refresh_access_token(&self, refresh_token: &str) -> Result<String, String> {
        let claims = self.verify_token(refresh_token)?;

        if claims.token_type != "refresh" {
            return Err("Invalid token type".to_string());
        }

        self.generate_access_token(claims.sub)
    }

    /// Verify an email verification token and extract the user ID
    pub fn verify_verification_token(&self, token: &str) -> Result<Uuid, String> {
        let claims = self.verify_token(token)?;

        if claims.token_type != "verification" {
            return Err("Invalid token type".to_string());
        }

        Ok(claims.sub)
    }

    pub fn generate_verification_token(&self, user_id: Uuid) -> Result<String, String> {
        let expiration = Utc::now() + Duration::hours(24); // Token valid for 24 hours
        let claims = JwtClaims {
            sub: user_id,
            exp: expiration.timestamp(),
            token_type: "verification".to_string(),
        };

        encode(&Header::new(Algorithm::HS256), &claims, &self.encoding_key)
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_generate_and_verify_access_token() {
        let secret = "mysecretkey".to_string();
        let issuer = "myapp".to_string();
        let config = JwtConfig {
            secret_key: secret.clone(),
            issuer,
            access_token_expiry: 3600,   // 1 hour
            refresh_token_expiry: 86400, // 1 day
        };

        let jwt_service = JwtService::new(config);
        let user_id = Uuid::new_v4();

        // Generate token
        let token = jwt_service
            .generate_access_token(user_id)
            .expect("Token should be generated");

        // Verify token
        let claims = jwt_service.verify_token(&token);
        assert!(claims.is_ok(), "Token should be valid");
        let claims = claims.unwrap();
        assert_eq!(claims.sub, user_id, "User ID should match");
    }

    #[test]
    fn test_invalid_token_verification() {
        let secret = "mysecretkey".to_string();
        let config = JwtConfig {
            secret_key: secret.clone(),
            issuer: "myapp".to_string(),
            access_token_expiry: 3600,
            refresh_token_expiry: 86400,
        };

        let jwt_service = JwtService::new(config);

        // Invalid token
        let invalid_token = "invalid.jwt.token";
        let claims = jwt_service.verify_token(invalid_token);
        assert!(claims.is_err(), "Invalid token should fail verification");
    }

    #[test]
    fn test_expired_token() {
        let secret = "mysecretkey".to_string();
        let config = JwtConfig {
            secret_key: secret.clone(),
            issuer: "myapp".to_string(),
            access_token_expiry: 2, // Set expiry to 2 seconds
            refresh_token_expiry: 86400,
        };

        let jwt_service = JwtService::new(config);
        let user_id = Uuid::new_v4();

        // Generate token
        let token = jwt_service
            .generate_access_token(user_id)
            .expect("Token should be generated");

        println!("Generated Token: {}", token);

        // Wait for expiration (5 seconds just to be sure)
        std::thread::sleep(Duration::from_secs(5));

        // Verify expired token
        let claims = jwt_service.verify_token(&token);

        println!("Verification Result: {:?}", claims);

        assert!(claims.is_err(), "Expired token should be invalid");
    }

    #[test]
    fn test_generate_refresh_token() {
        let secret = "mysecretkey".to_string();
        let config = JwtConfig {
            secret_key: secret.clone(),
            issuer: "myapp".to_string(),
            access_token_expiry: 3600,
            refresh_token_expiry: 86400, // 1 day
        };

        let jwt_service = JwtService::new(config);
        let user_id = Uuid::new_v4();

        // Generate refresh token
        let token = jwt_service
            .generate_refresh_token(user_id)
            .expect("Refresh token should be generated");

        // Verify refresh token
        let claims = jwt_service.verify_token(&token);
        assert!(claims.is_ok(), "Refresh token should be valid");
        let claims = claims.unwrap();
        assert_eq!(claims.sub, user_id, "User ID should match in refresh token");
    }
    #[test]
    fn test_generate_and_verify_verification_token() {
        let config = JwtConfig {
            secret_key: "mysecretkey".to_string(),
            issuer: "myapp".to_string(),
            access_token_expiry: 3600,
            refresh_token_expiry: 86400,
        };
        let jwt_service = JwtService::new(config);
        let user_id = Uuid::new_v4();

        let token = jwt_service
            .generate_verification_token(user_id)
            .expect("Should generate verification token");

        let result = jwt_service.verify_verification_token(&token);
        assert!(result.is_ok(), "Token should be valid");
        assert_eq!(result.unwrap(), user_id, "User ID should match");
    }
    // Helper function to create a test JwtService
    fn create_test_jwt_service() -> JwtService {
        let config = JwtConfig {
            secret_key: "test_secret_key_for_testing_purposes".to_string(),
            issuer: "test_issuer".to_string(),
            access_token_expiry: 3600,   // 1 hour
            refresh_token_expiry: 86400, // 24 hours
        };
        JwtService::new(config)
    }

    #[test]
    fn test_refresh_access_token_success() {
        let service = create_test_jwt_service();
        let user_id = Uuid::new_v4();

        // Generate a valid refresh token
        let refresh_token = service.generate_refresh_token(user_id).unwrap();

        // Refresh the access token
        let result = service.refresh_access_token(&refresh_token);

        assert!(result.is_ok());
        let new_access_token = result.unwrap();

        // Verify the new access token is valid
        let claims = service.verify_token(&new_access_token).unwrap();
        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.token_type, "access");
    }

    #[test]
    fn test_refresh_access_token_with_access_token_fails() {
        let service = create_test_jwt_service();
        let user_id = Uuid::new_v4();

        // Generate an access token (not a refresh token)
        let access_token = service.generate_access_token(user_id).unwrap();

        // Try to refresh using an access token (should fail)
        let result = service.refresh_access_token(&access_token);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid token type");
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
        assert_eq!(result.unwrap_err(), "Invalid token type");
    }

    #[test]
    fn test_refresh_access_token_with_invalid_token() {
        let service = create_test_jwt_service();

        // Try to refresh with an invalid token string
        let result = service.refresh_access_token("invalid_token_string");

        assert!(result.is_err());
        // Store the error to avoid moving result twice
        let error = result.unwrap_err();
        assert!(error.contains("Invalid") || error.contains("token"));
    }

    #[test]
    fn test_refresh_access_token_with_expired_refresh_token() {
        // Create a service with very short expiry
        let config = JwtConfig {
            secret_key: "test_secret_key_for_testing_purposes".to_string(),
            issuer: "test_issuer".to_string(),
            access_token_expiry: 3600,
            refresh_token_expiry: -1, // Already expired
        };
        let service = JwtService::new(config);
        let user_id = Uuid::new_v4();

        // Generate an already-expired refresh token
        let refresh_token = service.generate_refresh_token(user_id).unwrap();

        // Try to refresh with expired token
        let result = service.refresh_access_token(&refresh_token);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "TokenExpired");
    }

    #[test]
    fn test_refresh_access_token_with_tampered_token() {
        let service = create_test_jwt_service();
        let user_id = Uuid::new_v4();

        // Generate a valid refresh token
        let mut refresh_token = service.generate_refresh_token(user_id).unwrap();

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
        let refresh_token = service.generate_refresh_token(user_id).unwrap();

        // Refresh to get new access token
        let new_access_token = service.refresh_access_token(&refresh_token).unwrap();

        // Verify the user ID is preserved
        let claims = service.verify_token(&new_access_token).unwrap();
        assert_eq!(claims.sub, user_id);
    }

    #[test]
    fn test_refresh_access_token_multiple_times() {
        let service = create_test_jwt_service();
        let user_id = Uuid::new_v4();

        // Generate initial refresh token
        let refresh_token = service.generate_refresh_token(user_id).unwrap();

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
}
