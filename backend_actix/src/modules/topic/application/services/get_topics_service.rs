use async_trait::async_trait;

use crate::{
    auth::application::domain::entities::UserId,
    topic::application::{
        ports::incoming::use_cases::{GetTopicsError, GetTopicsUseCase},
        ports::outgoing::{TopicQuery, TopicQueryResult},
    },
};

#[derive(Debug, Clone)]
pub struct GetTopicsService<Q>
where
    Q: TopicQuery + Send + Sync,
{
    query: Q,
}

impl<Q> GetTopicsService<Q>
where
    Q: TopicQuery + Send + Sync,
{
    pub fn new(query: Q) -> Self {
        Self { query }
    }
}

#[async_trait]
impl<Q> GetTopicsUseCase for GetTopicsService<Q>
where
    Q: TopicQuery + Send + Sync,
{
    async fn execute(&self, owner: UserId) -> Result<Vec<TopicQueryResult>, GetTopicsError> {
        self.query
            .get_topics(owner)
            .await
            .map_err(|e| GetTopicsError::QueryFailed(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use uuid::Uuid;

    use crate::{
        auth::application::domain::entities::UserId,
        topic::application::ports::outgoing::{TopicQuery, TopicQueryError, TopicQueryResult},
    };

    // ============================================================
    // Mock Query
    // ============================================================

    #[derive(Clone)]
    struct MockTopicQuery {
        result: Result<Vec<TopicQueryResult>, TopicQueryError>,
    }

    impl MockTopicQuery {
        fn success(data: Vec<TopicQueryResult>) -> Self {
            Self { result: Ok(data) }
        }

        fn failure(message: &str) -> Self {
            Self {
                result: Err(TopicQueryError::DatabaseError(message.to_string())),
            }
        }
    }

    #[async_trait]
    impl TopicQuery for MockTopicQuery {
        async fn get_topics(
            &self,
            _owner: UserId,
        ) -> Result<Vec<TopicQueryResult>, TopicQueryError> {
            self.result.clone()
        }
    }

    // ============================================================
    // Helpers
    // ============================================================

    fn create_topic(id: Uuid, owner: UserId, title: &str) -> TopicQueryResult {
        TopicQueryResult {
            id,
            owner,
            title: title.to_string(),
            description: "desc".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    // ============================================================
    // Tests
    // ============================================================

    #[tokio::test]
    async fn test_get_topics_success_with_results() {
        // Arrange
        let owner = UserId::from(Uuid::new_v4());

        let topics = vec![
            create_topic(Uuid::new_v4(), owner.clone(), "Rust"),
            create_topic(Uuid::new_v4(), owner.clone(), "Backend"),
        ];

        let query = MockTopicQuery::success(topics.clone());
        let service = GetTopicsService::new(query);

        // Act
        let result = service.execute(owner).await;

        // Assert
        assert!(result.is_ok());
        let returned = result.unwrap();
        assert_eq!(returned.len(), 2);
        assert_eq!(returned[0].title, "Rust");
        assert_eq!(returned[1].title, "Backend");
    }

    #[tokio::test]
    async fn test_get_topics_success_empty_list() {
        // Arrange
        let owner = UserId::from(Uuid::new_v4());

        let query = MockTopicQuery::success(vec![]);
        let service = GetTopicsService::new(query);

        // Act
        let result = service.execute(owner).await;

        // Assert
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_topics_query_failure() {
        // Arrange
        let owner = UserId::from(Uuid::new_v4());

        let query = MockTopicQuery::failure("db down");
        let service = GetTopicsService::new(query);

        // Act
        let result = service.execute(owner).await;

        // Assert
        match result {
            Err(GetTopicsError::QueryFailed(msg)) => {
                assert!(msg.contains("db down"));
            }
            other => panic!("Expected QueryFailed error, got {:?}", other),
        }
    }

    #[test]
    fn test_get_topics_service_is_cloneable() {
        // Arrange
        let query = MockTopicQuery::success(vec![]);
        let service = GetTopicsService::new(query);

        // Act
        let _cloned = service.clone();

        // Assert
        assert!(true); // compile-time guarantee
    }
}
