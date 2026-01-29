use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;

/// Read-only DTO for topic queries
/// Contains all persisted fields except `is_deleted`
#[derive(Debug, Clone)]
pub struct TopicQueryResult {
    pub id: Uuid,
    pub owner: UserId,
    pub title: String,
    pub description: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum TopicQueryError {
    #[error("Database error: {0}")]
    DatabaseError(String),
}

#[async_trait]
pub trait TopicQuery: Send + Sync {
    async fn get_topics(&self, owner: UserId) -> Result<Vec<TopicQueryResult>, TopicQueryError>;
}
