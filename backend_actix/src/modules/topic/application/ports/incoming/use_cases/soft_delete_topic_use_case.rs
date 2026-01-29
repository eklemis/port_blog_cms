use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;

#[derive(Debug, Clone, thiserror::Error)]
pub enum SoftDeleteTopicError {
    #[error("Topic not found")]
    TopicNotFound,

    #[error("You are not the owner of this topic")]
    Forbidden,

    #[error("Database error: {0}")]
    DatabaseError(String),
}

#[async_trait]
pub trait SoftDeleteTopicUseCase: Send + Sync {
    async fn execute(&self, owner: UserId, topic_id: Uuid) -> Result<(), SoftDeleteTopicError>;
}
