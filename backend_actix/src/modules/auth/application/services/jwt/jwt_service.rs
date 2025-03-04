use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::jwt_config::JwtConfig;

/// Structure for JWT Claims
#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: Uuid,          // User ID
    pub exp: i64,           // Expiration timestamp
    pub token_type: String, // Either "access" or "refresh"
}

pub struct JwtService {
    config: JwtConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
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
            return Err("Token has expired".to_string());
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
}
