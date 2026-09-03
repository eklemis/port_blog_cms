//! Write-side port for user records.
//!
//! Paired with [`UserQuery`](super::user_query::UserQuery), which owns reads.
//! The split is deliberate: reads return whole rows including soft-deleted
//! ones, writes return only the fields a caller needs to confirm the new
//! state.

use async_trait::async_trait;
use uuid::Uuid;

/// Everything needed to insert a user.
///
/// The password arrives already hashed — this port never sees a plaintext
/// password, and hashing is [`PasswordHasher`](super::password_hasher::PasswordHasher)'s job.
#[derive(Debug, Clone)]
pub struct CreateUserData {
    /// Login identifier and the address mail is sent to.
    pub email: String,
    /// Public handle.
    pub username: String,
    /// Already-hashed password. This port never sees a plaintext one.
    pub password_hash: String,
    /// Display name.
    pub full_name: String,
}

/// The user fields returned after a successful write.
///
/// Deliberately narrower than [`UserQueryResult`](super::user_query::UserQueryResult):
/// it carries no password hash and no verification or deletion flags, because
/// a caller confirming a write does not need them.
#[derive(Debug, Clone)]
pub struct UserResult {
    /// Primary key of the stored user.
    pub id: Uuid,
    /// The stored email address.
    pub email: String,
    /// The stored username.
    pub username: String,
    /// The stored display name.
    pub full_name: String,
    /// The stored public bio, if the user has written one.
    pub bio: Option<String>,
    /// The stored interface language.
    pub locale: String,
}

/// Why a user write failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum UserRepositoryError {
    /// The store could not be reached, or rejected the statement for a reason
    /// this port does not model. Callers should treat it as a 500 and log the
    /// detail rather than showing it.
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// No row matched the id. Returned by every operation that targets an
    /// existing user; `create_user` never returns it.
    #[error("User not found")]
    UserNotFound,

    /// A unique constraint on email or username was violated.
    ///
    /// The port does not say *which* column collided, because the registration
    /// flow deliberately does not tell the caller either — that would confirm
    /// whether an address is registered.
    #[error("User already exists")]
    UserAlreadyExists,
}

/// Writes to the user store.
///
/// Implementors must map a unique-constraint violation on email or username to
/// [`UserRepositoryError::UserAlreadyExists`], and a write that matched no row
/// to [`UserRepositoryError::UserNotFound`]. Everything else is
/// [`DatabaseError`](UserRepositoryError::DatabaseError).
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// Inserts a user and returns the stored record.
    ///
    /// # Errors
    /// [`UserAlreadyExists`](UserRepositoryError::UserAlreadyExists) if the
    /// email or username is taken.
    async fn create_user(&self, data: CreateUserData) -> Result<UserResult, UserRepositoryError>;

    /// Clears the soft-delete flag, making the account usable again.
    ///
    /// Restoring an account that was never deleted succeeds and changes
    /// nothing; only a missing row is an error.
    async fn restore_user(&self, user_id: Uuid) -> Result<UserResult, UserRepositoryError>;

    /// Marks the user's email address as verified.
    ///
    /// Called after a verification token is redeemed. Idempotent — verifying an
    /// already-verified account is not an error.
    async fn activate_user(&self, user_id: Uuid) -> Result<UserResult, UserRepositoryError>;

    /// Replaces the user's public profile fields.
    ///
    /// `bio` is tri-state: `None` leaves it, `Some(None)` clears it,
    /// `Some(Some(text))` replaces it. The display name is always written,
    /// because the endpoint requires it.
    async fn set_profile(
        &self,
        user_id: Uuid,
        full_name: String,
        bio: Option<Option<String>>,
        locale: Option<String>,
    ) -> Result<UserResult, UserRepositoryError>;

    /// Replaces the user's display name.
    async fn set_full_name(
        &self,
        user_id: Uuid,
        full_name: String,
    ) -> Result<UserResult, UserRepositoryError>;

    /// Replaces the stored password hash.
    ///
    /// Takes a hash, never a plaintext password. Does not itself revoke
    /// existing sessions — callers that need that must also go through
    /// [`TokenRepository::revoke_all_user_tokens`](super::token_repository::TokenRepository::revoke_all_user_tokens).
    async fn update_password(
        &self,
        user_id: Uuid,
        new_password_hash: String,
    ) -> Result<(), UserRepositoryError>;

    /// Removes the row permanently.
    ///
    /// Irreversible, unlike [`soft_delete_user`](Self::soft_delete_user).
    async fn delete_user(&self, user_id: Uuid) -> Result<(), UserRepositoryError>;

    /// Flags the account as deleted while keeping the row.
    ///
    /// The row stays visible to [`UserQuery`](super::user_query::UserQuery),
    /// which does not filter on this flag — see that trait's documentation.
    /// Reversible with [`restore_user`](Self::restore_user).
    async fn soft_delete_user(&self, user_id: Uuid) -> Result<(), UserRepositoryError>;
}
