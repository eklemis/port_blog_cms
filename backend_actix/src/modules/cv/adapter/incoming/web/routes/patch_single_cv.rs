use crate::api::schemas::{ErrorResponse, SuccessResponse};
use crate::auth::adapter::incoming::web::extractors::auth::VerifiedUser;
use crate::cv::adapter::incoming::web::dto::{
    ContactDetailDto, CoreSkillDto, CvResponse, EducationDto, ExperienceDto, HighlightedProjectDto,
};
use crate::cv::application::ports::outgoing::PatchCVData;
use crate::cv::application::use_cases::patch_cv::PatchCVError;
use crate::shared::api::{ApiResponse, ErrorCode};
use crate::AppState;
use actix_web::patch;
use actix_web::{web, Responder};
use serde::{Deserialize, Serialize};
use tracing::error;
use utoipa::ToSchema;
use uuid::Uuid;

/// Wholesale replacement of a collection field.
///
/// Omitting the field leaves the stored collection untouched; supplying it
/// replaces every element. There is no append or per-element patch.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(bound = "T: Serialize + serde::de::DeserializeOwned")]
pub struct ReplaceOp<T> {
    /// The full replacement list. There is no per-item patch: a list is replaced
    /// wholesale or left alone.
    pub replace: Vec<T>,
}

/// Partial CV update. Every field is optional; omitted fields are left as-is.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct PatchCVRequest {
    /// New bio, or `None` to leave it.
    pub bio: Option<String>,
    /// New role, or `None` to leave it.
    pub role: Option<String>,
    /// New portrait URL, or `None` to leave it.
    pub photo_url: Option<String>,
    /// New display name, or `None` to leave it.
    pub display_name: Option<String>,

    /// Replaces the skills list when present.
    pub core_skills: Option<ReplaceOp<CoreSkillDto>>,
    /// Replaces the education list when present.
    pub educations: Option<ReplaceOp<EducationDto>>,
    /// Replaces the work history when present.
    pub experiences: Option<ReplaceOp<ExperienceDto>>,
    /// Replaces the featured projects when present.
    pub highlighted_projects: Option<ReplaceOp<HighlightedProjectDto>>,
    /// Replaces the contact rows when present.
    pub contact_info: Option<ReplaceOp<ContactDetailDto>>,
}

