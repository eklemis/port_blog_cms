use serde::{Deserialize, Deserializer};
use std::fmt;

use crate::modules::auth::application::services::jwt::{JwtError, JwtService};
use async_trait::async_trait;
use std::sync::Arc;

use crate::modules::auth::application::ports::outgoing::{
    token_repository::TokenRepository,
    user_repository::{UserRepository, UserRepositoryError},
};

/// ========================= Soft Delete Request =========================
///
/// Encapsulates and validates the access token required to soft-delete a user.
///
/// Design goals:
/// - The access token is private and immutable
/// - Validation happens at construction time
/// - Use cases operate on a *valid request only*
#[derive(Debug, Clone)]
pub struct SoftDeleteRequest {
    access_token: String,
}

impl SoftDeleteRequest {
    /// Create a new `SoftDeleteRequest`.
    ///
    /// This constructor:
    /// - trims the token
    /// - rejects empty tokens
    ///
    /// JWT cryptographic validation is intentionally **not**
    /// done here to keep this type deterministic and testable.
    pub fn new(access_token: String) -> Result<Self, SoftDeleteRequestError> {
        let token = access_token.trim();

        if token.is_empty() {
            return Err(SoftDeleteRequestError::MissingAccessToken);
        }

        Ok(Self {
            access_token: token.to_string(),
        })
    }

    /// Returns the validated access token.
    pub fn access_token(&self) -> &str {
        &self.access_token
    }
}

/// ========================= Request Error =========================
#[derive(Debug, Clone)]
pub enum SoftDeleteRequestError {
    MissingAccessToken,
}

impl fmt::Display for SoftDeleteRequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SoftDeleteRequestError::MissingAccessToken => {
                write!(f, "Access token is required")
            }
        }
    }
}

impl std::error::Error for SoftDeleteRequestError {}

/// Enable deserialization from HTTP payloads if needed
impl<'de> Deserialize<'de> for SoftDeleteRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            access_token: String,
        }

        let helper = Helper::deserialize(deserializer)?;
        SoftDeleteRequest::new(helper.access_token).map_err(serde::de::Error::custom)
    }
}

// Use case
#[async_trait]
pub trait ISoftDeleteUserUseCase: Send + Sync {
    async fn execute(&self, request: SoftDeleteRequest) -> Result<(), SoftDeleteUserError>;
}

// Service
/// ========================= Use Case Error =========================
#[derive(Debug)]
pub enum SoftDeleteUserError {
    Unauthorized,
    InvalidToken,
    UserNotFound,
    InfrastructureError(String),
}

impl std::fmt::Display for SoftDeleteUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SoftDeleteUserError::Unauthorized => write!(f, "Unauthorized"),
            SoftDeleteUserError::InvalidToken => write!(f, "Invalid token"),
            SoftDeleteUserError::UserNotFound => write!(f, "User not found"),
            SoftDeleteUserError::InfrastructureError(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for SoftDeleteUserError {}

/// ========================= Use Case =========================
pub struct SoftDeleteUserUseCase {
    user_repository: Arc<dyn UserRepository + Send + Sync>,
    token_repository: Arc<dyn TokenRepository + Send + Sync>,
    jwt_service: JwtService,
}

impl SoftDeleteUserUseCase {
    pub fn new(
        user_repository: Arc<dyn UserRepository + Send + Sync>,
        token_repository: Arc<dyn TokenRepository + Send + Sync>,
        jwt_service: JwtService,
    ) -> Self {
        Self {
            user_repository,
            token_repository,
            jwt_service,
        }
    }
}

#[async_trait]
impl ISoftDeleteUserUseCase for SoftDeleteUserUseCase {
    async fn execute(&self, request: SoftDeleteRequest) -> Result<(), SoftDeleteUserError> {
        // 1️⃣ Verify access token
        let claims = self
            .jwt_service
            .verify_token(request.access_token())
            .map_err(|e| match e {
                JwtError::TokenExpired | JwtError::InvalidSignature => {
                    SoftDeleteUserError::Unauthorized
                }
                _ => SoftDeleteUserError::InvalidToken,
            })?;

        // Enforce access token usage
        if claims.token_type != "access" {
            return Err(SoftDeleteUserError::InvalidToken);
        }

        let user_id = claims.sub;

        // 2️⃣ Soft delete user
        self.user_repository
            .soft_delete_user(user_id)
            .await
            .map_err(|e| match e {
                UserRepositoryError::UserNotFound => SoftDeleteUserError::UserNotFound,
                UserRepositoryError::DatabaseError(msg) => {
                    SoftDeleteUserError::InfrastructureError(msg)
                }
                _ => SoftDeleteUserError::InfrastructureError("Unknown error".into()),
            })?;

        // 3️⃣ Revoke all tokens (defense-in-depth)
        self.token_repository
            .revoke_all_user_tokens(user_id)
            .await
            .map_err(|e| SoftDeleteUserError::InfrastructureError(e.to_string()))?;

        Ok(())
    }
}
