use crate::auth::adapter::incoming::web::extractors::auth::VerifiedUser;
use crate::cv::application::ports::outgoing::PatchCVData;
use crate::cv::application::use_cases::patch_cv::PatchCVError;
use crate::cv::domain::entities::{CoreSkill, Education, Experience, HighlightedProject};
use crate::AppState;
use actix_web::patch;
use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReplaceOp<T> {
    pub replace: Vec<T>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PatchCVRequest {
    pub bio: Option<String>,
    pub role: Option<String>,
    pub photo_url: Option<String>,

    pub core_skills: Option<ReplaceOp<CoreSkill>>,
    pub educations: Option<ReplaceOp<Education>>,
    pub experiences: Option<ReplaceOp<Experience>>,
    pub highlighted_projects: Option<ReplaceOp<HighlightedProject>>,
}

#[patch("/api/cvs/{cv_id}")]
pub async fn patch_cv_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    req: web::Json<PatchCVRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let cv_id = path.into_inner();

    let patch_data = PatchCVData {
        bio: req.bio.clone(),
        role: req.role.clone(),
        photo_url: req.photo_url.clone(),
        core_skills: req.core_skills.as_ref().map(|op| op.replace.clone()),
        educations: req.educations.as_ref().map(|op| op.replace.clone()),
        experiences: req.experiences.as_ref().map(|op| op.replace.clone()),
        highlighted_projects: req
            .highlighted_projects
            .as_ref()
            .map(|op| op.replace.clone()),
    };

    match data
        .patch_cv_use_case
        .execute(user.user_id, cv_id, patch_data)
        .await
    {
        Ok(cv) => HttpResponse::Ok().json(cv),
        Err(PatchCVError::CVNotFound) => HttpResponse::NotFound().finish(),
        Err(PatchCVError::RepositoryError(e)) => HttpResponse::InternalServerError().body(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cv::domain::entities::CVInfo;
    use crate::tests::support::app_state_builder::TestAppStateBuilder;
    use crate::{
        cv::application::use_cases::patch_cv::IPatchCVUseCase,
        tests::support::auth_helper::test_helpers::create_test_jwt_service,
    };
    use actix_web::{test, web, App};
    use async_trait::async_trait;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use uuid::Uuid;

    // ==================== MOCK IMPLEMENTATIONS ====================

    #[derive(Clone)]
    struct MockPatchCvUseCase {
        should_fail: Arc<Mutex<Option<PatchCVError>>>,
        cv: Arc<Mutex<Option<CVInfo>>>,
    }

    impl MockPatchCvUseCase {
        fn new() -> Self {
            Self {
                should_fail: Arc::new(Mutex::new(None)),
                cv: Arc::new(Mutex::new(None)),
            }
        }

        async fn set_error(&self, error: PatchCVError) {
            *self.should_fail.lock().await = Some(error);
        }

        async fn set_success(&self, cv: CVInfo) {
            *self.cv.lock().await = Some(cv);
            *self.should_fail.lock().await = None;
        }
    }

    #[async_trait]
    impl IPatchCVUseCase for MockPatchCvUseCase {
        async fn execute(
            &self,
            _user_id: Uuid,
            _cv_id: Uuid,
            _data: PatchCVData,
        ) -> Result<CVInfo, PatchCVError> {
            if let Some(err) = self.should_fail.lock().await.clone() {
                return Err(err);
            }

            if let Some(cv) = self.cv.lock().await.clone() {
                return Ok(cv);
            }

            panic!("MockPatchCvUseCase called without configured result");
        }
    }

    #[actix_web::test]
    async fn test_patch_cv_handler_success() {
        // Arrange
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let patch_uc = MockPatchCvUseCase::new();

        let expected_cv = CVInfo {
            id: cv_id,
            user_id,
            bio: "Updated bio".to_string(),
            role: "Engineer".to_string(),
            photo_url: "https://example.com/photo.jpg".to_string(),
            core_skills: vec![],
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
        };

        patch_uc.set_success(expected_cv.clone()).await;

        let app_state = TestAppStateBuilder::default()
            .with_patch_cv(patch_uc)
            .build();

        let jwt_service = create_test_jwt_service();
        let token = jwt_service
            .generate_access_token(user_id, true)
            .expect("Failed to generate token");

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(jwt_service))
                .service(patch_cv_handler),
        )
        .await;

        // Act
        let req = test::TestRequest::patch()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(PatchCVRequest {
                bio: Some("Updated bio".to_string()),
                role: Some("Engineer".to_string()),
                photo_url: None,
                core_skills: None,
                educations: None,
                experiences: None,
                highlighted_projects: None,
            })
            .to_request();

        let resp = test::call_service(&app, req).await;

        // Assert
        assert_eq!(resp.status(), 200);

        let body: CVInfo = test::read_body_json(resp).await;
        assert_eq!(body.bio, "Updated bio");
        assert_eq!(body.role, "Engineer");
    }

    #[actix_web::test]
    async fn test_patch_cv_handler_partial_update() {
        // Arrange
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let patch_uc = MockPatchCvUseCase::new();

        let expected_cv = CVInfo {
            id: cv_id,
            user_id,
            bio: "Original bio".to_string(),
            role: "Updated role only".to_string(),
            photo_url: "https://example.com/photo.jpg".to_string(),
            core_skills: vec![],
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
        };

        patch_uc.set_success(expected_cv.clone()).await;

        let app_state = TestAppStateBuilder::default()
            .with_patch_cv(patch_uc)
            .build();

        let jwt_service = create_test_jwt_service();
        let token = jwt_service
            .generate_access_token(user_id, true)
            .expect("Failed to generate token");

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(jwt_service))
                .service(patch_cv_handler),
        )
        .await;

        // Act - Only update role
        let req = test::TestRequest::patch()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(PatchCVRequest {
                bio: None,
                role: Some("Updated role only".to_string()),
                photo_url: None,
                core_skills: None,
                educations: None,
                experiences: None,
                highlighted_projects: None,
            })
            .to_request();

        let resp = test::call_service(&app, req).await;

        // Assert
        assert_eq!(resp.status(), 200);

        let body: CVInfo = test::read_body_json(resp).await;
        assert_eq!(body.role, "Updated role only");
    }

    #[actix_web::test]
    async fn test_patch_cv_handler_cv_not_found() {
        // Arrange
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let patch_uc = MockPatchCvUseCase::new();
        patch_uc.set_error(PatchCVError::CVNotFound).await;

        let app_state = TestAppStateBuilder::default()
            .with_patch_cv(patch_uc)
            .build();

        let jwt_service = create_test_jwt_service();
        let token = jwt_service
            .generate_access_token(user_id, true)
            .expect("Failed to generate token");

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(jwt_service))
                .service(patch_cv_handler),
        )
        .await;

        // Act
        let req = test::TestRequest::patch()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(serde_json::json!({ "bio": "test" }))
            .to_request();

        let resp = test::call_service(&app, req).await;

        // Assert
        assert_eq!(resp.status(), 404);
    }

    #[actix_web::test]
    async fn test_patch_cv_handler_repository_error() {
        // Arrange
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let patch_uc = MockPatchCvUseCase::new();
        patch_uc
            .set_error(PatchCVError::RepositoryError(
                "DB update failed".to_string(),
            ))
            .await;

        let app_state = TestAppStateBuilder::default()
            .with_patch_cv(patch_uc)
            .build();

        let jwt_service = create_test_jwt_service();
        let token = jwt_service
            .generate_access_token(user_id, true)
            .expect("Failed to generate token");

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(jwt_service))
                .service(patch_cv_handler),
        )
        .await;

        // Act
        let req = test::TestRequest::patch()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(serde_json::json!({ "bio": "test" }))
            .to_request();

        let resp = test::call_service(&app, req).await;

        // Assert
        assert_eq!(resp.status(), 500);

        let body = test::read_body(resp).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("DB update failed"));
    }

    #[actix_web::test]
    async fn test_patch_cv_handler_missing_authorization() {
        // Arrange
        let cv_id = Uuid::new_v4();

        let patch_uc = MockPatchCvUseCase::new();

        let app_state = TestAppStateBuilder::default()
            .with_patch_cv(patch_uc)
            .build();

        let jwt_service = create_test_jwt_service();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(jwt_service))
                .service(patch_cv_handler),
        )
        .await;

        // Act - No Authorization header
        let req = test::TestRequest::patch()
            .uri(&format!("/api/cvs/{}", cv_id))
            .set_json(serde_json::json!({ "bio": "test" }))
            .to_request();

        let resp = test::call_service(&app, req).await;

        // Assert
        assert_eq!(resp.status(), 401); // Unauthorized
    }

    #[actix_web::test]
    async fn test_patch_cv_handler_invalid_token() {
        // Arrange
        let cv_id = Uuid::new_v4();

        let patch_uc = MockPatchCvUseCase::new();

        let app_state = TestAppStateBuilder::default()
            .with_patch_cv(patch_uc)
            .build();

        let jwt_service = create_test_jwt_service();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(jwt_service))
                .service(patch_cv_handler),
        )
        .await;

        // Act - Invalid token
        let req = test::TestRequest::patch()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", "Bearer invalid_token_here"))
            .set_json(serde_json::json!({ "bio": "test" }))
            .to_request();

        let resp = test::call_service(&app, req).await;

        // Assert
        assert_eq!(resp.status(), 401); // Unauthorized
    }

    #[actix_web::test]
    async fn test_patch_cv_handler_unverified_user() {
        // Arrange
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let patch_uc = MockPatchCvUseCase::new();

        let app_state = TestAppStateBuilder::default()
            .with_patch_cv(patch_uc)
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
                .service(patch_cv_handler),
        )
        .await;

        // Act
        let req = test::TestRequest::patch()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(serde_json::json!({ "bio": "test" }))
            .to_request();

        let resp = test::call_service(&app, req).await;

        // Assert
        assert_eq!(resp.status(), 403); // Forbidden - user not verified
    }
}
