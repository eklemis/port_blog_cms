//! Listing a user's topics.

use async_trait::async_trait;

use crate::{
    auth::application::domain::entities::UserId,
    topic::application::ports::outgoing::TopicQueryResult,
};

/// Why listing topics failed.
///
/// A user with no topics gets an empty `Vec`, not an error.
#[derive(Debug, Clone, thiserror::Error)]
pub enum GetTopicsError {
    /// The store could not be reached.
    #[error("Failed to fetch topics: {0}")]
    QueryFailed(String),
}

/// Lists the caller's topics.
#[async_trait]
pub trait GetTopicsUseCase: Send + Sync {
    /// Returns every topic owned by `owner`, excluding soft-deleted ones.
    async fn execute(&self, owner: UserId) -> Result<Vec<TopicQueryResult>, GetTopicsError>;
}
