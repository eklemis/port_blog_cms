//! Counts a topic's references.

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::topic::application::ports::incoming::use_cases::{
    GetTopicUsageError, GetTopicUsageUseCase,
};
use crate::topic::application::ports::outgoing::{TopicQuery, TopicUsage};

/// Implements the corresponding use-case contract.
#[derive(Debug, Clone)]
pub struct GetTopicUsageService<Q> {
    query: Q,
}

impl<Q> GetTopicUsageService<Q> {
    /// Builds it from the ports it depends on.
    pub fn new(query: Q) -> Self {
        Self { query }
    }
}

#[async_trait]
impl<Q: TopicQuery + Send + Sync> GetTopicUsageUseCase for GetTopicUsageService<Q> {
    async fn execute(
        &self,
        owner: UserId,
        topic_id: Uuid,
    ) -> Result<TopicUsage, GetTopicUsageError> {
        self.query
            .get_topic_usage(owner, topic_id)
            .await
            .map_err(|e| GetTopicUsageError::QueryFailed(e.to_string()))
    }
}
