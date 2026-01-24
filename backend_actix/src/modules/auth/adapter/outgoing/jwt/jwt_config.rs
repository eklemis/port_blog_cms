use std::env;

#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub secret_key: String,
    pub issuer: String,
    pub access_token_expiry: i64,       // Expiration in seconds
    pub refresh_token_expiry: i64,      // Expiration in seconds
    pub verification_token_expiry: i64, // Expiration in seconds
}

impl JwtConfig {
    /// Helper function to parse expiry values
    fn parse_expiry(key: &str, default: &str) -> i64 {
        env::var(key)
            .unwrap_or_else(|_| default.to_string())
            .parse::<i64>()
            .unwrap_or_else(|_| panic!("Invalid {} value", key))
    }
    /// Load JWT configuration from environment variables
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok(); // Load environment variables if available

        let secret_key = env::var("JWT_SECRET").expect("JWT_SECRET must be set");

        // Validate secret key length (HS256 requires at least 32 bytes)
        if secret_key.len() < 32 {
            panic!("JWT_SECRET must be at least 32 characters long for HS256 algorithm");
        }

        let access_token_expiry = Self::parse_expiry("JWT_ACCESS_EXPIRY", "1800");
        let refresh_token_expiry = Self::parse_expiry("JWT_REFRESH_EXPIRY", "604800");
        let verification_token_expiry = Self::parse_expiry("JWT_VERIFICATION_EXPIRY", "86400");

        // Validate expiry values
        if access_token_expiry <= 0 || access_token_expiry > 86400 {
            panic!("JWT_ACCESS_EXPIRY must be between 1 and 86400 seconds (24 hours)");
        }

        if refresh_token_expiry <= access_token_expiry {
            panic!("JWT_REFRESH_EXPIRY must be greater than JWT_ACCESS_EXPIRY");
        }

        let issuer = env::var("JWT_ISSUER").unwrap_or_else(|_| "Ekstion".to_string());

        Self {
            secret_key,
            issuer,
            access_token_expiry,
            refresh_token_expiry,
            verification_token_expiry,
        }
    }
}
