use actix_web::{get, web, Responder};

use crate::{
    auth::adapter::incoming::web::extractors::auth::VerifiedUser,
    cv::{application::use_cases::fetch_user_cvs::FetchCVError, domain::CVInfo},
    shared::api::ApiResponse,
    AppState,
};
use tracing::error;

#[get("/api/cvs")]
pub async fn get_cvs_handler(user: VerifiedUser, data: web::Data<AppState>) -> impl Responder {
    match data.fetch_cv_use_case.execute(user.user_id).await {
        Ok(cvs) => ApiResponse::success(cvs),

        Err(FetchCVError::NoCVs) => ApiResponse::success(Vec::<CVInfo>::new()),

        Err(FetchCVError::RepositoryError(err)) => {
            error!("Repository error fetching CVs: {}", err);
            ApiResponse::internal_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::{
        auth::application::ports::outgoing::token_provider::TokenProvider,
        cv::{
            application::use_cases::fetch_user_cvs::IFetchCVUseCase,
            domain::{CVInfo, Education},
        },
        tests::support::{
            app_state_builder::TestAppStateBuilder,
            auth_helper::test_helpers::create_test_jwt_service,
        },
    };
    use actix_web::{test, web, App};
    use async_trait::async_trait;
    use tokio::sync::Mutex;
    use uuid::Uuid;

    /* --------------------------------------------------
     * Mock Fetch CV Use Case
     * -------------------------------------------------- */

    #[derive(Clone)]
    struct MockFetchCVUseCase {
        should_fail: Arc<Mutex<Option<FetchCVError>>>,
        cvs: Arc<Mutex<Option<Vec<CVInfo>>>>,
    }

    impl MockFetchCVUseCase {
        fn new() -> Self {
            Self {
                should_fail: Arc::new(Mutex::new(None)),
                cvs: Arc::new(Mutex::new(None)),
            }
        }

        async fn set_error(&self, error: FetchCVError) {
            *self.should_fail.lock().await = Some(error);
        }

        async fn set_success(&self, cvs: Vec<CVInfo>) {
            *self.cvs.lock().await = Some(cvs);
            *self.should_fail.lock().await = None;
        }
    }

    #[async_trait]
    impl IFetchCVUseCase for MockFetchCVUseCase {
        async fn execute(&self, _user_id: Uuid) -> Result<Vec<CVInfo>, FetchCVError> {
            if let Some(err) = self.should_fail.lock().await.clone() {
                return Err(err);
            }

            if let Some(cvs) = self.cvs.lock().await.clone() {
                return Ok(cvs);
            }

            panic!("MockFetchCVUseCase called without configured result");
        }
    }

    /* --------------------------------------------------
     * Tests
     * -------------------------------------------------- */

    use serde_json::Value;

    #[actix_web::test]
    async fn test_get_cv_handler_success() {
        let fetch_uc = MockFetchCVUseCase::new();

        let user_id = Uuid::new_v4();
        let expected_cv = CVInfo {
            id: Uuid::new_v4(),
            user_id,
            display_name: "Berto Fang".to_string(),
            bio: "Software Engineer with 5 years of experience".to_string(),
            role: "Software Engineer".to_string(),
            photo_url: "https://example.com/photo.jpg".to_string(),
            core_skills: vec![],
            educations: vec![Education {
                degree: "Bachelor of Computer Science".to_string(),
                institution: "MIT".to_string(),
                graduation_year: 2018,
            }],
            experiences: vec![],
            highlighted_projects: vec![],
            contact_info: vec![],
        };

        fetch_uc.set_success(vec![expected_cv.clone()]).await;

        let app_state = TestAppStateBuilder::default()
            .with_fetch_cv(fetch_uc)
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
        assert_eq!(resp.status(), 200);

        let body: Value = test::read_body_json(resp).await;
        let cvs: Vec<CVInfo> = serde_json::from_value(body["data"].clone()).unwrap();

        assert_eq!(cvs.len(), 1);
        assert_eq!(cvs[0].bio, expected_cv.bio);
    }

    #[actix_web::test]
    async fn test_get_cv_handler_no_cvs() {
        let fetch_uc = MockFetchCVUseCase::new();
        fetch_uc.set_error(FetchCVError::NoCVs).await;

        let app_state = TestAppStateBuilder::default()
            .with_fetch_cv(fetch_uc)
            .build();

        let jwt_service = create_test_jwt_service();
        let user_id = Uuid::new_v4();
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
        assert_eq!(resp.status(), 200);

        let body: Value = test::read_body_json(resp).await;
        let cvs: Vec<CVInfo> = serde_json::from_value(body["data"].clone()).unwrap();

        assert!(cvs.is_empty());
    }

    #[actix_web::test]
    async fn test_get_cv_handler_repository_error() {
        let fetch_uc = MockFetchCVUseCase::new();
        fetch_uc
            .set_error(FetchCVError::RepositoryError(
                "Database connection failed".to_string(),
            ))
            .await;

        let app_state = TestAppStateBuilder::default()
            .with_fetch_cv(fetch_uc)
            .build();

        let jwt_service = create_test_jwt_service();
        let user_id = Uuid::new_v4();
        let token = jwt_service.generate_access_token(user_id, true).unwrap();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(jwt_service))
                .service(get_cvs_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/cvs")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 500);
    }

    #[actix_web::test]
    async fn test_get_cv_handler_missing_auth_header() {
        let fetch_uc = MockFetchCVUseCase::new();

        let app_state = TestAppStateBuilder::default()
            .with_fetch_cv(fetch_uc)
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
        assert_eq!(resp.status(), 401);
    }

    #[actix_web::test]
    async fn test_get_cv_handler_invalid_token() {
        let fetch_uc = MockFetchCVUseCase::new();

        let app_state = TestAppStateBuilder::default()
            .with_fetch_cv(fetch_uc)
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
            .insert_header(("Authorization", "Bearer invalid_token_here"))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 401);
    }

    #[actix_web::test]
    async fn test_get_cv_handler_unverified_user() {
        let fetch_uc = MockFetchCVUseCase::new();

        let app_state = TestAppStateBuilder::default()
            .with_fetch_cv(fetch_uc)
            .build();

        let jwt_service = create_test_jwt_service();
        let user_id = Uuid::new_v4();
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
        assert_eq!(resp.status(), 403);
    }
}
