use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    auth::application::domain::entities::UserId,
    topic::application::ports::incoming::use_cases::{
        SoftDeleteTopicError, SoftDeleteTopicUseCase,
    },
    topic::application::ports::outgoing::{TopicQuery, TopicRepository, TopicRepositoryError},
};

#[derive(Debug, Clone)]
pub struct SoftDeleteTopicService<Q, R>
where
    Q: TopicQuery,
    R: TopicRepository,
{
    query: Q,
    repository: R,
}

impl<Q, R> SoftDeleteTopicService<Q, R>
where
    Q: TopicQuery,
    R: TopicRepository,
{
    pub fn new(query: Q, repository: R) -> Self {
        Self { query, repository }
    }
}

#[async_trait]
impl<Q, R> SoftDeleteTopicUseCase for SoftDeleteTopicService<Q, R>
where
    Q: TopicQuery + Send + Sync,
    R: TopicRepository + Send + Sync,
{
    async fn execute(&self, owner: UserId, topic_id: Uuid) -> Result<(), SoftDeleteTopicError> {
        // 1️⃣ Load topics for owner
        let topics = self
            .query
            .get_topics(owner.clone())
            .await
            .map_err(|e| SoftDeleteTopicError::DatabaseError(e.to_string()))?;

        // 2️⃣ Ensure ownership
        let owns_topic = topics.iter().any(|t| t.id == topic_id);
        if !owns_topic {
            return Err(SoftDeleteTopicError::Forbidden);
        }

        // 3️⃣ Soft delete
        self.repository
            .soft_delete_topic(topic_id)
            .await
            .map_err(|e| match e {
                TopicRepositoryError::TopicNotFound => SoftDeleteTopicError::TopicNotFound,
                TopicRepositoryError::DatabaseError(msg) => {
                    SoftDeleteTopicError::DatabaseError(msg)
                }
                _ => SoftDeleteTopicError::DatabaseError(e.to_string()),
            })?;

        Ok(())
    }
}
