use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;

#[derive(Debug, Clone, thiserror::Error)]
pub enum BlogPostTopicRepositoryError {
    #[error("Blog post not found")]
    PostNotFound,

    #[error("Topic not found")]
    TopicNotFound,

    #[error("Database error: {0}")]
    DatabaseError(String),
}

#[async_trait]
pub trait BlogPostTopicRepository: Send + Sync {
    /// Attaching a topic already attached succeeds without inserting a second
    /// row; the join table's composite primary key makes that a no-op.
    async fn attach(
        &self,
        owner: UserId,
        post_id: Uuid,
        topic_id: Uuid,
    ) -> Result<(), BlogPostTopicRepositoryError>;

    async fn detach(
        &self,
        owner: UserId,
        post_id: Uuid,
        topic_id: Uuid,
    ) -> Result<(), BlogPostTopicRepositoryError>;

    async fn clear(
        &self,
        owner: UserId,
        post_id: Uuid,
    ) -> Result<(), BlogPostTopicRepositoryError>;
}
