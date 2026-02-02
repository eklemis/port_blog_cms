use actix_web::{get, web, Responder};
use serde::Deserialize;
use tracing::error;

use crate::auth::adapter::incoming::web::extractors::auth::VerifiedUser;
use crate::cv::application::ports::outgoing::{CVListFilter, CVPageRequest, CVSort};
use crate::cv::application::use_cases::fetch_user_cvs::FetchCVError;
use crate::shared::api::ApiResponse;
use crate::AppState;

//
// ──────────────────────────────────────────────────────────
// Query DTO
// ──────────────────────────────────────────────────────────
//

#[derive(Debug, Deserialize)]
pub struct GetCVsQuery {
    pub search: Option<String>,

    #[serde(default)]
    pub sort: CVSort,

    #[serde(default)]
    pub page: u32,

    #[serde(default)]
    pub per_page: u32,
}

impl From<GetCVsQuery> for (CVListFilter, CVPageRequest, CVSort) {
    fn from(q: GetCVsQuery) -> Self {
        let filter = CVListFilter { search: q.search };

        let page = CVPageRequest {
            page: if q.page == 0 { 1 } else { q.page },
            per_page: if q.per_page == 0 { 10 } else { q.per_page },
        };

        (filter, page, q.sort)
    }
}

//
// ──────────────────────────────────────────────────────────
// Handler
// ──────────────────────────────────────────────────────────
//

#[get("/api/cvs")]
pub async fn get_cvs_handler(
    user: VerifiedUser,
    query: web::Query<GetCVsQuery>,
    data: web::Data<AppState>,
) -> impl Responder {
    let (filter, page, sort) = query.into_inner().into();

    match data
        .fetch_cv_use_case
        .execute(user.user_id, filter, sort, page)
        .await
    {
        Ok(result) => ApiResponse::success(result),

        Err(FetchCVError::QueryFailed(msg)) => {
            error!("Failed to list CVs: {}", msg);
            ApiResponse::internal_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{http::StatusCode, test, web, App};
    use async_trait::async_trait;
    use serde_json::Value as JsonValue;
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::auth::application::ports::outgoing::token_provider::TokenProvider;
    use crate::cv::application::ports::outgoing::{
        CVListFilter, CVPageRequest, CVPageResult, CVSort,
    };
    use crate::cv::application::use_cases::fetch_user_cvs::{FetchCVError, IFetchCVUseCase};
    use crate::cv::domain::entities::CVInfo;
    use crate::tests::support::{
        app_state_builder::TestAppStateBuilder, auth_helper::test_helpers::create_test_jwt_service,
    };

    /* --------------------------------------------------
     * Mock Fetch CV Use Case
     * -------------------------------------------------- */

    #[derive(Clone)]
    struct MockFetchCVUseCase {
        result: Result<CVPageResult<CVInfo>, FetchCVError>,
    }

    impl MockFetchCVUseCase {
        fn success(data: CVPageResult<CVInfo>) -> Self {
            Self { result: Ok(data) }
        }

        fn error(err: FetchCVError) -> Self {
            Self { result: Err(err) }
        }
    }

    #[async_trait]
    impl IFetchCVUseCase for MockFetchCVUseCase {
        async fn execute(
            &self,
            _user_id: Uuid,
            _filter: CVListFilter,
            _sort: CVSort,
            _page: CVPageRequest,
        ) -> Result<CVPageResult<CVInfo>, FetchCVError> {
            self.result.clone()
        }
    }

    /* --------------------------------------------------
     * Helpers
     * -------------------------------------------------- */

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

    /* --------------------------------------------------
     * Tests
     * -------------------------------------------------- */

    #[actix_web::test]
    async fn test_get_cvs_success() {
        let user_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_fetch_cv(MockFetchCVUseCase::success(sample_page_result()))
            .build();

        let jwt_service = create_test_jwt_service();
        let token = jwt_service.generate_access_token(user_id, true).unwrap();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(get_cvs_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/cvs")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: JsonValue = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert!(body["error"].is_null());

        let page: CVPageResult<CVInfo> = serde_json::from_value(body["data"].clone()).unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.items.len(), 1);
    }

    #[actix_web::test]
    async fn test_get_cvs_empty_result() {
        let user_id = Uuid::new_v4();

        let empty = CVPageResult {
            items: vec![],
            page: 1,
            per_page: 10,
            total: 0,
        };

        let app_state = TestAppStateBuilder::default()
            .with_fetch_cv(MockFetchCVUseCase::success(empty))
            .build();

        let jwt_service = create_test_jwt_service();
        let token = jwt_service.generate_access_token(user_id, true).unwrap();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(get_cvs_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/cvs")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: JsonValue = test::read_body_json(resp).await;
        let page: CVPageResult<CVInfo> = serde_json::from_value(body["data"].clone()).unwrap();
        assert!(page.items.is_empty());
        assert_eq!(page.total, 0);
    }

    #[actix_web::test]
    async fn test_get_cvs_internal_error_on_query_failed() {
        let user_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_fetch_cv(MockFetchCVUseCase::error(FetchCVError::QueryFailed(
                "db down".to_string(),
            )))
            .build();

        let jwt_service = create_test_jwt_service();
        let token = jwt_service.generate_access_token(user_id, true).unwrap();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(get_cvs_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/cvs")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[actix_web::test]
    async fn test_get_cvs_unverified_user_forbidden() {
        let user_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_fetch_cv(MockFetchCVUseCase::success(sample_page_result()))
            .build();

        let jwt_service = create_test_jwt_service();
        let token = jwt_service.generate_access_token(user_id, false).unwrap();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(get_cvs_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/cvs")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn test_get_cvs_missing_auth_header() {
        let app_state = TestAppStateBuilder::default()
            .with_fetch_cv(MockFetchCVUseCase::success(sample_page_result()))
            .build();

        let jwt_service = create_test_jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(get_cvs_handler),
        )
        .await;

        let req = test::TestRequest::get().uri("/api/cvs").to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn test_get_cvs_invalid_token() {
        let app_state = TestAppStateBuilder::default()
            .with_fetch_cv(MockFetchCVUseCase::success(sample_page_result()))
            .build();

        let jwt_service = create_test_jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(get_cvs_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/cvs")
            .insert_header(("Authorization", "Bearer invalid_token"))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}
