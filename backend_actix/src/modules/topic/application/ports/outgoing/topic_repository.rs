use async_trait::async_trait;
use serde::Serialize;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;

// Input DTO for creating a user
#[derive(Debug, Clone)]
pub struct CreateTopicData {
    pub owner: UserId,
    pub title: String,
    pub description: String,
}

// Unified output DTO for all user operations that return user data
// This represents the essential user information after any state change
#[derive(Debug, Clone, Serialize)]
pub struct TopicResult {
    pub id: Uuid,
    pub owner: UserId,
    pub title: String,
    pub description: String,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum TopicRepositoryError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Topic not found")]
    TopicNotFound,

    #[error("Topic already exists")]
    TopicAlreadyExists,
}

#[async_trait]
pub trait TopicRepository: Send + Sync {
    async fn create_topic(
        &self,
        data: CreateTopicData,
    ) -> Result<TopicResult, TopicRepositoryError>;

    async fn restore_topic(&self, topic_id: Uuid) -> Result<TopicResult, TopicRepositoryError>;

    async fn soft_delete_topic(&self, topic_id: Uuid) -> Result<(), TopicRepositoryError>;
}
