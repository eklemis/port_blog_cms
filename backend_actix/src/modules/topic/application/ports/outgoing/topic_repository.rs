//! Write-side port for topics.
//!
//! Topics are owned per user and act as shared vocabulary for blog posts and
//! projects, which link to them through their own join-table ports.

use async_trait::async_trait;
use serde::Serialize;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;

/// Everything needed to insert a topic.
#[derive(Debug, Clone)]
pub struct CreateTopicData {
    pub owner: UserId,
    pub title: String,
    pub description: String,
}

/// A topic as returned after a write, and as serialised to clients.
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

/// Why a topic write failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum TopicRepositoryError {
    /// The store could not be reached.
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// No topic matched the id, or it belongs to another user.
    #[error("Topic not found")]
    TopicNotFound,

    /// This owner already has a topic with that title. Uniqueness is per
    /// owner, so another user holding the title is not a conflict.
    #[error("Topic already exists")]
    TopicAlreadyExists,
}

/// Writes topics.
#[async_trait]
pub trait TopicRepository: Send + Sync {
    /// Inserts a topic.
    ///
    /// # Errors
    /// [`TopicAlreadyExists`](TopicRepositoryError::TopicAlreadyExists) if the
    /// owner already has one with that title.
    async fn create_topic(
        &self,
        data: CreateTopicData,
    ) -> Result<TopicResult, TopicRepositoryError>;

    /// Clears the soft-delete flag, returning the topic to
    /// [`TopicQuery::get_topics`](super::topic_query::TopicQuery::get_topics).
    ///
    /// Unlike the archiver ports in blog and project, this takes no owner and
    /// so does not scope on one — the caller must check ownership first.
    async fn restore_topic(&self, topic_id: Uuid) -> Result<TopicResult, TopicRepositoryError>;

    /// Flags the topic as deleted, hiding it from queries while leaving
    /// existing post and project links intact.
    async fn soft_delete_topic(&self, topic_id: Uuid) -> Result<(), TopicRepositoryError>;
}
