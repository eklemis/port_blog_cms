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
        cv::{application::use_cases::fetch_cv_by_id::IFetchCVByIdUseCase, domain::CVInfo},
        tests::support::{
            app_state_builder::TestAppStateBuilder,
            auth_helper::test_helpers::create_test_jwt_service,
        },
    };
    use actix_web::{test, web, App};
    use tokio::sync::Mutex;
    use uuid::Uuid;

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

    #[actix_web::test]
    async fn test_get_cv_by_id_handler_success() {
        // Arrange
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let expected_cv = CVInfo {
            id: cv_id,
            user_id,
            bio: "Senior Backend Engineer".to_string(),
            role: "Backend Engineer".to_string(),
            photo_url: "https://example.com/photo.jpg".to_string(),
            core_skills: vec![],
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
        };

        let fetch_cv_by_id_uc = {
            let uc = MockFetchCVByIdUseCase::new();
            *uc.cv.lock().await = Some(expected_cv.clone());
            uc
        };

        let app_state = TestAppStateBuilder::default()
            .with_fetch_cv_by_id(fetch_cv_by_id_uc)
            .build();

        let jwt_service = create_test_jwt_service();
        let token = jwt_service
            .generate_access_token(user_id, true)
            .expect("Failed to generate token");

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(jwt_service))
                .service(get_cv_by_id_handler),
        )
        .await;

        // Act
        let req = test::TestRequest::get()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;

        // Assert
        assert_eq!(resp.status(), 200);

        let body: CVInfo = test::read_body_json(resp).await;
        assert_eq!(body.id, cv_id);
        assert_eq!(body.bio, expected_cv.bio);
    }

    #[actix_web::test]
    async fn test_get_cv_by_id_handler_cv_not_found() {
        // Arrange
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let fetch_cv_by_id_uc = {
            let uc = MockFetchCVByIdUseCase::new();
            uc.should_fail
                .lock()
                .await
                .replace(FetchCVByIdError::CVNotFound);
            uc
        };

        let app_state = TestAppStateBuilder::default()
            .with_fetch_cv_by_id(fetch_cv_by_id_uc)
            .build();

        let jwt_service = create_test_jwt_service();
        let token = jwt_service
            .generate_access_token(user_id, true)
            .expect("Failed to generate token");

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(jwt_service))
                .service(get_cv_by_id_handler),
        )
        .await;

        // Act
        let req = test::TestRequest::get()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;

        // Assert
        assert_eq!(resp.status(), 404);
    }

    #[actix_web::test]
    async fn test_get_cv_by_id_handler_repository_error() {
        // Arrange
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let fetch_cv_by_id_uc = {
            let uc = MockFetchCVByIdUseCase::new();
            uc.should_fail
                .lock()
                .await
                .replace(FetchCVByIdError::RepositoryError(
                    "DB read failed".to_string(),
                ));
            uc
        };

        let app_state = TestAppStateBuilder::default()
            .with_fetch_cv_by_id(fetch_cv_by_id_uc)
            .build();

        let jwt_service = create_test_jwt_service();
        let token = jwt_service
            .generate_access_token(user_id, true)
            .expect("Failed to generate token");

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(jwt_service))
                .service(get_cv_by_id_handler),
        )
        .await;

        // Act
        let req = test::TestRequest::get()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;

        // Assert
        assert_eq!(resp.status(), 500);

        let body = test::read_body(resp).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("DB read failed"));
    }

    #[actix_web::test]
    async fn test_get_cv_by_id_handler_missing_authorization() {
        // Arrange
        let cv_id = Uuid::new_v4();

        let fetch_cv_by_id_uc = MockFetchCVByIdUseCase::new();

        let app_state = TestAppStateBuilder::default()
            .with_fetch_cv_by_id(fetch_cv_by_id_uc)
            .build();

        let jwt_service = create_test_jwt_service();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(jwt_service))
                .service(get_cv_by_id_handler),
        )
        .await;

        // Act - No Authorization header
        let req = test::TestRequest::get()
            .uri(&format!("/api/cvs/{}", cv_id))
            .to_request();

        let resp = test::call_service(&app, req).await;

        // Assert
        assert_eq!(resp.status(), 401); // Unauthorized
    }

    #[actix_web::test]
    async fn test_get_cv_by_id_handler_invalid_token() {
        // Arrange
        let cv_id = Uuid::new_v4();

        let fetch_cv_by_id_uc = MockFetchCVByIdUseCase::new();

        let app_state = TestAppStateBuilder::default()
            .with_fetch_cv_by_id(fetch_cv_by_id_uc)
            .build();

        let jwt_service = create_test_jwt_service();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(jwt_service))
                .service(get_cv_by_id_handler),
        )
        .await;

        // Act - Invalid token
        let req = test::TestRequest::get()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", "Bearer invalid_token_here"))
            .to_request();

        let resp = test::call_service(&app, req).await;

        // Assert
        assert_eq!(resp.status(), 401); // Unauthorized
    }

    #[actix_web::test]
    async fn test_get_cv_by_id_handler_unverified_user() {
        // Arrange
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let fetch_cv_by_id_uc = MockFetchCVByIdUseCase::new();

        let app_state = TestAppStateBuilder::default()
            .with_fetch_cv_by_id(fetch_cv_by_id_uc)
            .build();

        let jwt_service = create_test_jwt_service();
        // Generate token for UNVERIFIED user (is_verified = false)
        let token = jwt_service
            .generate_access_token(user_id, false)
            .expect("Failed to generate token");

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(jwt_service))
                .service(get_cv_by_id_handler),
        )
        .await;

        // Act
        let req = test::TestRequest::get()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;

        // Assert
        assert_eq!(resp.status(), 403); // Forbidden - user not verified
    }
}
