//! Read-side port for user records.
//!
//! Paired with [`UserRepository`](super::user_repository::UserRepository),
//! which owns writes.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// A full user row.
///
/// Carries the password hash, because login needs to verify against it. Do not
/// let this type reach a response DTO.
#[derive(Debug, Clone)]
pub struct UserQueryResult {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub password_hash: String,
    pub full_name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_verified: bool,
    pub is_deleted: bool,
}

/// Why a user read failed.
///
/// Note there is no "not found" variant: absence is `Ok(None)`, not an
/// error. Both variants here mean the store itself failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum UserQueryError {
    /// The store could not be reached.
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// The store was reached but the query did not execute — a malformed
    /// statement or a schema mismatch.
    #[error("Query execution failed: {0}")]
    QueryFailed(String),
}

/// Reads from the user store.
///
/// # A user that does not exist is `Ok(None)`, not an error
///
/// Every lookup returns `Ok(None)` when nothing matched. An `Err` always means
/// the store failed, never that the user is absent. Callers decide what a
/// missing user means for their endpoint — login reports invalid credentials,
/// a profile fetch reports 404.
///
/// # Soft-deleted users are returned
///
/// None of these filter on `is_deleted`. A deleted account comes back as
/// `Ok(Some(_))` with [`UserQueryResult::is_deleted`] set, and **the caller
/// must check that flag**. This is deliberate — password reset and account
/// restore both need to find deleted users — but it means a caller that
/// forgets the check will happily authenticate a deleted account.
#[async_trait]
pub trait UserQuery: Send + Sync {
    /// Looks a user up by primary key.
    async fn find_by_id(&self, user_id: Uuid) -> Result<Option<UserQueryResult>, UserQueryError>;

    /// Looks a user up by email address.
    ///
    /// Matching is exact, so callers must normalise case before calling if
    /// they want case-insensitive lookup.
    async fn find_by_email(&self, email: &str) -> Result<Option<UserQueryResult>, UserQueryError>;

    /// Looks a user up by username.
    ///
    /// Used by the public profile routes, which address users by name rather
    /// than id.
    async fn find_by_username(
        &self,
        username: &str,
    ) -> Result<Option<UserQueryResult>, UserQueryError>;
}
