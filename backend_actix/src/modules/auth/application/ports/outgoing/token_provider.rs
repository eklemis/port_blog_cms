use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use uuid::Uuid;

#[derive(Debug)]
pub enum TokenError {
    TokenExpired,
    TokenNotYetValid,
    InvalidTokenType(String),
    InvalidSignature,
    MalformedToken,
    EncodingError(String),
}

impl fmt::Display for TokenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenError::TokenExpired => write!(f, "Token has expired"),
            TokenError::TokenNotYetValid => write!(f, "Token is not yet valid"), // ADD THIS
            TokenError::InvalidTokenType(expected) => {
                write!(f, "Invalid token type, expected: {}", expected)
            }
            TokenError::InvalidSignature => write!(f, "Invalid token signature"),
            TokenError::MalformedToken => write!(f, "Malformed token"),
            TokenError::EncodingError(msg) => write!(f, "Token encoding error: {}", msg),
        }
    }
}
impl Error for TokenError {}

/// Structure for JWT Claims
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenClaims {
    pub sub: Uuid,          // User ID
    pub exp: i64,           // Expiration timestamp
    pub iat: i64,           // Issued at timestamp - ADD THIS
    pub nbf: i64,           // Not before timestamp - ADD THIS
    pub token_type: String, // "access", "refresh", or "verification"
    pub is_verified: bool,  // User verification status
}

pub trait TokenProvider: Send + Sync {
    fn generate_access_token(&self, user_id: Uuid, is_verified: bool)
        -> Result<String, TokenError>;
    fn generate_refresh_token(
        &self,
        user_id: Uuid,
        is_verified: bool,
    ) -> Result<String, TokenError>;
    fn verify_token(&self, token: &str) -> Result<TokenClaims, TokenError>;
    fn refresh_access_token(&self, refresh_token: &str) -> Result<String, TokenError>;
    fn generate_verification_token(&self, user_id: Uuid) -> Result<String, TokenError>;
    fn verify_verification_token(&self, token: &str) -> Result<Uuid, TokenError>;
}
