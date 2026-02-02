use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::modules::project::application::ports::incoming::use_cases::{
    ClearProjectTopicsError, ClearProjectTopicsUseCase,
};
use crate::modules::project::application::ports::outgoing::project_topic_repository::ProjectTopicRepository;

pub struct ClearProjectTopicsService<R>
where
    R: ProjectTopicRepository,
{
    repo: R,
}

impl<R> ClearProjectTopicsService<R>
where
    R: ProjectTopicRepository,
{
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R> ClearProjectTopicsUseCase for ClearProjectTopicsService<R>
where
    R: ProjectTopicRepository + Send + Sync,
{
    async fn execute(
        &self,
        owner: UserId,
        project_id: Uuid,
    ) -> Result<(), ClearProjectTopicsError> {
        self.repo
            .clear_project_topics(owner, project_id)
            .await
            .map_err(ClearProjectTopicsError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::modules::project::application::ports::outgoing::project_topic_repository::ProjectTopicRepositoryError;

    #[derive(Clone)]
    struct MockProjectTopicRepository {
        clear_result: Result<(), ProjectTopicRepositoryError>,
    }

    #[async_trait]
    impl ProjectTopicRepository for MockProjectTopicRepository {
        async fn add_project_topic(
            &self,
            _owner: UserId,
            _project_id: Uuid,
            _topic_id: Uuid,
        ) -> Result<(), ProjectTopicRepositoryError> {
            unimplemented!("not used")
        }

        async fn remove_project_topic(
            &self,
            _owner: UserId,
            _project_id: Uuid,
            _topic_id: Uuid,
        ) -> Result<(), ProjectTopicRepositoryError> {
            unimplemented!("not used")
        }

        async fn clear_project_topics(
            &self,
            _owner: UserId,
            _project_id: Uuid,
        ) -> Result<(), ProjectTopicRepositoryError> {
            self.clear_result.clone()
        }

        async fn set_project_topics(
            &self,
            _owner: UserId,
            _project_id: Uuid,
            _topic_ids: Vec<Uuid>,
        ) -> Result<(), ProjectTopicRepositoryError> {
            unimplemented!("not used")
        }
    }

    #[actix_web::test]
    async fn execute_returns_ok_when_repository_returns_ok() {
        let repo = MockProjectTopicRepository {
            clear_result: Ok(()),
        };
        let service = ClearProjectTopicsService::new(repo);

        let owner = UserId::from(Uuid::new_v4());
        let project_id = Uuid::new_v4();

        let result = service.execute(owner, project_id).await;

        assert!(result.is_ok());
    }

    #[actix_web::test]
    async fn execute_maps_project_not_found() {
        let repo = MockProjectTopicRepository {
            clear_result: Err(ProjectTopicRepositoryError::ProjectNotFound),
        };
        let service = ClearProjectTopicsService::new(repo);

        let owner = UserId::from(Uuid::new_v4());
        let project_id = Uuid::new_v4();

        let result = service.execute(owner, project_id).await;

        assert!(matches!(
            result,
            Err(ClearProjectTopicsError::ProjectNotFound)
        ));
    }

    #[actix_web::test]
    async fn execute_maps_database_error_into_repository_error_string() {
        let repo = MockProjectTopicRepository {
            clear_result: Err(ProjectTopicRepositoryError::DatabaseError(
                "db is down".to_string(),
            )),
        };
        let service = ClearProjectTopicsService::new(repo);

        let owner = UserId::from(Uuid::new_v4());
        let project_id = Uuid::new_v4();

        let result = service.execute(owner, project_id).await;

        assert!(matches!(
            result,
            Err(ClearProjectTopicsError::RepositoryError(msg)) if msg == "db is down"
        ));
    }
}
