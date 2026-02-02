use async_trait::async_trait;
use uuid::Uuid;

use crate::cv::application::ports::outgoing::{
    CVListFilter, CVPageRequest, CVPageResult, CVQuery, CVQueryError, CVSort,
};
use crate::cv::domain::entities::CVInfo;

// ============================================================================
// Errors
// ============================================================================

#[derive(Debug, Clone)]
pub enum FetchCVError {
    QueryFailed(String),
}

impl From<CVQueryError> for FetchCVError {
    fn from(e: CVQueryError) -> Self {
        match e {
            CVQueryError::DatabaseError(msg) => FetchCVError::QueryFailed(msg),
            CVQueryError::QueryFailed(msg) => FetchCVError::QueryFailed(msg),
        }
    }
}

// ============================================================================
// Service Implementation
// ============================================================================

pub struct FetchCVService<Q>
where
    Q: CVQuery,
{
    query: Q,
}

impl<Q> FetchCVService<Q>
where
    Q: CVQuery,
{
    pub fn new(query: Q) -> Self {
        Self { query }
    }
}

#[async_trait]
pub trait IFetchCVUseCase: Send + Sync {
    async fn execute(
        &self,
        user_id: Uuid,
        filter: CVListFilter,
        sort: CVSort,
        page: CVPageRequest,
    ) -> Result<CVPageResult<CVInfo>, FetchCVError>;
}

#[async_trait]
impl<Q> IFetchCVUseCase for FetchCVService<Q>
where
    Q: CVQuery + Send + Sync,
{
    async fn execute(
        &self,
        user_id: Uuid,
        filter: CVListFilter,
        sort: CVSort,
        page: CVPageRequest,
    ) -> Result<CVPageResult<CVInfo>, FetchCVError> {
        self.query
            .list(user_id, filter, sort, page)
            .await
            .map_err(FetchCVError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use uuid::Uuid;

    // ========================================================================
    // Mock CVQuery
    // ========================================================================

    #[derive(Clone)]
    struct MockCVQuery {
        result: Result<CVPageResult<CVInfo>, CVQueryError>,
    }

    impl MockCVQuery {
        fn success(result: CVPageResult<CVInfo>) -> Self {
            Self { result: Ok(result) }
        }

        fn error(err: CVQueryError) -> Self {
            Self { result: Err(err) }
        }
    }

    #[async_trait]
    impl CVQuery for MockCVQuery {
        async fn list(
            &self,
            _user_id: Uuid,
            _filter: CVListFilter,
            _sort: CVSort,
            _page: CVPageRequest,
        ) -> Result<CVPageResult<CVInfo>, CVQueryError> {
            self.result.clone()
        }

        async fn fetch_cv_by_id(&self, _cv_id: Uuid) -> Result<Option<CVInfo>, CVQueryError> {
            unimplemented!("not used in FetchCVService tests")
        }
    }

    // ========================================================================
    // Helpers
    // ========================================================================

    fn sample_page_result() -> CVPageResult<CVInfo> {
        CVPageResult {
            items: vec![CVInfo {
                id: Uuid::new_v4(),
                user_id: Uuid::new_v4(),
                display_name: "John Doe".to_string(),
                role: "Backend Engineer".to_string(),
                bio: "Test bio".to_string(),
                photo_url: "".to_string(),
                core_skills: vec![],
                educations: vec![],
                experiences: vec![],
                highlighted_projects: vec![],
                contact_info: vec![],
            }],
            page: 1,
            per_page: 10,
            total: 1,
        }
    }

    fn empty_page_result() -> CVPageResult<CVInfo> {
        CVPageResult {
            items: vec![],
            page: 1,
            per_page: 10,
            total: 0,
        }
    }

    // ========================================================================
    // Tests
    // ========================================================================

    #[tokio::test]
    async fn execute_success() {
        let user_id = Uuid::new_v4();

        let query = MockCVQuery::success(sample_page_result());
        let service = FetchCVService::new(query);

        let result = service
            .execute(
                user_id,
                CVListFilter::default(),
                CVSort::default(),
                CVPageRequest::default(),
            )
            .await;

        assert!(result.is_ok());
        let page = result.unwrap();
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.total, 1);
    }

    #[tokio::test]
    async fn execute_empty_result_is_ok() {
        let user_id = Uuid::new_v4();

        let query = MockCVQuery::success(empty_page_result());
        let service = FetchCVService::new(query);

        let result = service
            .execute(
                user_id,
                CVListFilter::default(),
                CVSort::default(),
                CVPageRequest::default(),
            )
            .await;

        assert!(result.is_ok());
        let page = result.unwrap();
        assert!(page.items.is_empty());
        assert_eq!(page.total, 0);
    }

    #[tokio::test]
    async fn execute_maps_database_error() {
        let user_id = Uuid::new_v4();

        let query = MockCVQuery::error(CVQueryError::DatabaseError("db down".to_string()));
        let service = FetchCVService::new(query);

        let result = service
            .execute(
                user_id,
                CVListFilter::default(),
                CVSort::default(),
                CVPageRequest::default(),
            )
            .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FetchCVError::QueryFailed(_)));
    }

    #[tokio::test]
    async fn execute_maps_query_failed_error() {
        let user_id = Uuid::new_v4();

        let query = MockCVQuery::error(CVQueryError::QueryFailed("bad query".to_string()));
        let service = FetchCVService::new(query);

        let result = service
            .execute(
                user_id,
                CVListFilter::default(),
                CVSort::default(),
                CVPageRequest::default(),
            )
            .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FetchCVError::QueryFailed(_)));
    }
}
