use async_trait::async_trait;

use crate::topic::application::ports::{
    incoming::use_cases::{CreateTopicCommand, CreateTopicError, CreateTopicUseCase},
    outgoing::{CreateTopicData, TopicRepository, TopicRepositoryError, TopicResult},
};

#[derive(Debug, Clone)]
pub struct CreateTopicService<R>
where
    R: TopicRepository + Send + Sync,
{
    repository: R,
}

impl<R> CreateTopicService<R>
where
    R: TopicRepository + Send + Sync,
{
    pub fn new(repository: R) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl<R> CreateTopicUseCase for CreateTopicService<R>
where
    R: TopicRepository + Send + Sync,
{
    async fn execute(&self, command: CreateTopicCommand) -> Result<TopicResult, CreateTopicError> {
        let data = CreateTopicData {
            owner: command.owner().clone(),
            title: command.title().to_string(),
            description: command.description().cloned().unwrap_or_default(),
        };

        self.repository
            .create_topic(data)
            .await
            .map_err(|e| match e {
                TopicRepositoryError::TopicAlreadyExists => CreateTopicError::TopicAlreadyExists,
                other => CreateTopicError::RepositoryError(other.to_string()),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use uuid::Uuid;

    use crate::{
        auth::application::domain::entities::UserId,
        topic::application::{
            ports::incoming::use_cases::{CreateTopicCommand, CreateTopicError},
            ports::outgoing::{
                CreateTopicData, TopicRepository, TopicRepositoryError, TopicResult,
            },
        },
    };

    // ──────────────────────────────────────────────────────────
    // Mock Repository
    // ──────────────────────────────────────────────────────────

    #[derive(Debug, Clone)]
    struct MockTopicRepository {
        result: Result<TopicResult, TopicRepositoryError>,
    }

    impl MockTopicRepository {
        fn success(result: TopicResult) -> Self {
            Self { result: Ok(result) }
        }

        fn topic_already_exists() -> Self {
            Self {
                result: Err(TopicRepositoryError::TopicAlreadyExists),
            }
        }

        fn db_error(msg: &str) -> Self {
            Self {
                result: Err(TopicRepositoryError::DatabaseError(msg.to_string())),
            }
        }
    }

    #[async_trait]
    impl TopicRepository for MockTopicRepository {
        async fn create_topic(
            &self,
            _data: CreateTopicData,
        ) -> Result<TopicResult, TopicRepositoryError> {
            self.result.clone()
        }

        async fn restore_topic(
            &self,
            _topic_id: Uuid,
        ) -> Result<TopicResult, TopicRepositoryError> {
            unimplemented!()
        }

        async fn soft_delete_topic(&self, _topic_id: Uuid) -> Result<(), TopicRepositoryError> {
            unimplemented!()
        }
    }

    // ──────────────────────────────────────────────────────────
    // Helpers
    // ──────────────────────────────────────────────────────────

    fn valid_command() -> CreateTopicCommand {
        CreateTopicCommand::new(
            UserId::from(Uuid::new_v4()),
            "Rust".to_string(),
            Some("Rust-related topic".to_string()),
        )
        .unwrap()
    }

    fn sample_topic_result(owner: UserId) -> TopicResult {
        TopicResult {
            id: Uuid::new_v4(),
            owner,
            title: "Rust".to_string(),
            description: "Rust-related topic".to_string(),
        }
    }

    // ──────────────────────────────────────────────────────────
    // Tests
    // ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn create_topic_success() {
        // Arrange
        let owner = UserId::from(Uuid::new_v4());
        let command = CreateTopicCommand::new(
            owner.clone(),
            "Rust".to_string(),
            Some("Rust-related topic".to_string()),
        )
        .unwrap();

        let expected = sample_topic_result(owner.clone());

        let repo = MockTopicRepository::success(expected.clone());
        let service = CreateTopicService::new(repo);

        // Act
        let result = service.execute(command).await;

        // Assert
        assert!(result.is_ok(), "Expected success, got {:?}", result);
        let topic = result.unwrap();

        assert_eq!(topic.id, expected.id);
        assert_eq!(topic.owner, expected.owner);
        assert_eq!(topic.title, "Rust");
        assert_eq!(topic.description, "Rust-related topic");
    }

    #[tokio::test]
    async fn create_topic_topic_already_exists() {
        // Arrange
        let command = valid_command();

        let repo = MockTopicRepository::topic_already_exists();
        let service = CreateTopicService::new(repo);

        // Act
        let result = service.execute(command).await;

        // Assert
        assert!(
            matches!(result, Err(CreateTopicError::TopicAlreadyExists)),
            "Expected TopicAlreadyExists, got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn create_topic_repository_error_is_mapped() {
        // Arrange
        let command = valid_command();

        let repo = MockTopicRepository::db_error("connection lost");
        let service = CreateTopicService::new(repo);

        // Act
        let result = service.execute(command).await;

        // Assert
        match result {
            Err(CreateTopicError::RepositoryError(msg)) => {
                assert!(msg.contains("connection lost"));
            }
            other => panic!("Expected RepositoryError, got {:?}", other),
        }
    }

    #[test]
    fn service_is_cloneable() {
        let repo = MockTopicRepository::topic_already_exists();
        let service = CreateTopicService::new(repo);

        let _clone = service.clone();

        // If it compiles and runs, Clone works
        assert!(true);
    }
}
