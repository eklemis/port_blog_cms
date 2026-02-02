use async_trait::async_trait;

use crate::auth::application::domain::entities::UserId;
use crate::modules::project::application::ports::incoming::use_cases::{
    GetProjectsError, GetProjectsUseCase,
};
use crate::modules::project::application::ports::outgoing::project_query::{
    PageRequest, PageResult, ProjectCardView, ProjectListFilter, ProjectQuery, ProjectSort,
};

// ============================================================================
// Service Implementation
// ============================================================================

pub struct GetProjectsService<Q>
where
    Q: ProjectQuery,
{
    query: Q,
}

impl<Q> GetProjectsService<Q>
where
    Q: ProjectQuery,
{
    pub fn new(query: Q) -> Self {
        Self { query }
    }
}

#[async_trait]
impl<Q> GetProjectsUseCase for GetProjectsService<Q>
where
    Q: ProjectQuery + Send + Sync,
{
    async fn execute(
        &self,
        owner: UserId,
        filter: ProjectListFilter,
        sort: ProjectSort,
        page: PageRequest,
    ) -> Result<PageResult<ProjectCardView>, GetProjectsError> {
        self.query
            .list(owner, filter, sort, page)
            .await
            .map_err(GetProjectsError::from)
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
        ProjectQueryError, ProjectSort,
    };
    use crate::project::application::ports::outgoing::project_query::{
        ProjectTopicItem, ProjectView,
    };

    /* --------------------------------------------------
     * Mock ProjectQuery
     * -------------------------------------------------- */

    #[derive(Clone)]
    struct MockProjectQuery {
        result: Result<PageResult<ProjectCardView>, ProjectQueryError>,
    }

    impl MockProjectQuery {
        fn success(result: PageResult<ProjectCardView>) -> Self {
            Self { result: Ok(result) }
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
            unimplemented!("not used in GetProjectsService tests")
        }

        async fn get_by_slug(&self, _slug: &str) -> Result<ProjectView, ProjectQueryError> {
            unimplemented!("not used in GetProjectsService tests")
        }

        async fn list(
            &self,
            _owner: UserId,
            _filter: ProjectListFilter,
            _sort: ProjectSort,
            _page: PageRequest,
        ) -> Result<PageResult<ProjectCardView>, ProjectQueryError> {
            self.result.clone()
        }

        async fn get_project_topics(
            &self,
            _project_id: Uuid,
        ) -> Result<Vec<ProjectTopicItem>, ProjectQueryError> {
            unimplemented!("not used in GetProjectsService tests")
        }

        async fn slug_exists(&self, _slug: &str) -> Result<bool, ProjectQueryError> {
            unimplemented!("not used in GetProjectsService tests")
        }
    }

    /* --------------------------------------------------
     * Helpers
     * -------------------------------------------------- */

    fn sample_page_result() -> PageResult<ProjectCardView> {
        PageResult {
            items: vec![ProjectCardView {
                id: Uuid::new_v4(),
                title: "Test Project".to_string(),
                slug: "test-project".to_string(),
                tech_stack: vec!["Rust".to_string()],
                repo_url: None,
                live_demo_url: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            }],
            page: 1,
            per_page: 10,
            total: 1,
        }
    }

    /* --------------------------------------------------
     * Tests
     * -------------------------------------------------- */

    #[tokio::test]
    async fn execute_success() {
        let owner = UserId::from(Uuid::new_v4());

        let query = MockProjectQuery::success(sample_page_result());
        let service = GetProjectsService::new(query);

        let result = service
            .execute(
                owner,
                ProjectListFilter::default(),
                ProjectSort::default(),
                PageRequest::default(),
            )
            .await;

        assert!(result.is_ok());

        let page = result.unwrap();
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.total, 1);
    }

    #[tokio::test]
    async fn execute_maps_database_error() {
        let owner = UserId::from(Uuid::new_v4());

        let query =
            MockProjectQuery::error(ProjectQueryError::DatabaseError("db down".to_string()));
        let service = GetProjectsService::new(query);

        let result = service
            .execute(
                owner,
                ProjectListFilter::default(),
                ProjectSort::default(),
                PageRequest::default(),
            )
            .await;

        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, GetProjectsError::QueryFailed(_)));
    }

    #[tokio::test]
    async fn execute_maps_serialization_error() {
        let owner = UserId::from(Uuid::new_v4());

        let query = MockProjectQuery::error(ProjectQueryError::SerializationError(
            "bad json".to_string(),
        ));
        let service = GetProjectsService::new(query);

        let result = service
            .execute(
                owner,
                ProjectListFilter::default(),
                ProjectSort::default(),
                PageRequest::default(),
            )
            .await;

        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, GetProjectsError::QueryFailed(_)));
    }
}
