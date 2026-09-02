//! Validates a topic edit, then writes it.

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::topic::application::ports::incoming::use_cases::{PatchTopicError, PatchTopicUseCase};
use crate::topic::application::ports::outgoing::{
    TopicRepository, TopicRepositoryError, TopicResult,
};

/// Implements the corresponding use-case contract.
#[derive(Debug, Clone)]
pub struct PatchTopicService<R> {
    repository: R,
}

impl<R> PatchTopicService<R> {
    /// Builds it from the ports it depends on.
    pub fn new(repository: R) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl<R: TopicRepository + Send + Sync> PatchTopicUseCase for PatchTopicService<R> {
    async fn execute(
        &self,
        owner: UserId,
        topic_id: Uuid,
        title: Option<String>,
        description: Option<String>,
    ) -> Result<TopicResult, PatchTopicError> {
        // Validated here rather than in the repository, and with the same
        // rules `CreateTopicCommand` applies — a title that could not be
        // created should not be reachable by renaming into it.
        let title = match title {
            Some(t) => {
                let t = t.trim().to_string();
                if t.is_empty() {
                    return Err(PatchTopicError::EmptyTitle);
                }
                if t.len() > 100 {
                    return Err(PatchTopicError::TitleTooLong);
                }
                Some(t)
            }
            None => None,
        };

        self.repository
            .patch_topic(owner, topic_id, title, description)
            .await
            .map_err(|e| match e {
                TopicRepositoryError::TopicNotFound => PatchTopicError::TopicNotFound,
                TopicRepositoryError::TopicAlreadyExists => PatchTopicError::TopicAlreadyExists,
                TopicRepositoryError::DatabaseError(m) => PatchTopicError::RepositoryError(m),
            })
    }
}
