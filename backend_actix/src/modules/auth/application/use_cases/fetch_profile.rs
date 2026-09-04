use async_trait::async_trait;

use crate::auth::application::{
    domain::entities::UserId, ports::outgoing::user_query::UserQueryError,
};

/// The profile fields returned to their owner.
#[derive(Clone, Debug)]
pub struct FetchUserOutput {
    /// The user's identifier.
    pub user_id: UserId,
    /// Login address. Only ever returned to the account's owner.
    pub email: String,
    /// Public handle.
    pub username: String,
    /// Display name.
    pub full_name: String,
    /// Public bio. `None` when the user has not written one.
    pub bio: Option<String>,
    /// Interface language.
    pub locale: String,
}

/// Why a profile read failed.
#[derive(Debug, thiserror::Error, Clone)]
pub enum FetchUserError {
    /// No user matched the id.
    #[error("User not found: {0}")]
    UserNotFound(String),

    /// The user store could not be reached.
    #[error("Query error: {0}")]
    QueryError(#[from] UserQueryError),
}

/// Reads the authenticated user's own profile.
#[async_trait]
pub trait FetchUserProfileUseCase: Send + Sync {
    /// Returns the profile.
    async fn execute(&self, user_id: UserId) -> Result<FetchUserOutput, FetchUserError>;
}