/// Partially update a CV
///
/// Only the fields present in the body are changed. Collection fields use the
/// `{ "replace": [...] }` wrapper and are swapped out entirely; omitting a
/// collection leaves it untouched.
#[utoipa::path(
    patch,
    path = "/api/cvs/{cv_id}",
    tag = "cvs",
    params(
        ("cv_id" = Uuid, Path, description = "Identifier of the CV to update")
    ),
    request_body = PatchCVRequest,
    responses(
        (
            status = 200,
            description = "CV updated successfully",
            body = inline(SuccessResponse<CvResponse>)
        ),
        (status = 400, description = "Malformed request body", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
        (
            status = 404,
            description = "CV not found",
            body = ErrorResponse,
            example = json!({
                "success": false,
                "error": { "code": "CV_NOT_FOUND", "message": "CV not found" }
            })
        ),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
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
        display_name: req.display_name.clone(),
        role: req.role.clone(),
        photo_url: req.photo_url.clone(),
        core_skills: req
            .core_skills
            .as_ref()
            .map(|op| op.replace.iter().cloned().map(Into::into).collect()),
        educations: req
            .educations
            .as_ref()
            .map(|op| op.replace.iter().cloned().map(Into::into).collect()),
        experiences: req
            .experiences
            .as_ref()
            .map(|op| op.replace.iter().cloned().map(Into::into).collect()),
        highlighted_projects: req
            .highlighted_projects
            .as_ref()
            .map(|op| op.replace.iter().cloned().map(Into::into).collect()),
        contact_info: req
            .contact_info
            .as_ref()
            .map(|op| op.replace.iter().cloned().map(Into::into).collect()),
    };

    match data
        .patch_cv_use_case
        .execute(user.user_id, cv_id, patch_data)
        .await
    {
        Ok(cv) => ApiResponse::success(CvResponse::from(cv)),
        Err(PatchCVError::CVNotFound) => {
            ApiResponse::not_found(ErrorCode::CvNotFound, "CV not found")
        }
        Err(PatchCVError::RepositoryError(e)) => {
            error!("Repository error patching CV: {}", e);
            ApiResponse::internal_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::application::ports::outgoing::token_provider::TokenProvider;
    // The handler no longer references domain entities; only these tests do,
    // to build the CVInfo the mocked use case returns.
    use crate::cv::domain::entities::{CVInfo, ContactDetail, ContactType, Education};
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

    /* --------------------------------------------------
     * Helpers
     * -------------------------------------------------- */

    fn base_patch_request() -> PatchCVRequest {
        PatchCVRequest {
            bio: None,
            role: None,
            display_name: None,
            photo_url: None,
            core_skills: None,
            educations: None,
            experiences: None,
            highlighted_projects: None,
            contact_info: None,
        }
    }

    /* --------------------------------------------------
     * Mock Patch CV Use Case
     * -------------------------------------------------- */

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
            cv_id: Uuid,
            data: PatchCVData,
        ) -> Result<CVInfo, PatchCVError> {
            if let Some(err) = self.should_fail.lock().await.clone() {
                return Err(err);
            }

            let existing = self
                .cv
                .lock()
                .await
                .clone()
                .expect("MockPatchCvUseCase called without set_success");

            Ok(CVInfo {
                id: cv_id,
                user_id: existing.user_id,
                display_name: data.display_name.unwrap_or(existing.display_name),
                role: data.role.unwrap_or(existing.role),
                bio: data.bio.unwrap_or(existing.bio),
                photo_url: data.photo_url.unwrap_or(existing.photo_url),
                core_skills: data.core_skills.unwrap_or(existing.core_skills),
                educations: data.educations.unwrap_or(existing.educations),
                experiences: data.experiences.unwrap_or(existing.experiences),
                highlighted_projects: data
                    .highlighted_projects
                    .unwrap_or(existing.highlighted_projects),
                contact_info: data.contact_info.unwrap_or(existing.contact_info),
            })
        }
    }

    /* --------------------------------------------------
     * Tests
     * -------------------------------------------------- */

    #[actix_web::test]
    async fn test_patch_cv_handler_success() {
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let patch_uc = MockPatchCvUseCase::new();

        let expected_cv = CVInfo {
            id: Uuid::new_v4(),
            user_id,
            display_name: "Initial Name".to_string(),
            bio: "Software Engineer with 5 years of experience".to_string(),
            role: "Initial Role".to_string(),
            photo_url: "https://example.com/photo.jpg".to_string(),
            core_skills: vec![],
            educations: vec![Education {
                degree: "Bachelor of Computer Science".to_string(),
                institution: "MIT".to_string(),
                graduation_year: 2018,
            }],
            experiences: vec![],
            highlighted_projects: vec![],
            contact_info: vec![ContactDetail {
                title: "Blog".to_string(),
                contact_type: ContactType::WebPage,
                content: "www.nonexist.blog.com".to_string(),
            }],
        };

        patch_uc.set_success(expected_cv).await;

        let app_state = TestAppStateBuilder::default()
            .with_patch_cv(patch_uc)
            .build();

        let jwt_service = create_test_jwt_service();
        let token = jwt_service.generate_access_token(user_id, true).unwrap();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service.clone());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(patch_cv_handler),
        )
        .await;

        let req = test::TestRequest::patch()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(PatchCVRequest {
                bio: Some("Updated bio".to_string()),
                role: Some("Engineer".to_string()),
                display_name: Some("Berto Fang".to_string()),
                ..base_patch_request()
            })
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert_eq!(body["data"]["bio"], "Updated bio");
        assert_eq!(body["data"]["role"], "Engineer");
        assert_eq!(body["data"]["display_name"], "Berto Fang");
        assert_eq!(body["data"]["contact_info"][0]["contact_type"], "WebPage");
        assert_eq!(body["data"]["contact_info"][0]["title"], "Blog");
        assert_eq!(
            body["data"]["contact_info"][0]["content"],
            "www.nonexist.blog.com"
        );
        assert!(body.get("error").is_none());
    }

    #[actix_web::test]
    async fn test_patch_cv_handler_partial_update() {
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let patch_uc = MockPatchCvUseCase::new();

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

        patch_uc.set_success(expected_cv).await;

        let app_state = TestAppStateBuilder::default()
            .with_patch_cv(patch_uc)
            .build();

        let jwt_service = create_test_jwt_service();
        let token = jwt_service.generate_access_token(user_id, true).unwrap();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service.clone());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(patch_cv_handler),
        )
        .await;

        let req = test::TestRequest::patch()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(PatchCVRequest {
                display_name: Some("Updated name here".to_string()),
                role: Some("Updated role here".to_string()),
                ..base_patch_request()
            })
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert_eq!(body["data"]["role"], "Updated role here");
        assert_eq!(body["data"]["display_name"], "Updated name here");
        assert!(body.get("error").is_none());
    }

    #[actix_web::test]
    async fn test_patch_cv_handler_cv_not_found() {
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let patch_uc = MockPatchCvUseCase::new();
        patch_uc.set_error(PatchCVError::CVNotFound).await;

        let app_state = TestAppStateBuilder::default()
            .with_patch_cv(patch_uc)
            .build();

        let jwt_service = create_test_jwt_service();
        let token = jwt_service.generate_access_token(user_id, true).unwrap();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service.clone());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(patch_cv_handler),
        )
        .await;

        let req = test::TestRequest::patch()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(serde_json::json!({ "bio": "test" }))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 404);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "CV_NOT_FOUND");
        assert_eq!(body["error"]["message"], "CV not found");
        assert!(body.get("data").is_none());
    }

    #[actix_web::test]
    async fn test_patch_cv_handler_repository_error() {
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
        let token = jwt_service.generate_access_token(user_id, true).unwrap();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service.clone());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(patch_cv_handler),
        )
        .await;

        let req = test::TestRequest::patch()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(serde_json::json!({ "bio": "test" }))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 500);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "INTERNAL_ERROR");
        assert_eq!(body["error"]["message"], "An unexpected error occurred");
        assert!(body.get("data").is_none());
    }

    #[actix_web::test]
    async fn test_patch_cv_handler_missing_authorization() {
        let cv_id = Uuid::new_v4();

        let patch_uc = MockPatchCvUseCase::new();

        let app_state = TestAppStateBuilder::default()
            .with_patch_cv(patch_uc)
            .build();

        let jwt_service = create_test_jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service.clone());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(patch_cv_handler),
        )
        .await;

        let req = test::TestRequest::patch()
            .uri(&format!("/api/cvs/{}", cv_id))
            .set_json(serde_json::json!({ "bio": "test" }))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 401);
    }

    #[actix_web::test]
    async fn test_patch_cv_handler_invalid_token() {
        let cv_id = Uuid::new_v4();

        let patch_uc = MockPatchCvUseCase::new();

        let app_state = TestAppStateBuilder::default()
            .with_patch_cv(patch_uc)
            .build();

        let jwt_service = create_test_jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service.clone());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(patch_cv_handler),
        )
        .await;

        let req = test::TestRequest::patch()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", "Bearer invalid_token_here"))
            .set_json(serde_json::json!({ "bio": "test" }))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 401);
    }

    #[actix_web::test]
    async fn test_patch_cv_handler_unverified_user() {
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let patch_uc = MockPatchCvUseCase::new();

        let app_state = TestAppStateBuilder::default()
            .with_patch_cv(patch_uc)
            .build();

        let jwt_service = create_test_jwt_service();
        let token = jwt_service.generate_access_token(user_id, false).unwrap();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service.clone());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(patch_cv_handler),
        )
        .await;

        let req = test::TestRequest::patch()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(serde_json::json!({ "bio": "test" }))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 403);
    }
}
