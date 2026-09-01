use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::modules::project::application::ports::incoming::use_cases::{
    SoftDeleteProjectError, SoftDeleteProjectUseCase,
};
use crate::modules::project::application::ports::outgoing::project_archiver::ProjectArchiver;

/// Implements the corresponding use-case contract.
pub struct SoftDeleteProjectService<A>
where
    A: ProjectArchiver,
{
    archiver: A,
}

impl<A> SoftDeleteProjectService<A>
where
    A: ProjectArchiver,
{
    /// Builds it from the ports it depends on.
    pub fn new(archiver: A) -> Self {
        Self { archiver }
    }
}

#[async_trait]
impl<A> SoftDeleteProjectUseCase for SoftDeleteProjectService<A>
where
    A: ProjectArchiver + Send + Sync,
{
    async fn execute(&self, owner: UserId, project_id: Uuid) -> Result<(), SoftDeleteProjectError> {
        self.archiver
            .soft_delete(owner, project_id)
            .await
            .map_err(SoftDeleteProjectError::from)
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
            self.result.clone()
        }

        async fn hard_delete(
            &self,
            _owner: UserId,
            _project_id: Uuid,
        ) -> Result<(), ProjectArchiverError> {
            unimplemented!("not used")
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
        let service = SoftDeleteProjectService::new(MockProjectArchiver::success());

        let result = service
            .execute(UserId::from(Uuid::new_v4()), Uuid::new_v4())
            .await;

        assert!(result.is_ok());
    }

    #[actix_web::test]
    async fn execute_project_not_found() {
        let service = SoftDeleteProjectService::new(MockProjectArchiver::error(
            ProjectArchiverError::NotFound,
        ));

        let result = service
            .execute(UserId::from(Uuid::new_v4()), Uuid::new_v4())
            .await;

        assert!(matches!(
            result,
            Err(SoftDeleteProjectError::ProjectNotFound)
        ));
    }

    #[actix_web::test]
    async fn execute_repository_error() {
        let service = SoftDeleteProjectService::new(MockProjectArchiver::error(
            ProjectArchiverError::DatabaseError("db down".into()),
        ));

        let result = service
            .execute(UserId::from(Uuid::new_v4()), Uuid::new_v4())
            .await;

        assert!(matches!(
            result,
            Err(SoftDeleteProjectError::RepositoryError(msg)) if msg == "db down"
        ));
    }
}
