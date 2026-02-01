use async_trait::async_trait;

use crate::auth::application::domain::entities::UserId;
use crate::modules::project::application::ports::incoming::use_cases::{
    GetPublicSingleProjectError, GetPublicSingleProjectUseCase,
};
use crate::modules::project::application::ports::outgoing::project_query::{
    ProjectQuery, ProjectQueryError, ProjectView,
};

pub struct GetPublicSingleProjectService<Q>
where
    Q: ProjectQuery,
{
    query: Q,
}

impl<Q> GetPublicSingleProjectService<Q>
where
    Q: ProjectQuery,
{
    pub fn new(query: Q) -> Self {
        Self { query }
    }
}

#[async_trait]
impl<Q> GetPublicSingleProjectUseCase for GetPublicSingleProjectService<Q>
where
    Q: ProjectQuery + Send + Sync,
{
    async fn execute(
        &self,
        owner: UserId,
        slug: &str,
    ) -> Result<ProjectView, GetPublicSingleProjectError> {
        let project = self.query.get_by_slug(slug).await.map_err(|e| match e {
            ProjectQueryError::NotFound => GetPublicSingleProjectError::NotFound,
            ProjectQueryError::DatabaseError(msg) => {
                GetPublicSingleProjectError::RepositoryError(msg)
            }
            ProjectQueryError::SerializationError(msg) => {
                GetPublicSingleProjectError::RepositoryError(msg)
            }
        })?;

        // Public route is username-scoped, so we must not leak that a slug exists for another user.
        if project.owner != owner {
            return Err(GetPublicSingleProjectError::NotFound);
        }

        Ok(project)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use uuid::Uuid;

    use crate::modules::project::application::ports::outgoing::project_query::{
        PageRequest, PageResult, ProjectCardView, ProjectListFilter, ProjectQuery,
        ProjectQueryError, ProjectSort, ProjectView,
    };

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
            unimplemented!("not used in GetPublicSingleProjectService tests")
        }

        async fn get_by_slug(&self, _slug: &str) -> Result<ProjectView, ProjectQueryError> {
            self.result.clone()
        }

        async fn list(
            &self,
            _owner: UserId,
            _filter: ProjectListFilter,
            _sort: ProjectSort,
            _page: PageRequest,
        ) -> Result<PageResult<ProjectCardView>, ProjectQueryError> {
            unimplemented!("not used in GetPublicSingleProjectService tests")
        }

        async fn get_project_topics(
            &self,
            _project_id: Uuid,
        ) -> Result<Vec<Uuid>, ProjectQueryError> {
            unimplemented!("not used in GetPublicSingleProjectService tests")
        }

        async fn slug_exists(&self, _slug: &str) -> Result<bool, ProjectQueryError> {
            unimplemented!("not used in GetPublicSingleProjectService tests")
        }
    }

    /* --------------------------------------------------
     * Helpers
     * -------------------------------------------------- */

    fn sample_project_view(owner: UserId) -> ProjectView {
        ProjectView {
            id: Uuid::new_v4(),
            owner,
            title: "Public Project".to_string(),
            slug: "public-project".to_string(),
            description: "desc".to_string(),
            tech_stack: vec!["Rust".to_string()],
            screenshots: vec!["img.png".to_string()],
            repo_url: None,
            live_demo_url: None,
            topic_ids: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    /* --------------------------------------------------
     * Tests
     * -------------------------------------------------- */

    #[tokio::test]
    async fn execute_success_when_owner_matches() {
        let owner = UserId::from(Uuid::new_v4());
        let view = sample_project_view(owner.clone());

        let query = MockProjectQuery::success(view.clone());
        let service = GetPublicSingleProjectService::new(query);

        let result = service.execute(owner, "public-project").await;

        assert!(result.is_ok());
        let got = result.unwrap();
        assert_eq!(got.slug, "public-project");
    }

    #[tokio::test]
    async fn execute_returns_not_found_when_slug_exists_but_owner_mismatch() {
        let requested_owner = UserId::from(Uuid::new_v4());
        let actual_owner = UserId::from(Uuid::new_v4());
        let view = sample_project_view(actual_owner);

        let query = MockProjectQuery::success(view);
        let service = GetPublicSingleProjectService::new(query);

        let result = service.execute(requested_owner, "public-project").await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            GetPublicSingleProjectError::NotFound
        ));
    }

    #[tokio::test]
    async fn execute_maps_query_not_found() {
        let owner = UserId::from(Uuid::new_v4());

        let query = MockProjectQuery::error(ProjectQueryError::NotFound);
        let service = GetPublicSingleProjectService::new(query);

        let result = service.execute(owner, "missing").await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            GetPublicSingleProjectError::NotFound
        ));
    }

    #[tokio::test]
    async fn execute_maps_database_error_to_repository_error() {
        let owner = UserId::from(Uuid::new_v4());

        let query =
            MockProjectQuery::error(ProjectQueryError::DatabaseError("db down".to_string()));
        let service = GetPublicSingleProjectService::new(query);

        let result = service.execute(owner, "public-project").await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            GetPublicSingleProjectError::RepositoryError(msg) if msg == "db down"
        ));
    }

    #[tokio::test]
    async fn execute_maps_serialization_error_to_repository_error() {
        let owner = UserId::from(Uuid::new_v4());

        let query = MockProjectQuery::error(ProjectQueryError::SerializationError(
            "bad json".to_string(),
        ));
        let service = GetPublicSingleProjectService::new(query);

        let result = service.execute(owner, "public-project").await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            GetPublicSingleProjectError::RepositoryError(msg) if msg == "bad json"
        ));
    }
}
