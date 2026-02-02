use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::modules::project::application::ports::incoming::use_cases::{
    GetSingleProjectError, GetSingleProjectUseCase,
};
use crate::modules::project::application::ports::outgoing::project_query::{
    ProjectQuery, ProjectQueryError, ProjectView,
};

pub struct GetSingleProjectService<Q>
where
    Q: ProjectQuery,
{
    query: Q,
}

impl<Q> GetSingleProjectService<Q>
where
    Q: ProjectQuery,
{
    pub fn new(query: Q) -> Self {
        Self { query }
    }
}

#[async_trait]
impl<Q> GetSingleProjectUseCase for GetSingleProjectService<Q>
where
    Q: ProjectQuery + Send + Sync,
{
    async fn execute(
        &self,
        owner: UserId,
        project_id: Uuid,
    ) -> Result<ProjectView, GetSingleProjectError> {
        self.query
            .get_by_id(owner, project_id)
            .await
            .map_err(|e| match e {
                ProjectQueryError::NotFound => GetSingleProjectError::NotFound,
                ProjectQueryError::DatabaseError(msg) => {
                    GetSingleProjectError::RepositoryError(msg)
                }
                ProjectQueryError::SerializationError(msg) => {
                    GetSingleProjectError::RepositoryError(msg)
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use uuid::Uuid;

    use crate::auth::application::domain::entities::UserId;
    use crate::modules::project::application::ports::outgoing::project_query::{
        PageRequest, PageResult, ProjectCardView, ProjectListFilter, ProjectQuery,
        ProjectQueryError, ProjectSort, ProjectView,
    };
    use crate::project::application::ports::outgoing::project_query::ProjectTopicItem;

    /* --------------------------------------------------
     * Mock ProjectQuery
     * -------------------------------------------------- */

    #[derive(Clone)]
    struct MockProjectQuery {
        result: Result<ProjectView, ProjectQueryError>,
    }

    impl MockProjectQuery {
        fn success(view: ProjectView) -> Self {
            Self { result: Ok(view) }
        }

        fn error(err: ProjectQueryError) -> Self {
            Self { result: Err(err) }
        }
    }

    #[async_trait]
    impl ProjectQuery for MockProjectQuery {
        async fn get_by_id(
            &self,
            _owner: UserId,
            _project_id: Uuid,
        ) -> Result<ProjectView, ProjectQueryError> {
            self.result.clone()
        }

        async fn get_by_slug(&self, _slug: &str) -> Result<ProjectView, ProjectQueryError> {
            unimplemented!("not used in GetSingleProjectService tests")
        }

        async fn list(
            &self,
            _owner: UserId,
            _filter: ProjectListFilter,
            _sort: ProjectSort,
            _page: PageRequest,
        ) -> Result<PageResult<ProjectCardView>, ProjectQueryError> {
            unimplemented!("not used in GetSingleProjectService tests")
        }

        async fn get_project_topics(
            &self,
            project_id: Uuid,
        ) -> Result<Vec<ProjectTopicItem>, ProjectQueryError> {
            unimplemented!("not used in GetSingleProjectService tests")
        }

        async fn slug_exists(&self, _slug: &str) -> Result<bool, ProjectQueryError> {
            unimplemented!("not used in GetSingleProjectService tests")
        }
    }

    /* --------------------------------------------------
     * Helpers
     * -------------------------------------------------- */

    fn sample_project_view(owner: UserId, project_id: Uuid) -> ProjectView {
        ProjectView {
            id: project_id,
            owner,
            title: "Test Project".to_string(),
            slug: "test-project".to_string(),
            description: "desc".to_string(),
            tech_stack: vec!["Rust".to_string()],
            screenshots: vec!["img.png".to_string()],
            repo_url: None,
            live_demo_url: None,
            topics: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    /* --------------------------------------------------
     * Tests
     * -------------------------------------------------- */

    #[tokio::test]
    async fn execute_success() {
        let owner = UserId::from(Uuid::new_v4());
        let project_id = Uuid::new_v4();

        let view = sample_project_view(owner.clone(), project_id);

        let query = MockProjectQuery::success(view.clone());
        let service = GetSingleProjectService::new(query);

        let result = service.execute(owner, project_id).await;

        assert!(result.is_ok());
        let got = result.unwrap();
        assert_eq!(got.id, view.id);
        assert_eq!(got.slug, view.slug);
    }

    #[tokio::test]
    async fn execute_maps_not_found() {
        let owner = UserId::from(Uuid::new_v4());
        let project_id = Uuid::new_v4();

        let query = MockProjectQuery::error(ProjectQueryError::NotFound);
        let service = GetSingleProjectService::new(query);

        let result = service.execute(owner, project_id).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            GetSingleProjectError::NotFound
        ));
    }

    #[tokio::test]
    async fn execute_maps_database_error_to_repository_error() {
        let owner = UserId::from(Uuid::new_v4());
        let project_id = Uuid::new_v4();

        let query =
            MockProjectQuery::error(ProjectQueryError::DatabaseError("db down".to_string()));
        let service = GetSingleProjectService::new(query);

        let result = service.execute(owner, project_id).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            GetSingleProjectError::RepositoryError(_)
        ));
    }

    #[tokio::test]
    async fn execute_maps_serialization_error_to_repository_error() {
        let owner = UserId::from(Uuid::new_v4());
        let project_id = Uuid::new_v4();

        let query = MockProjectQuery::error(ProjectQueryError::SerializationError(
            "bad json".to_string(),
        ));
        let service = GetSingleProjectService::new(query);

        let result = service.execute(owner, project_id).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            GetSingleProjectError::RepositoryError(_)
        ));
    }
}
