use async_trait::async_trait;

use crate::modules::project::application::ports::incoming::use_cases::{
    CreateProjectError, CreateProjectUseCase,
};
use crate::modules::project::application::ports::outgoing::project_repository::{
    CreateProjectData, ProjectRepository, ProjectRepositoryError, ProjectResult,
};

//
// ──────────────────────────────────────────────────────────
// Service
// ──────────────────────────────────────────────────────────
//

pub struct CreateProjectService<R>
where
    R: ProjectRepository,
{
    project_repository: R,
}

impl<R> CreateProjectService<R>
where
    R: ProjectRepository,
{
    pub fn new(project_repository: R) -> Self {
        Self { project_repository }
    }
}

#[async_trait]
impl<R> CreateProjectUseCase for CreateProjectService<R>
where
    R: ProjectRepository + Send + Sync,
{
    async fn execute(&self, data: CreateProjectData) -> Result<ProjectResult, CreateProjectError> {
        self.project_repository
            .create_project(data)
            .await
            .map_err(|e| match e {
                ProjectRepositoryError::SlugAlreadyExists => CreateProjectError::SlugAlreadyExists,
                ProjectRepositoryError::DatabaseError(msg) => {
                    CreateProjectError::RepositoryError(msg)
                }
                ProjectRepositoryError::SerializationError(msg) => {
                    CreateProjectError::RepositoryError(msg)
                }
                // Defensive: should never happen on create
                ProjectRepositoryError::NotFound => CreateProjectError::RepositoryError(
                    "unexpected not found while creating project".to_string(),
                ),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use uuid::Uuid;

    use crate::auth::application::domain::entities::UserId;
    use crate::modules::project::application::ports::outgoing::project_repository::{
        CreateProjectData, ProjectRepository, ProjectRepositoryError, ProjectResult,
    };
    use chrono::Utc;

    #[derive(Clone)]
    struct MockProjectRepo {
        result: Result<ProjectResult, ProjectRepositoryError>,
    }

    #[async_trait]
    impl ProjectRepository for MockProjectRepo {
        async fn create_project(
            &self,
            _data: CreateProjectData,
        ) -> Result<ProjectResult, ProjectRepositoryError> {
            self.result.clone()
        }

        async fn patch_project(
            &self,
            _owner: UserId,
            _project_id: Uuid,
            _data: crate::modules::project::application::ports::outgoing::project_repository::PatchProjectData,
        ) -> Result<ProjectResult, ProjectRepositoryError> {
            unimplemented!("not needed for create_project tests")
        }
    }

    fn sample_create_data() -> CreateProjectData {
        CreateProjectData {
            owner: UserId::from(Uuid::new_v4()),
            title: "Title".to_string(),
            slug: "slug".to_string(),
            description: "Desc".to_string(),
            tech_stack: vec!["Rust".to_string()],
            screenshots: vec!["img.png".to_string()],
            repo_url: None,
            live_demo_url: None,
        }
    }

    fn sample_project_result() -> ProjectResult {
        ProjectResult {
            id: Uuid::new_v4(),
            owner: UserId::from(Uuid::new_v4()),
            title: "Title".to_string(),
            slug: "slug".to_string(),
            description: "Desc".to_string(),
            tech_stack: vec!["Rust".to_string()],
            screenshots: vec!["img.png".to_string()],
            repo_url: None,
            live_demo_url: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    // =====================================================
    // Success
    // =====================================================

    #[tokio::test]
    async fn test_execute_success() {
        let repo = MockProjectRepo {
            result: Ok(sample_project_result()),
        };
        let service = CreateProjectService::new(repo);

        let res = service.execute(sample_create_data()).await;

        assert!(res.is_ok());
    }

    // =====================================================
    // Error mapping
    // =====================================================

    #[tokio::test]
    async fn test_execute_maps_slug_already_exists() {
        let repo = MockProjectRepo {
            result: Err(ProjectRepositoryError::SlugAlreadyExists),
        };
        let service = CreateProjectService::new(repo);

        let res = service.execute(sample_create_data()).await;

        assert!(matches!(
            res.unwrap_err(),
            CreateProjectError::SlugAlreadyExists
        ));
    }

    #[tokio::test]
    async fn test_execute_maps_database_error() {
        let repo = MockProjectRepo {
            result: Err(ProjectRepositoryError::DatabaseError("db down".to_string())),
        };
        let service = CreateProjectService::new(repo);

        let res = service.execute(sample_create_data()).await;

        assert!(matches!(
            res.unwrap_err(),
            CreateProjectError::RepositoryError(msg) if msg == "db down"
        ));
    }

    #[tokio::test]
    async fn test_execute_maps_serialization_error() {
        let repo = MockProjectRepo {
            result: Err(ProjectRepositoryError::SerializationError(
                "bad json".to_string(),
            )),
        };
        let service = CreateProjectService::new(repo);

        let res = service.execute(sample_create_data()).await;

        assert!(matches!(
            res.unwrap_err(),
            CreateProjectError::RepositoryError(msg) if msg == "bad json"
        ));
    }

    #[tokio::test]
    async fn test_execute_maps_unexpected_not_found() {
        let repo = MockProjectRepo {
            result: Err(ProjectRepositoryError::NotFound),
        };
        let service = CreateProjectService::new(repo);

        let res = service.execute(sample_create_data()).await;

        assert!(matches!(
            res.unwrap_err(),
            CreateProjectError::RepositoryError(msg)
                if msg == "unexpected not found while creating project"
        ));
    }
}
