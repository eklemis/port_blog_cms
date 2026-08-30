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
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct TopicResult {
    /// Topic identifier
    #[schema(example = "9f1b2c3d-4e5f-6a7b-8c9d-0e1f2a3b4c5d")]
    pub id: Uuid,

    /// Identifier of the owning user. Serialises as a bare UUID string, so it
    /// is described as `String` rather than pulling `UserId` into the schema.
    #[schema(value_type = String, example = "123e4567-e89b-12d3-a456-426614174000")]
    pub owner: UserId,

    /// Topic title
    #[schema(example = "Distributed Systems")]
    pub title: String,

    /// Topic description
    #[schema(example = "Notes and projects on consensus and replication")]
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
