use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::modules::project::application::ports::incoming::use_cases::{
    GetProjectTopicsError, GetProjectTopicsUseCase,
};
use crate::modules::project::application::ports::outgoing::project_query::ProjectQuery;
use crate::project::application::ports::outgoing::project_query::ProjectTopicItem;

pub struct GetProjectTopicsService<Q>
where
    Q: ProjectQuery,
{
    query: Q,
}

impl<Q> GetProjectTopicsService<Q>
where
    Q: ProjectQuery,
{
    pub fn new(query: Q) -> Self {
        Self { query }
    }
}

#[async_trait]
impl<Q> GetProjectTopicsUseCase for GetProjectTopicsService<Q>
where
    Q: ProjectQuery + Send + Sync,
{
    async fn execute(
        &self,
        _owner: UserId,
        project_id: Uuid,
    ) -> Result<Vec<ProjectTopicItem>, GetProjectTopicsError> {
        // Owner is intentionally unused:
        // - get_project_topics() is based on project_id
        // - project existence/visibility is enforced by the query layer where appropriate
        self.query
            .get_project_topics(project_id)
            .await
            .map_err(GetProjectTopicsError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::modules::project::application::ports::outgoing::project_query::{
        PageRequest, PageResult, ProjectCardView, ProjectListFilter, ProjectQueryError,
        ProjectSort, ProjectView,
    };

    #[derive(Clone)]
    struct MockProjectQuery {
        result: Result<Vec<ProjectTopicItem>, ProjectQueryError>,
    }

    impl MockProjectQuery {
        fn success(items: Vec<ProjectTopicItem>) -> Self {
            Self { result: Ok(items) }
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
            unimplemented!("not used")
        }

        async fn get_by_slug(&self, _slug: &str) -> Result<ProjectView, ProjectQueryError> {
            unimplemented!("not used")
        }

        async fn list(
            &self,
            _owner: UserId,
            _filter: ProjectListFilter,
            _sort: ProjectSort,
            _page: PageRequest,
        ) -> Result<PageResult<ProjectCardView>, ProjectQueryError> {
            unimplemented!("not used")
        }

        async fn get_project_topics(
            &self,
            _project_id: Uuid,
        ) -> Result<Vec<ProjectTopicItem>, ProjectQueryError> {
            self.result.clone()
        }

        async fn slug_exists(&self, _slug: &str) -> Result<bool, ProjectQueryError> {
            unimplemented!("not used")
        }
    }

    #[actix_web::test]
    async fn execute_returns_topics_from_query() {
        let topic_id = Uuid::new_v4();

        let query = MockProjectQuery::success(vec![ProjectTopicItem {
            id: topic_id,
            title: "Rust".to_string(),
            description: "Systems".to_string(),
        }]);

        let service = GetProjectTopicsService::new(query);

        let owner = UserId::from(Uuid::new_v4());
        let project_id = Uuid::new_v4();

        let result = service.execute(owner, project_id).await;

        assert!(result.is_ok());
        let topics = result.unwrap();
        assert_eq!(topics.len(), 1);
        assert_eq!(topics[0].id, topic_id);
        assert_eq!(topics[0].title, "Rust");
    }

    #[actix_web::test]
    async fn execute_maps_not_found_to_project_not_found() {
        let query = MockProjectQuery::error(ProjectQueryError::NotFound);
        let service = GetProjectTopicsService::new(query);

        let owner = UserId::from(Uuid::new_v4());
        let project_id = Uuid::new_v4();

        let result = service.execute(owner, project_id).await;

        assert!(matches!(
            result,
            Err(GetProjectTopicsError::ProjectNotFound)
        ));
    }

    #[actix_web::test]
    async fn execute_maps_database_error_to_query_failed() {
        let query = MockProjectQuery::error(ProjectQueryError::DatabaseError("db down".into()));
        let service = GetProjectTopicsService::new(query);

        let owner = UserId::from(Uuid::new_v4());
        let project_id = Uuid::new_v4();

        let result = service.execute(owner, project_id).await;

        assert!(matches!(
            result,
            Err(GetProjectTopicsError::QueryFailed(msg)) if msg == "db down"
        ));
    }

    #[actix_web::test]
    async fn execute_maps_serialization_error_to_query_failed() {
        let query =
            MockProjectQuery::error(ProjectQueryError::SerializationError("bad row".into()));
        let service = GetProjectTopicsService::new(query);

        let owner = UserId::from(Uuid::new_v4());
        let project_id = Uuid::new_v4();

        let result = service.execute(owner, project_id).await;

        assert!(matches!(
            result,
            Err(GetProjectTopicsError::QueryFailed(msg)) if msg == "bad row"
        ));
    }
}
