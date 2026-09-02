use async_trait::async_trait;

use crate::auth::application::{
    domain::entities::UserId,
    ports::outgoing::{user_query::UserQueryError, UserRepositoryError},
};

/// The profile as stored after the edit.
#[derive(Clone, Debug)]
pub struct UpdateUserOutput {
    /// The user's identifier.
    pub user_id: UserId,
    /// Public handle. Not editable through this use case.
    pub username: String,
    /// Login address. Not editable through this use case — changing it would
    /// need re-verification.
    pub email: String,
    /// Display name.
    pub full_name: String,
    /// Public bio as stored after the edit.
    pub bio: Option<String>,
}

/// A profile edit. Only the display name is editable today.
#[derive(Clone, Debug)]
pub struct UpdateUserInput {
    /// The user's identifier.
    pub user_id: UserId,
    /// Display name.
    pub full_name: String,
    /// New bio. `None` leaves the stored value alone; `Some(None)` clears it.
    pub bio: Option<Option<String>>,
}

/// Why the edit failed.
#[derive(Debug, thiserror::Error, Clone)]
pub enum UpdateUserError {
    /// The display name is empty or too long.
    #[error("Invalid full name: {0}")]
    InvalidFullName(String),

    /// The write failed.
    #[error("Repository error: {0}")]
    RepositoryError(#[from] UserRepositoryError),

    /// The user could not be read back.
    #[error("Query error: {0}")]
    QueryError(#[from] UserQueryError),
}

/// Edits the authenticated user's own profile.
#[async_trait]
pub trait UpdateUserProfileUseCase: Send + Sync {
    /// Applies the edit and returns the stored profile.
    async fn execute(&self, data: UpdateUserInput) -> Result<UpdateUserOutput, UpdateUserError>;
}
