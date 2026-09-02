//! Renaming a topic.

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::topic::application::ports::outgoing::TopicResult;

/// Why a topic edit failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum PatchTopicError {
    /// The title was empty once trimmed.
    #[error("Title cannot be empty")]
    EmptyTitle,

    /// The title exceeds 100 characters, matching creation.
    #[error("Title too long")]
    TitleTooLong,

    /// No topic matched the id, or it belongs to another user.
    #[error("Topic not found")]
    TopicNotFound,

    /// The owner already has a topic with that title.
    #[error("Topic already exists")]
    TopicAlreadyExists,

    /// The store could not be reached.
    #[error("Repository error: {0}")]
    RepositoryError(String),
}

/// Edits a topic's title or description.
#[async_trait]
pub trait PatchTopicUseCase: Send + Sync {
    /// Applies the edit and returns the topic as stored.
    ///
    /// The topic keeps its id, so every post and project tagged with it
    /// follows the new name automatically — no retagging. That is the point:
    /// the workaround was create-retag-retire by hand across every tagged
    /// item, and a typo was otherwise permanent and visible everywhere.
    ///
    /// Omitted fields are left alone. Both omitted is a no-op that still
    /// returns the topic, so a caller can use it as a read.
    async fn execute(
        &self,
        owner: UserId,
        topic_id: Uuid,
        title: Option<String>,
        description: Option<String>,
    ) -> Result<TopicResult, PatchTopicError>;
}
