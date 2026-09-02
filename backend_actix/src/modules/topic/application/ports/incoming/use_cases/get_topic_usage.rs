//! Counting what a topic is attached to, before retiring it.

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::topic::application::ports::outgoing::TopicUsage;

/// Why a usage count could not be produced.
#[derive(Debug, Clone, thiserror::Error)]
pub enum GetTopicUsageError {
    /// The store could not be reached.
    #[error("Query failed: {0}")]
    QueryFailed(String),
}

/// Reports how many posts and projects carry a topic.
#[async_trait]
pub trait GetTopicUsageUseCase: Send + Sync {
    /// Counts live posts and projects referencing the topic.
    ///
    /// An unused topic, an unknown one, and one belonging to another user all
    /// report `{0, 0}`. The caller is about to be told "not found" by whatever
    /// operation it actually wants; this endpoint's job is a number for a
    /// confirmation dialog, not an existence check.
    async fn execute(
        &self,
        owner: UserId,
        topic_id: Uuid,
    ) -> Result<TopicUsage, GetTopicUsageError>;
}
