use std::env;

#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub secret_key: String,
    pub issuer: String,
    pub access_token_expiry: i64,  // Expiration in seconds
    pub refresh_token_expiry: i64, // Expiration in seconds
}

impl JwtConfig {
    /// Load JWT configuration from environment variables
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok(); // Load environment variables if available

        let secret_key = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
        let access_token_expiry = env::var("JWT_ACCESS_EXPIRY")
            .unwrap_or_else(|_| "3600".to_string()) // Default 1 hour
            .parse::<i64>()
            .expect("Invalid JWT_ACCESS_EXPIRY value");

        let refresh_token_expiry = env::var("JWT_REFRESH_EXPIRY")
            .unwrap_or_else(|_| "604800".to_string()) // Default 7 days
            .parse::<i64>()
            .expect("Invalid JWT_REFRESH_EXPIRY value");

        Self {
            secret_key,
            issuer: String::from("Ekstion"),
            access_token_expiry,
            refresh_token_expiry,
        }
    }
}
