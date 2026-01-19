use actix_web::{get, web, HttpResponse, Responder};

use crate::{
    auth::adapter::incoming::web::extractors::auth::VerifiedUser,
    cv::{application::use_cases::fetch_user_cvs::FetchCVError, domain::CVInfo},
    AppState,
};

#[get("/api/cvs")]
pub async fn get_cvs_handler(
    user: VerifiedUser,
    data: web::Data<AppState>, // The state from .app_data(...)
) -> impl Responder {
    match data.fetch_cv_use_case.execute(user.user_id).await {
        Ok(cvs) => HttpResponse::Ok().json(cvs),

        Err(FetchCVError::NoCVs) => HttpResponse::Ok().json(Vec::<CVInfo>::new()),

        Err(FetchCVError::RepositoryError(err)) => HttpResponse::InternalServerError().body(err),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::{
        cv::{application::use_cases::fetch_user_cvs::IFetchCVUseCase, domain::Education},
        tests::support::{
            app_state_builder::TestAppStateBuilder,
            auth_helper::test_helpers::create_test_jwt_service,
        },
    };
    use actix_web::{test, App};
    use async_trait::async_trait;
    use tokio::sync::Mutex;
    use uuid::Uuid;

    #[derive(Clone)]
    struct MockFetchCVUseCase {
        should_fail: Arc<Mutex<Option<FetchCVError>>>,
        cv: Arc<Mutex<Option<CVInfo>>>,
    }

    impl MockFetchCVUseCase {
        fn new() -> Self {
            Self {
                should_fail: Arc::new(Mutex::new(None)),
                cv: Arc::new(Mutex::new(None)),
            }
        }

        async fn set_error(&self, error: FetchCVError) {
            *self.should_fail.lock().await = Some(error);
        }

        async fn set_success(&self, cv: CVInfo) {
            *self.cv.lock().await = Some(cv);
            *self.should_fail.lock().await = None;
        }
    }

    #[async_trait]
    impl IFetchCVUseCase for MockFetchCVUseCase {
        async fn execute(&self, _user_id: Uuid) -> Result<Vec<CVInfo>, FetchCVError> {
            let error = self.should_fail.lock().await;
            if let Some(err) = error.as_ref() {
                return Err(err.clone());
            }

            let cv = self.cv.lock().await;
            if let Some(c) = cv.as_ref() {
                return Ok(vec![c.clone()]);
            }

            // Default success case - return a vector with one CV
            Ok(vec![CVInfo {
                id: Uuid::new_v4(),
                user_id: Uuid::new_v4(),
                bio: "Default bio".to_string(),
                role: "Data Engineer".to_string(),
                photo_url: "https://example.com/photo.jpg".to_string(),
                core_skills: vec![],
                educations: vec![],
                experiences: vec![],
                highlighted_projects: vec![],
            }])
        }
    }

    #[actix_web::test]
    async fn test_get_cv_handler_success() {
        let fetch_uc = MockFetchCVUseCase::new();

        let user_id = Uuid::new_v4();
        let expected_cv = CVInfo {
            id: Uuid::new_v4(),
            user_id,
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
        };

        fetch_uc.set_success(expected_cv.clone()).await;

        let app_state = TestAppStateBuilder::default()
            .with_fetch_cv(fetch_uc)
            .build();

        let jwt_service = create_test_jwt_service();

        // Generate a valid token for a verified user
        let token = jwt_service
            .generate_access_token(user_id, true)
            .expect("Failed to generate token");

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
        assert_eq!(resp.status(), 200);

        let body: Vec<CVInfo> = test::read_body_json(resp).await;
        assert_eq!(body.len(), 1);
        assert_eq!(body[0].bio, expected_cv.bio);
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
        let token = jwt_service
            .generate_access_token(user_id, true)
            .expect("Failed to generate token");

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
        assert_eq!(resp.status(), 200);

        let body: Vec<CVInfo> = test::read_body_json(resp).await;
        assert_eq!(body.len(), 0); // Empty array when no CVs
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
        let token = jwt_service
            .generate_access_token(user_id, true)
            .expect("Failed to generate token");

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

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(jwt_service))
                .service(get_cvs_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/cvs")
            // No Authorization header
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 401); // Unauthorized
    }

    #[actix_web::test]
    async fn test_get_cv_handler_invalid_token() {
        let fetch_uc = MockFetchCVUseCase::new();

        let app_state = TestAppStateBuilder::default()
            .with_fetch_cv(fetch_uc)
            .build();

        let jwt_service = create_test_jwt_service();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(jwt_service))
                .service(get_cvs_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/cvs")
            .insert_header(("Authorization", "Bearer invalid_token_here"))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 401); // Unauthorized
    }

    #[actix_web::test]
    async fn test_get_cv_handler_unverified_user() {
        let fetch_uc = MockFetchCVUseCase::new();

        let app_state = TestAppStateBuilder::default()
            .with_fetch_cv(fetch_uc)
            .build();

        let jwt_service = create_test_jwt_service();

        let user_id = Uuid::new_v4();
        // Generate token for UNVERIFIED user (is_verified = false)
        let token = jwt_service
            .generate_access_token(user_id, false)
            .expect("Failed to generate token");

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
        assert_eq!(resp.status(), 403); // Forbidden - user not verified
    }
}
