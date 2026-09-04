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
    /// Primary key.
    pub id: Uuid,
    /// Login identifier and the address mail is sent to.
    pub email: String,
    /// Public handle. Appears in public URLs, so it is visible to anyone.
    pub username: String,
    /// Argon2 hash, carrying its own salt and parameters.
    /// Never let this reach a response DTO.
    pub password_hash: String,
    /// Display name, shown to the user and used to greet them in mail.
    pub full_name: String,

    /// Free-form introduction shown on the author's public pages.
    ///
    /// `None` when never set — distinct from an empty string, which the author
    /// chose. Lives on the user rather than per-CV because a reader's URL names
    /// a person, not a document.
    pub bio: Option<String>,

    /// The language the interface is shown in. A property of the person, not
    /// of anything they write — see the migration for why those are separate.
    pub locale: String,
    /// When the account was created.
    pub created_at: DateTime<Utc>,
    /// When the row was last written.
    pub updated_at: DateTime<Utc>,
    /// Whether the email address has been confirmed. Endpoints that require a
    /// verified account must check this.
    pub is_verified: bool,
    /// Whether the account is soft-deleted.
    ///
    /// **The query does not filter on this** — a deleted account is returned
    /// with the flag set, and the caller must check it. See the trait
    /// documentation.
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
