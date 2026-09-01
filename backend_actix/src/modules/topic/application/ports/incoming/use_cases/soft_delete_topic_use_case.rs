//! Archiving a topic.

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;

/// Why archiving a topic failed.
///
/// Unlike the blog and project archivers, this distinguishes
/// [`Forbidden`](Self::Forbidden) from
/// [`TopicNotFound`](Self::TopicNotFound), because
/// [`TopicRepository::soft_delete_topic`](crate::topic::application::ports::outgoing::TopicRepository::soft_delete_topic)
/// does not scope on owner — the use case reads the topic and checks
/// ownership itself.
#[derive(Debug, Clone, thiserror::Error)]
pub enum SoftDeleteTopicError {
    /// No topic matched the id.
    #[error("Topic not found")]
    TopicNotFound,

    /// The topic exists but belongs to another user. Distinguishable here
    /// because the repository does not scope on owner — the use case reads the
    /// topic and checks.
    #[error("You are not the owner of this topic")]
    Forbidden,

    /// The store could not be reached.
    #[error("Database error: {0}")]
    DatabaseError(String),
}

/// Hides a topic without deleting it.
///
/// Existing blog-post and project links are left intact, so restoring the
/// topic brings them back rather than orphaning them.
#[async_trait]
pub trait SoftDeleteTopicUseCase: Send + Sync {
    /// Archives the topic, after verifying `owner` owns it.
    async fn execute(&self, owner: UserId, topic_id: Uuid) -> Result<(), SoftDeleteTopicError>;
}
