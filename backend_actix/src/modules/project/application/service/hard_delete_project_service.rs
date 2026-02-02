use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::modules::project::application::ports::incoming::use_cases::{
    HardDeleteProjectError, HardDeleteProjectUseCase,
};
use crate::modules::project::application::ports::outgoing::project_archiver::ProjectArchiver;

pub struct HardDeleteProjectService<A>
where
    A: ProjectArchiver,
{
    archiver: A,
}

impl<A> HardDeleteProjectService<A>
where
    A: ProjectArchiver,
{
    pub fn new(archiver: A) -> Self {
        Self { archiver }
    }
}

#[async_trait]
impl<A> HardDeleteProjectUseCase for HardDeleteProjectService<A>
where
    A: ProjectArchiver + Send + Sync,
{
    async fn execute(&self, owner: UserId, project_id: Uuid) -> Result<(), HardDeleteProjectError> {
        self.archiver
            .hard_delete(owner, project_id)
            .await
            .map_err(HardDeleteProjectError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::modules::project::application::ports::outgoing::project_archiver::{
        ProjectArchiver, ProjectArchiverError,
    };

    #[derive(Clone)]
    struct MockProjectArchiver {
        result: Result<(), ProjectArchiverError>,
    }

    impl MockProjectArchiver {
        fn success() -> Self {
            Self { result: Ok(()) }
        }

        fn error(err: ProjectArchiverError) -> Self {
            Self { result: Err(err) }
        }
    }

    #[async_trait]
    impl ProjectArchiver for MockProjectArchiver {
        async fn soft_delete(
            &self,
            _owner: UserId,
            _project_id: Uuid,
        ) -> Result<(), ProjectArchiverError> {
            unimplemented!("not used")
        }

        async fn hard_delete(
            &self,
            _owner: UserId,
            _project_id: Uuid,
        ) -> Result<(), ProjectArchiverError> {
            self.result.clone()
        }

        async fn restore(
            &self,
            _owner: UserId,
            _project_id: Uuid,
        ) -> Result<(), ProjectArchiverError> {
            unimplemented!("not used")
        }
    }

    #[actix_web::test]
    async fn execute_success() {
        let archiver = MockProjectArchiver::success();
        let service = HardDeleteProjectService::new(archiver);

        let owner = UserId::from(Uuid::new_v4());
        let project_id = Uuid::new_v4();

        let result = service.execute(owner, project_id).await;
        assert!(result.is_ok());
    }

    #[actix_web::test]
    async fn execute_project_not_found() {
        let archiver = MockProjectArchiver::error(ProjectArchiverError::NotFound);
        let service = HardDeleteProjectService::new(archiver);

        let owner = UserId::from(Uuid::new_v4());
        let project_id = Uuid::new_v4();

        let result = service.execute(owner, project_id).await;

        assert!(matches!(
            result,
            Err(HardDeleteProjectError::ProjectNotFound)
        ));
    }

    #[actix_web::test]
    async fn execute_repository_error() {
        let archiver =
            MockProjectArchiver::error(ProjectArchiverError::DatabaseError("db down".into()));
        let service = HardDeleteProjectService::new(archiver);

        let owner = UserId::from(Uuid::new_v4());
        let project_id = Uuid::new_v4();

        let result = service.execute(owner, project_id).await;

        assert!(matches!(
            result,
            Err(HardDeleteProjectError::RepositoryError(msg)) if msg == "db down"
        ));
    }
}
