use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::modules::project::application::ports::incoming::use_cases::{
    AddProjectTopicError, AddProjectTopicUseCase,
};
use crate::modules::project::application::ports::outgoing::project_topic_repository::ProjectTopicRepository;

pub struct AddProjectTopicService<R>
where
    R: ProjectTopicRepository,
{
    repo: R,
}

impl<R> AddProjectTopicService<R>
where
    R: ProjectTopicRepository,
{
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R> AddProjectTopicUseCase for AddProjectTopicService<R>
where
    R: ProjectTopicRepository + Send + Sync,
{
    async fn execute(
        &self,
        owner: UserId,
        project_id: Uuid,
        topic_id: Uuid,
    ) -> Result<(), AddProjectTopicError> {
        self.repo
            .add_project_topic(owner, project_id, topic_id)
            .await
            .map_err(AddProjectTopicError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use uuid::Uuid;

    use crate::modules::project::application::ports::outgoing::project_topic_repository::{
        ProjectTopicRepository, ProjectTopicRepositoryError,
    };

    #[derive(Clone)]
    struct MockProjectTopicRepository {
        result: Result<(), ProjectTopicRepositoryError>,
    }

    impl MockProjectTopicRepository {
        fn ok() -> Self {
            Self { result: Ok(()) }
        }

        fn err(err: ProjectTopicRepositoryError) -> Self {
            Self { result: Err(err) }
        }
    }

    #[async_trait]
    impl ProjectTopicRepository for MockProjectTopicRepository {
        async fn add_project_topic(
            &self,
            _owner: UserId,
            _project_id: Uuid,
            _topic_id: Uuid,
        ) -> Result<(), ProjectTopicRepositoryError> {
            self.result.clone()
        }

        async fn remove_project_topic(
            &self,
            _owner: UserId,
            _project_id: Uuid,
            _topic_id: Uuid,
        ) -> Result<(), ProjectTopicRepositoryError> {
            unimplemented!("not used in AddProjectTopicService tests")
        }

        async fn clear_project_topics(
            &self,
            _owner: UserId,
            _project_id: Uuid,
        ) -> Result<(), ProjectTopicRepositoryError> {
            unimplemented!("not used in AddProjectTopicService tests")
        }

        async fn set_project_topics(
            &self,
            _owner: UserId,
            _project_id: Uuid,
            _topic_ids: Vec<Uuid>,
        ) -> Result<(), ProjectTopicRepositoryError> {
            unimplemented!("not used in AddProjectTopicService tests")
        }
    }

    #[tokio::test]
    async fn execute_success() {
        let owner = UserId::from(Uuid::new_v4());
        let project_id = Uuid::new_v4();
        let topic_id = Uuid::new_v4();

        let repo = MockProjectTopicRepository::ok();
        let service = AddProjectTopicService::new(repo);

        let result = service.execute(owner, project_id, topic_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn execute_maps_project_not_found() {
        let owner = UserId::from(Uuid::new_v4());

        let repo = MockProjectTopicRepository::err(ProjectTopicRepositoryError::ProjectNotFound);
        let service = AddProjectTopicService::new(repo);

        let result = service.execute(owner, Uuid::new_v4(), Uuid::new_v4()).await;

        assert!(matches!(result, Err(AddProjectTopicError::ProjectNotFound)));
    }

    #[tokio::test]
    async fn execute_maps_topic_not_found() {
        let owner = UserId::from(Uuid::new_v4());

        let repo = MockProjectTopicRepository::err(ProjectTopicRepositoryError::TopicNotFound);
        let service = AddProjectTopicService::new(repo);

        let result = service.execute(owner, Uuid::new_v4(), Uuid::new_v4()).await;

        assert!(matches!(result, Err(AddProjectTopicError::TopicNotFound)));
    }

    #[tokio::test]
    async fn execute_maps_database_error() {
        let owner = UserId::from(Uuid::new_v4());

        let repo = MockProjectTopicRepository::err(ProjectTopicRepositoryError::DatabaseError(
            "db down".to_string(),
        ));
        let service = AddProjectTopicService::new(repo);

        let result = service.execute(owner, Uuid::new_v4(), Uuid::new_v4()).await;

        assert!(matches!(
            result,
            Err(AddProjectTopicError::RepositoryError(_))
        ));
    }
}
