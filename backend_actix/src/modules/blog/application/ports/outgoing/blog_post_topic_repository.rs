//! Write-side port for the `blog_post_topics` join table.
//!
//! Scoped on `owner` in SQL, like the project equivalent.

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;

/// Why a post-topic link operation failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum BlogPostTopicRepositoryError {
    #[error("Blog post not found")]
    PostNotFound,

    #[error("Topic not found")]
    TopicNotFound,

    #[error("Database error: {0}")]
    DatabaseError(String),
}

/// Links blog posts to topics.
#[async_trait]
pub trait BlogPostTopicRepository: Send + Sync {
    /// Attaching a topic already attached succeeds without inserting a second
    /// row; the join table's composite primary key makes that a no-op.
    /// Links one topic to a post. Idempotent — re-attaching an existing link
    /// succeeds.
    async fn attach(
        &self,
        owner: UserId,
        post_id: Uuid,
        topic_id: Uuid,
    ) -> Result<(), BlogPostTopicRepositoryError>;

    /// Removes one link. Detaching a link that is not there succeeds.
    async fn detach(
        &self,
        owner: UserId,
        post_id: Uuid,
        topic_id: Uuid,
    ) -> Result<(), BlogPostTopicRepositoryError>;

    /// Removes every topic link for a post.
    async fn clear(&self, owner: UserId, post_id: Uuid)
        -> Result<(), BlogPostTopicRepositoryError>;
}
