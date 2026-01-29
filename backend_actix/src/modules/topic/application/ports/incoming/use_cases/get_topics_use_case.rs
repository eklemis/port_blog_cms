use async_trait::async_trait;

use crate::{
    auth::application::domain::entities::UserId,
    topic::application::ports::outgoing::TopicQueryResult,
};

#[derive(Debug, Clone, thiserror::Error)]
pub enum GetTopicsError {
    #[error("Failed to fetch topics: {0}")]
    QueryFailed(String),
}

#[async_trait]
pub trait GetTopicsUseCase: Send + Sync {
    async fn execute(&self, owner: UserId) -> Result<Vec<TopicQueryResult>, GetTopicsError>;
}
