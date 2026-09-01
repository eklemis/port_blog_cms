use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    auth::application::domain::entities::UserId,
    topic::application::ports::incoming::use_cases::{
        SoftDeleteTopicError, SoftDeleteTopicUseCase,
    },
    topic::application::ports::outgoing::{TopicQuery, TopicRepository, TopicRepositoryError},
};

/// Implements the corresponding use-case contract.
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
    /// Builds it from the ports it depends on.
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
            .get_topics(owner)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topic::application::ports::outgoing::{
        CreateTopicData, TopicQueryError, TopicQueryResult, TopicResult,
    };
    use chrono::Utc;
    use std::sync::Mutex;

    struct MockQuery {
        result: Result<Vec<TopicQueryResult>, TopicQueryError>,
    }

    #[async_trait]
    impl TopicQuery for MockQuery {
        async fn get_topics(&self, _o: UserId) -> Result<Vec<TopicQueryResult>, TopicQueryError> {
            self.result.clone()
        }
    }

    #[derive(Default)]
    struct SpyRepo {
        deleted: Mutex<Vec<Uuid>>,
        result: Option<TopicRepositoryError>,
    }

    #[async_trait]
    impl TopicRepository for SpyRepo {
        async fn create_topic(
            &self,
            _d: CreateTopicData,
        ) -> Result<TopicResult, TopicRepositoryError> {
            unimplemented!()
        }
        async fn restore_topic(&self, _t: Uuid) -> Result<TopicResult, TopicRepositoryError> {
            unimplemented!()
        }
        async fn soft_delete_topic(&self, topic_id: Uuid) -> Result<(), TopicRepositoryError> {
            self.deleted.lock().unwrap().push(topic_id);
            match &self.result {
                Some(e) => Err(e.clone()),
                None => Ok(()),
            }
        }
    }

    fn a_topic(id: Uuid, owner: UserId) -> TopicQueryResult {
        TopicQueryResult {
            id,
            owner,
            title: "Rust".into(),
            description: "d".into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn deletes_a_topic_the_caller_owns() {
        let owner = UserId::from(Uuid::new_v4());
        let topic_id = Uuid::new_v4();

        let svc = SoftDeleteTopicService::new(
            MockQuery {
                result: Ok(vec![a_topic(topic_id, owner)]),
            },
            SpyRepo::default(),
        );

        svc.execute(owner, topic_id).await.unwrap();
        assert_eq!(
            svc.repository.deleted.lock().unwrap().as_slice(),
            [topic_id]
        );
    }

    /// Ownership is established by listing the caller's topics and checking
    /// membership, because soft_delete_topic takes only an id. Without this a
    /// caller could delete anyone's topic by guessing a UUID.
    #[tokio::test]
    async fn refuses_a_topic_the_caller_does_not_own() {
        let owner = UserId::from(Uuid::new_v4());

        let svc = SoftDeleteTopicService::new(
            MockQuery {
                result: Ok(vec![a_topic(Uuid::new_v4(), owner)]),
            },
            SpyRepo::default(),
        );

        let err = svc.execute(owner, Uuid::new_v4()).await.unwrap_err();
        assert!(matches!(err, SoftDeleteTopicError::Forbidden));
        // Nothing was written.
        assert!(svc.repository.deleted.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn an_owner_with_no_topics_is_forbidden_rather_than_not_found() {
        let owner = UserId::from(Uuid::new_v4());
        let svc = SoftDeleteTopicService::new(MockQuery { result: Ok(vec![]) }, SpyRepo::default());

        assert!(matches!(
            svc.execute(owner, Uuid::new_v4()).await.unwrap_err(),
            SoftDeleteTopicError::Forbidden
        ));
    }

    #[tokio::test]
    async fn surfaces_a_query_failure() {
        let owner = UserId::from(Uuid::new_v4());
        let svc = SoftDeleteTopicService::new(
            MockQuery {
                result: Err(TopicQueryError::DatabaseError("db down".into())),
            },
            SpyRepo::default(),
        );

        assert!(matches!(
            svc.execute(owner, Uuid::new_v4()).await.unwrap_err(),
            SoftDeleteTopicError::DatabaseError(m) if m.contains("db down")
        ));
    }

    /// The repository can still report NotFound after the ownership check
    /// passes — for instance if the row was deleted between the two calls.
    #[tokio::test]
    async fn maps_a_repository_not_found() {
        let owner = UserId::from(Uuid::new_v4());
        let topic_id = Uuid::new_v4();

        let svc = SoftDeleteTopicService::new(
            MockQuery {
                result: Ok(vec![a_topic(topic_id, owner)]),
            },
            SpyRepo {
                result: Some(TopicRepositoryError::TopicNotFound),
                ..Default::default()
            },
        );

        assert!(matches!(
            svc.execute(owner, topic_id).await.unwrap_err(),
            SoftDeleteTopicError::TopicNotFound
        ));
    }

    #[tokio::test]
    async fn maps_other_repository_errors_to_database_error() {
        let owner = UserId::from(Uuid::new_v4());
        let topic_id = Uuid::new_v4();

        for e in [
            TopicRepositoryError::DatabaseError("db down".into()),
            // Hits the catch-all arm.
            TopicRepositoryError::TopicAlreadyExists,
        ] {
            let svc = SoftDeleteTopicService::new(
                MockQuery {
                    result: Ok(vec![a_topic(topic_id, owner)]),
                },
                SpyRepo {
                    result: Some(e),
                    ..Default::default()
                },
            );

            assert!(matches!(
                svc.execute(owner, topic_id).await.unwrap_err(),
                SoftDeleteTopicError::DatabaseError(_)
            ));
        }
    }
}
