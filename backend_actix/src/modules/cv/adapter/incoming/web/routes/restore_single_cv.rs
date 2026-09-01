use crate::shared::api::ErrorCode;
use actix_web::{post, web, Responder};
use tracing::error;
use uuid::Uuid;

use crate::{
    api::schemas::{ErrorResponse, SuccessResponse},
    auth::{
        adapter::incoming::web::extractors::auth::VerifiedUser,
        application::domain::entities::UserId,
    },
    cv::adapter::incoming::web::dto::CvResponse,
    cv::application::use_cases::restore_cv::RestoreCVError,
    shared::api::ApiResponse,
    AppState,
};

/// Restore an archived CV
///
/// Brings back a CV archived with `DELETE /api/cvs/{cv_id}` and returns it, so
/// no follow-up fetch is needed.
///
/// Idempotent: restoring a CV that is not archived succeeds and returns it
/// unchanged.
#[utoipa::path(
    post,
    path = "/api/cvs/{cv_id}/restore",
    tag = "cvs",
    params(
        ("cv_id" = Uuid, Path, description = "Identifier of the CV to restore")
    ),
    responses(
        (
            status = 200,
            description = "CV restored, or was already active",
            body = inline(SuccessResponse<CvResponse>)
        ),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (
            status = 403,
            description = "Email not verified, or the CV belongs to another user",
            body = ErrorResponse,
            example = json!({
                "success": false,
                "error": {
                    "code": "CV_UNAUTHORIZED",
                    "message": "You are not authorized to restore this CV"
                }
            })
        ),
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
#[post("/api/cvs/{cv_id}/restore")]
pub async fn restore_cv_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    app_data: web::Data<AppState>,
) -> impl Responder {
    let cv_id = path.into_inner();

    match app_data
        .restore_cv_use_case
        .execute(UserId::from(user.user_id), cv_id)
        .await
    {
        Ok(cv) => ApiResponse::success(CvResponse::from(cv)),

        Err(RestoreCVError::CVNotFound) => {
            ApiResponse::not_found(ErrorCode::CvNotFound, "CV not found")
        }

        Err(RestoreCVError::Unauthorized) => ApiResponse::forbidden(
            ErrorCode::CvUnauthorized,
            "You are not authorized to restore this CV",
        ),

        Err(RestoreCVError::RepositoryError(e)) => {
            error!("Repository error restoring CV: {}", e);
            ApiResponse::internal_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::application::ports::outgoing::token_provider::TokenProvider;
    use crate::cv::application::use_cases::restore_cv::RestoreDeletedCvUseCase;
    use crate::cv::domain::entities::CVInfo;
    use crate::tests::support::app_state_builder::TestAppStateBuilder;
    use crate::tests::support::auth_helper::test_helpers::create_test_jwt_service;
    use actix_web::{http::StatusCode, test, web, App};
    use async_trait::async_trait;
    use serde_json::Value;
    use std::sync::Arc;

    struct MockRestoreCv {
        result: Result<CVInfo, RestoreCVError>,
    }

    #[async_trait]
    impl RestoreDeletedCvUseCase for MockRestoreCv {
        async fn execute(&self, _u: UserId, _c: Uuid) -> Result<CVInfo, RestoreCVError> {
            self.result.clone()
        }
    }

    fn a_cv(cv_id: Uuid, user_id: Uuid) -> CVInfo {
        CVInfo {
            id: cv_id,
            user_id,
            role: "Developer".to_string(),
            display_name: "Test User".to_string(),
            bio: "Test bio".to_string(),
            photo_url: "https://example.com/photo.jpg".to_string(),
            core_skills: vec![],
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
            contact_info: vec![],
        }
    }

    async fn call(
        result: Result<CVInfo, RestoreCVError>,
        verified: bool,
    ) -> actix_web::dev::ServiceResponse {
        let jwt = create_test_jwt_service();
        let token = jwt.generate_access_token(Uuid::new_v4(), verified).unwrap();
        let provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt);

        let app = test::init_service(
            App::new()
                .app_data(
                    TestAppStateBuilder::default()
                        .with_restore_cv(MockRestoreCv { result })
                        .build(),
                )
                .app_data(web::Data::new(provider))
                .service(restore_cv_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri(&format!("/api/cvs/{}/restore", Uuid::new_v4()))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();

        test::call_service(&app, req).await
    }

    #[actix_web::test]
    async fn restores_and_returns_the_cv() {
        let (cv_id, user_id) = (Uuid::new_v4(), Uuid::new_v4());
        let resp = call(Ok(a_cv(cv_id, user_id)), true).await;

        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        // The restored CV comes back on the response, so no follow-up GET.
        assert_eq!(body["data"]["id"], cv_id.to_string());
        assert_eq!(body["data"]["user_id"], user_id.to_string());
        assert_eq!(body["data"]["role"], "Developer");
    }

    #[actix_web::test]
    async fn missing_cv_is_not_found() {
        let resp = call(Err(RestoreCVError::CVNotFound), true).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "CV_NOT_FOUND");
    }

    #[actix_web::test]
    async fn another_users_cv_is_forbidden() {
        let resp = call(Err(RestoreCVError::Unauthorized), true).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "CV_UNAUTHORIZED");
    }

    #[actix_web::test]
    async fn repository_error_is_internal_error() {
        let resp = call(Err(RestoreCVError::RepositoryError("db down".into())), true).await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "INTERNAL_ERROR");
    }

    #[actix_web::test]
    async fn unverified_user_is_forbidden() {
        let resp = call(Ok(a_cv(Uuid::new_v4(), Uuid::new_v4())), false).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "EMAIL_NOT_VERIFIED");
    }
}
