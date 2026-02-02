use async_trait::async_trait;
use uuid::Uuid;

use crate::cv::application::ports::outgoing::{CVQuery, CVQueryError};
use crate::cv::application::use_cases::get_public_single_cv::{
    GetPublicSingleCvError, GetPublicSingleCvUseCase,
};
use crate::cv::domain::entities::CVInfo;

//
// ──────────────────────────────────────────────────────────
// Service
// ──────────────────────────────────────────────────────────
//

pub struct GetPublicSingleCvService<Q>
where
    Q: CVQuery,
{
    query: Q,
}

impl<Q> GetPublicSingleCvService<Q>
where
    Q: CVQuery,
{
    pub fn new(query: Q) -> Self {
        Self { query }
    }
}

#[async_trait]
impl<Q> GetPublicSingleCvUseCase for GetPublicSingleCvService<Q>
where
    Q: CVQuery + Send + Sync,
{
    async fn execute(&self, owner_id: Uuid, cv_id: Uuid) -> Result<CVInfo, GetPublicSingleCvError> {
        let cv = self
            .query
            .fetch_cv_by_id(cv_id)
            .await
            .map_err(|e| match e {
                CVQueryError::DatabaseError(msg) => GetPublicSingleCvError::RepositoryError(msg),
                CVQueryError::QueryFailed(msg) => GetPublicSingleCvError::RepositoryError(msg),
            })?;

        match cv {
            None => Err(GetPublicSingleCvError::NotFound),
            Some(cv) if cv.user_id != owner_id => Err(GetPublicSingleCvError::NotFound),
            Some(cv) => Ok(cv),
        }
    }
}

//
// ──────────────────────────────────────────────────────────
// Unit tests (service only)
// ──────────────────────────────────────────────────────────
//

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cv::application::ports::outgoing::{CVListFilter, CVSort};
    use crate::cv::application::ports::outgoing::{CVPageRequest, CVPageResult};
    use async_trait::async_trait;

    #[derive(Clone)]
    struct MockCVQuery {
        result: Result<Option<CVInfo>, CVQueryError>,
    }

    impl MockCVQuery {
        fn found(cv: CVInfo) -> Self {
            Self {
                result: Ok(Some(cv)),
            }
        }

        fn not_found() -> Self {
            Self { result: Ok(None) }
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
            unimplemented!("not used in GetPublicSingleCvService tests")
        }

        async fn fetch_cv_by_id(&self, _cv_id: Uuid) -> Result<Option<CVInfo>, CVQueryError> {
            self.result.clone()
        }
    }

    fn sample_cv(user_id: Uuid) -> CVInfo {
        CVInfo {
            id: Uuid::new_v4(),
            user_id,
            display_name: "Gandalf Wood".to_string(),
            role: "Engineer".to_string(),
            bio: "Test CV".to_string(),
            photo_url: "".to_string(),
            core_skills: vec![],
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
            contact_info: vec![],
        }
    }

    // =====================================================
    // Success
    // =====================================================

    #[tokio::test]
    async fn execute_success_when_cv_belongs_to_owner() {
        let owner_id = Uuid::new_v4();
        let cv = sample_cv(owner_id);

        let query = MockCVQuery::found(cv.clone());
        let service = GetPublicSingleCvService::new(query);

        let result = service.execute(owner_id, cv.id).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, cv.id);
    }

    // =====================================================
    // Not found
    // =====================================================

    #[tokio::test]
    async fn execute_not_found_when_cv_missing() {
        let owner_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let query = MockCVQuery::not_found();
        let service = GetPublicSingleCvService::new(query);

        let result = service.execute(owner_id, cv_id).await;

        assert!(matches!(result, Err(GetPublicSingleCvError::NotFound)));
    }

    #[tokio::test]
    async fn execute_not_found_when_cv_belongs_to_other_user() {
        let owner_id = Uuid::new_v4();
        let other_user_id = Uuid::new_v4();
        let cv = sample_cv(other_user_id);

        let query = MockCVQuery::found(cv);
        let service = GetPublicSingleCvService::new(query);

        let result = service.execute(owner_id, Uuid::new_v4()).await;

        assert!(matches!(result, Err(GetPublicSingleCvError::NotFound)));
    }

    // =====================================================
    // Error mapping
    // =====================================================

    #[tokio::test]
    async fn execute_maps_database_error_to_repository_error() {
        let owner_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let query = MockCVQuery::error(CVQueryError::DatabaseError("db down".to_string()));
        let service = GetPublicSingleCvService::new(query);

        let result = service.execute(owner_id, cv_id).await;

        assert!(matches!(
            result.unwrap_err(),
            GetPublicSingleCvError::RepositoryError(msg) if msg == "db down"
        ));
    }

    #[tokio::test]
    async fn execute_maps_query_failed_to_repository_error() {
        let owner_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let query = MockCVQuery::error(CVQueryError::QueryFailed("query failed".to_string()));
        let service = GetPublicSingleCvService::new(query);

        let result = service.execute(owner_id, cv_id).await;

        assert!(matches!(
            result.unwrap_err(),
            GetPublicSingleCvError::RepositoryError(msg) if msg == "query failed"
        ));
    }
}
