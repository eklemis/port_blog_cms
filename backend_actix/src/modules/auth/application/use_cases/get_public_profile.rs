//! Reading an author's public profile.
//!
//! Every public page is keyed on `{username}` but nothing returned who that
//! was, so a public site could not introduce the person whose work it was
//! showing.

use async_trait::async_trait;

use crate::multimedia::application::domain::entities::PublicMedia;

/// An author, as a reader sees them.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct PublicProfile {
    /// Public handle. The same value that keys every public URL.
    #[schema(example = "janedoe")]
    pub username: String,

    /// Display name.
    #[schema(example = "Jane Doe")]
    pub full_name: String,

    /// Free-form introduction. `null` when the author has not written one.
    #[schema(example = "Backend engineer, mostly Rust.")]
    pub bio: Option<String>,

    /// The author's avatar, or `null` when they have not uploaded one.
    ///
    /// Backed by the ordinary media machinery — an attachment on the user with
    /// role `avatar` — so it carries the same generated sizes and the same
    /// public URLs as any other image.
    pub avatar: Option<PublicMedia>,
}

/// Why a public profile could not be read.
#[derive(Debug, Clone, thiserror::Error)]
pub enum GetPublicProfileError {
    /// No such username, or the account is deleted. The two are the same
    /// answer: a deleted author has no public pages either.
    #[error("User not found")]
    NotFound,

    /// The store could not be reached.
    #[error("Query error: {0}")]
    QueryError(String),
}

/// Reads an author's public profile.
#[async_trait]
pub trait GetPublicProfileUseCase: Send + Sync {
    /// Returns the profile behind a public username.
    ///
    /// Deliberately returns no email and no account state: this is the only
    /// endpoint in the product that serves one user's details to another, so
    /// it carries the minimum a page needs to introduce them.
    async fn execute(&self, username: &str) -> Result<PublicProfile, GetPublicProfileError>;
}
