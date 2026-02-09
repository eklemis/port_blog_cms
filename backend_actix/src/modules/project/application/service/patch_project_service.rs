use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::modules::project::application::ports::incoming::use_cases::{
    PatchProjectError, PatchProjectUseCase,
};
use crate::modules::project::application::ports::outgoing::project_repository::{
    PatchProjectData, ProjectRepository, ProjectRepositoryError, ProjectResult,
};

//
// ──────────────────────────────────────────────────────────
// Service
// ──────────────────────────────────────────────────────────
//

pub struct PatchProjectService<R>
where
    R: ProjectRepository,
{
    project_repository: R,
}

impl<R> PatchProjectService<R>
where
    R: ProjectRepository,
{
    pub fn new(project_repository: R) -> Self {
        Self { project_repository }
    }
}

#[async_trait]
impl<R> PatchProjectUseCase for PatchProjectService<R>
where
    R: ProjectRepository + Send + Sync,
{
    async fn execute(
        &self,
        owner: UserId,
        project_id: Uuid,
        data: PatchProjectData,
    ) -> Result<ProjectResult, PatchProjectError> {
        self.project_repository
            .patch_project(owner, project_id, data)
            .await
            .map_err(|e| match e {
                ProjectRepositoryError::NotFound => PatchProjectError::NotFound,

                ProjectRepositoryError::DatabaseError(msg) => {
                    PatchProjectError::RepositoryError(msg)
                }

                ProjectRepositoryError::SerializationError(msg) => {
                    PatchProjectError::RepositoryError(msg)
                }

                // Defensive: slug is immutable and shouldn't be patched
                ProjectRepositoryError::SlugAlreadyExists => PatchProjectError::RepositoryError(
                    "unexpected slug conflict while patching project".to_string(),
                ),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use uuid::Uuid;

    use crate::auth::application::domain::entities::UserId;
    use crate::modules::project::application::ports::outgoing::project_repository::{
        CreateProjectData, PatchField, PatchProjectData, ProjectRepository, ProjectRepositoryError,
        ProjectResult,
    };

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
            unimplemented!("not needed for patch_project tests")
        }

        async fn patch_project(
            &self,
            _owner: UserId,
            _project_id: Uuid,
            _data: PatchProjectData,
        ) -> Result<ProjectResult, ProjectRepositoryError> {
            self.result.clone()
        }
    }

    fn sample_owner() -> UserId {
        UserId::from(Uuid::new_v4())
    }

    fn sample_project_id() -> Uuid {
        Uuid::new_v4()
    }

    fn sample_patch_data() -> PatchProjectData {
        PatchProjectData {
            title: PatchField::Value("New Title".to_string()),
            ..Default::default()
        }
    }

    fn sample_project_result(owner: UserId, project_id: Uuid) -> ProjectResult {
        ProjectResult {
            id: project_id,
            owner,
            title: "New Title".to_string(),
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
        let owner = sample_owner();
        let project_id = sample_project_id();

        let repo = MockProjectRepo {
            result: Ok(sample_project_result(owner.clone(), project_id)),
        };
        let service = PatchProjectService::new(repo);

        let res = service
            .execute(owner, project_id, sample_patch_data())
            .await;

        assert!(res.is_ok());
    }

    // =====================================================
    // Error mapping
    // =====================================================

    #[tokio::test]
    async fn test_execute_maps_not_found() {
        let owner = sample_owner();
        let project_id = sample_project_id();

        let repo = MockProjectRepo {
            result: Err(ProjectRepositoryError::NotFound),
        };
        let service = PatchProjectService::new(repo);

        let res = service
            .execute(owner, project_id, sample_patch_data())
            .await;

        assert!(matches!(res.unwrap_err(), PatchProjectError::NotFound));
    }

    #[tokio::test]
    async fn test_execute_maps_database_error() {
        let owner = sample_owner();
        let project_id = sample_project_id();

        let repo = MockProjectRepo {
            result: Err(ProjectRepositoryError::DatabaseError("db down".to_string())),
        };
        let service = PatchProjectService::new(repo);

        let res = service
            .execute(owner, project_id, sample_patch_data())
            .await;

        assert!(matches!(
            res.unwrap_err(),
            PatchProjectError::RepositoryError(msg) if msg == "db down"
        ));
    }

    #[tokio::test]
    async fn test_execute_maps_serialization_error() {
        let owner = sample_owner();
        let project_id = sample_project_id();

        let repo = MockProjectRepo {
            result: Err(ProjectRepositoryError::SerializationError(
                "bad json".to_string(),
            )),
        };
        let service = PatchProjectService::new(repo);

        let res = service
            .execute(owner, project_id, sample_patch_data())
            .await;

        assert!(matches!(
            res.unwrap_err(),
            PatchProjectError::RepositoryError(msg) if msg == "bad json"
        ));
    }

    #[tokio::test]
    async fn test_execute_maps_unexpected_slug_already_exists_defensively() {
        let owner = sample_owner();
        let project_id = sample_project_id();

        let repo = MockProjectRepo {
            result: Err(ProjectRepositoryError::SlugAlreadyExists),
        };
        let service = PatchProjectService::new(repo);

        let res = service
            .execute(owner, project_id, sample_patch_data())
            .await;

        assert!(matches!(
            res.unwrap_err(),
            PatchProjectError::RepositoryError(msg)
                if msg == "unexpected slug conflict while patching project"
        ));
    }
}
