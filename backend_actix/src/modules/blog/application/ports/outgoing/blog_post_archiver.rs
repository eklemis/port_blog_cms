//! Lifecycle port for blog posts: soft delete, restore, hard delete.

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;

/// Why a blog-post lifecycle operation failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum BlogPostArchiverError {
    /// Post does not exist, or does not belong to the owner.
    #[error("Blog post not found")]
    NotFound,

    #[error("Database error: {0}")]
    DatabaseError(String),
}

/// Lifecycle operations, kept separate from the write path so a service can
/// depend on archiving without gaining the ability to edit content.
/// Lifecycle operations, kept separate from the write path so a service can
/// depend on archiving without gaining the ability to edit content.
///
/// Every method takes the `owner` and scopes on it **in SQL**, so a caller
/// cannot act on someone else's post even by passing a valid id. That is why
/// these return [`NotFound`](BlogPostArchiverError::NotFound) rather than a
/// separate "forbidden": at this layer the two are indistinguishable, and
/// collapsing them avoids confirming that a post exists.
#[async_trait]
pub trait BlogPostArchiver: Send + Sync {
    /// Flags the post as deleted, hiding it from queries while keeping the
    /// row. Reversible with [`restore`](Self::restore).
    async fn soft_delete(&self, owner: UserId, post_id: Uuid) -> Result<(), BlogPostArchiverError>;

    /// Clears the deleted flag. Publication state is untouched — a restored
    /// post returns as the draft or published post it was.
    async fn restore(&self, owner: UserId, post_id: Uuid) -> Result<(), BlogPostArchiverError>;

    /// Removes the row permanently, along with its topic links. Irreversible.
    async fn hard_delete(&self, owner: UserId, post_id: Uuid) -> Result<(), BlogPostArchiverError>;
}
