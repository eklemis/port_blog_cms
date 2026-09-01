use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;

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
#[async_trait]
pub trait BlogPostArchiver: Send + Sync {
    async fn soft_delete(&self, owner: UserId, post_id: Uuid) -> Result<(), BlogPostArchiverError>;

    async fn restore(&self, owner: UserId, post_id: Uuid) -> Result<(), BlogPostArchiverError>;

    async fn hard_delete(&self, owner: UserId, post_id: Uuid) -> Result<(), BlogPostArchiverError>;
}
