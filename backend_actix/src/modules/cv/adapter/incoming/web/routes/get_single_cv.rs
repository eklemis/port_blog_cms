use actix_web::{get, web, HttpResponse, Responder};
use uuid::Uuid;

use crate::{
    auth::adapter::incoming::web::extractors::auth::VerifiedUser,
    cv::application::use_cases::fetch_cv_by_id::FetchCVByIdError, AppState,
};

#[get("/api/cvs/{cv_id}")]
pub async fn get_cv_by_id_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> impl Responder {
    let cv_id = path.into_inner();

    match data
        .fetch_cv_by_id_use_case
        .execute(user.user_id, cv_id)
        .await
    {
        Ok(cv) => HttpResponse::Ok().json(cv),

        Err(FetchCVByIdError::CVNotFound) => HttpResponse::NotFound().finish(),

        Err(FetchCVByIdError::RepositoryError(err)) => {
            HttpResponse::InternalServerError().body(err)
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
            application::use_cases::fetch_cv_by_id::IFetchCVByIdUseCase,
            domain::{CVInfo, Education},
        },
        tests::support::{
            app_state_builder::TestAppStateBuilder,
            auth_helper::test_helpers::create_test_jwt_service,
        },
    };
    use actix_web::{test, web, App};
    use tokio::sync::Mutex;
    use uuid::Uuid;

    /* --------------------------------------------------
     * Mock Fetch CV By ID Use Case
     * -------------------------------------------------- */

    #[derive(Clone)]
    struct MockFetchCVByIdUseCase {
        should_fail: Arc<Mutex<Option<FetchCVByIdError>>>,
        cv: Arc<Mutex<Option<CVInfo>>>,
    }

    impl MockFetchCVByIdUseCase {
        fn new() -> Self {
            Self {
                should_fail: Arc::new(Mutex::new(None)),
                cv: Arc::new(Mutex::new(None)),
            }
        }

        async fn set_success(&self, cv: CVInfo) {
            *self.cv.lock().await = Some(cv);
            *self.should_fail.lock().await = None;
        }

        async fn set_error(&self, err: FetchCVByIdError) {
            *self.should_fail.lock().await = Some(err);
        }
    }

    #[async_trait::async_trait]
    impl IFetchCVByIdUseCase for MockFetchCVByIdUseCase {
        async fn execute(&self, _user_id: Uuid, _cv_id: Uuid) -> Result<CVInfo, FetchCVByIdError> {
            if let Some(err) = self.should_fail.lock().await.clone() {
                return Err(err);
            }

            if let Some(cv) = self.cv.lock().await.clone() {
                return Ok(cv);
            }

            panic!("MockFetchCVByIdUseCase called without configured result");
        }
    }

    /* --------------------------------------------------
     * Tests
     * -------------------------------------------------- */

    #[actix_web::test]
    async fn test_get_cv_by_id_handler_success() {
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let expected_cv = CVInfo {
            id: cv_id,
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

        let fetch_uc = MockFetchCVByIdUseCase::new();
        fetch_uc.set_success(expected_cv.clone()).await;

        let app_state = TestAppStateBuilder::default()
            .with_fetch_cv_by_id(fetch_uc)
            .build();

        let jwt_service = create_test_jwt_service();
        let token = jwt_service.generate_access_token(user_id, true).unwrap();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service.clone());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(get_cv_by_id_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 200);

        let body: CVInfo = test::read_body_json(resp).await;
        assert_eq!(body.id, cv_id);
        assert_eq!(body.bio, expected_cv.bio);
    }

    #[actix_web::test]
    async fn test_get_cv_by_id_handler_cv_not_found() {
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let fetch_uc = MockFetchCVByIdUseCase::new();
        fetch_uc.set_error(FetchCVByIdError::CVNotFound).await;

        let app_state = TestAppStateBuilder::default()
            .with_fetch_cv_by_id(fetch_uc)
            .build();

        let jwt_service = create_test_jwt_service();
        let token = jwt_service.generate_access_token(user_id, true).unwrap();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(get_cv_by_id_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 404);
    }

    #[actix_web::test]
    async fn test_get_cv_by_id_handler_repository_error() {
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let fetch_uc = MockFetchCVByIdUseCase::new();
        fetch_uc
            .set_error(FetchCVByIdError::RepositoryError(
                "DB read failed".to_string(),
            ))
            .await;

        let app_state = TestAppStateBuilder::default()
            .with_fetch_cv_by_id(fetch_uc)
            .build();

        let jwt_service = create_test_jwt_service();
        let token = jwt_service.generate_access_token(user_id, true).unwrap();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service.clone());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(get_cv_by_id_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 500);

        let body = test::read_body(resp).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("DB read failed"));
    }

    #[actix_web::test]
    async fn test_get_cv_by_id_handler_missing_authorization() {
        let cv_id = Uuid::new_v4();

        let fetch_uc = MockFetchCVByIdUseCase::new();

        let app_state = TestAppStateBuilder::default()
            .with_fetch_cv_by_id(fetch_uc)
            .build();

        let jwt_service = create_test_jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(get_cv_by_id_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/cvs/{}", cv_id))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 401);
    }

    #[actix_web::test]
    async fn test_get_cv_by_id_handler_invalid_token() {
        let cv_id = Uuid::new_v4();

        let fetch_uc = MockFetchCVByIdUseCase::new();

        let app_state = TestAppStateBuilder::default()
            .with_fetch_cv_by_id(fetch_uc)
            .build();

        let jwt_service = create_test_jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(get_cv_by_id_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", "Bearer invalid_token_here"))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 401);
    }

    #[actix_web::test]
    async fn test_get_cv_by_id_handler_unverified_user() {
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let fetch_uc = MockFetchCVByIdUseCase::new();

        let app_state = TestAppStateBuilder::default()
            .with_fetch_cv_by_id(fetch_uc)
            .build();

        let jwt_service = create_test_jwt_service();
        let token = jwt_service.generate_access_token(user_id, false).unwrap();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service.clone());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(get_cv_by_id_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 403);
    }
}
