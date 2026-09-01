use crate::shared::api::ErrorCode;
use actix_web::{delete, web, Responder};
use tracing::error;
use uuid::Uuid;

use crate::{
    api::schemas::ErrorResponse,
    auth::{
        adapter::incoming::web::extractors::auth::VerifiedUser,
        application::domain::entities::UserId,
    },
    cv::application::use_cases::soft_delete_cv::SoftDeleteCVError,
    shared::api::ApiResponse,
    AppState,
};

/// Archive a CV
///
/// Marks the CV deleted so it drops out of listings while the row survives, and
/// can be brought back with `POST /api/cvs/{cv_id}/restore`. Use
/// `DELETE /api/cvs/{cv_id}/hard` to remove it outright.
///
/// Idempotent: archiving an already-archived CV returns 204 rather than an
/// error.
#[utoipa::path(
    delete,
    path = "/api/cvs/{cv_id}",
    tag = "cvs",
    params(
        ("cv_id" = Uuid, Path, description = "Identifier of the CV to archive")
    ),
    responses(
        (status = 204, description = "CV archived, or was already archived"),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (
            status = 403,
            description = "Email not verified, or the CV belongs to another user",
            body = ErrorResponse,
            example = json!({
                "success": false,
                "error": {
                    "code": "CV_UNAUTHORIZED",
                    "message": "You are not authorized to delete this CV"
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
#[delete("/api/cvs/{cv_id}")]
pub async fn soft_delete_cv_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    app_data: web::Data<AppState>,
) -> impl Responder {
    let cv_id = path.into_inner();

    match app_data
        .soft_delete_cv_use_case
        .execute(UserId::from(user.user_id), cv_id)
        .await
    {
        Ok(()) => ApiResponse::no_content(),

        Err(SoftDeleteCVError::CVNotFound) => {
            ApiResponse::not_found(ErrorCode::CvNotFound, "CV not found")
        }

        Err(SoftDeleteCVError::Unauthorized) => ApiResponse::forbidden(
            ErrorCode::CvUnauthorized,
            "You are not authorized to delete this CV",
        ),

        Err(SoftDeleteCVError::RepositoryError(e)) => {
            error!("Repository error archiving CV: {}", e);
            ApiResponse::internal_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::application::ports::outgoing::token_provider::TokenProvider;
    use crate::cv::application::use_cases::soft_delete_cv::SoftDeleteCvUseCase;
    use crate::tests::support::app_state_builder::TestAppStateBuilder;
    use crate::tests::support::auth_helper::test_helpers::create_test_jwt_service;
    use actix_web::{http::StatusCode, test, web, App};
    use async_trait::async_trait;
    use serde_json::Value;
    use std::sync::Arc;

    struct MockSoftDeleteCv {
        result: Result<(), SoftDeleteCVError>,
    }

    #[async_trait]
    impl SoftDeleteCvUseCase for MockSoftDeleteCv {
        async fn execute(&self, _u: UserId, _c: Uuid) -> Result<(), SoftDeleteCVError> {
            self.result.clone()
        }
    }

    async fn call(
        result: Result<(), SoftDeleteCVError>,
        verified: bool,
    ) -> actix_web::dev::ServiceResponse {
        let jwt = create_test_jwt_service();
        let token = jwt.generate_access_token(Uuid::new_v4(), verified).unwrap();
        let provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt);

        let app = test::init_service(
            App::new()
                .app_data(
                    TestAppStateBuilder::default()
                        .with_soft_delete_cv(MockSoftDeleteCv { result })
                        .build(),
                )
                .app_data(web::Data::new(provider))
                .service(soft_delete_cv_handler),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri(&format!("/api/cvs/{}", Uuid::new_v4()))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();

        test::call_service(&app, req).await
    }

    #[actix_web::test]
    async fn archives_and_returns_no_content() {
        assert_eq!(call(Ok(()), true).await.status(), StatusCode::NO_CONTENT);
    }

    #[actix_web::test]
    async fn missing_cv_is_not_found() {
        let resp = call(Err(SoftDeleteCVError::CVNotFound), true).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "CV_NOT_FOUND");
    }

    #[actix_web::test]
    async fn another_users_cv_is_forbidden() {
        let resp = call(Err(SoftDeleteCVError::Unauthorized), true).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "CV_UNAUTHORIZED");
    }

    #[actix_web::test]
    async fn repository_error_is_internal_error() {
        let resp = call(
            Err(SoftDeleteCVError::RepositoryError("db down".into())),
            true,
        )
        .await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "INTERNAL_ERROR");
    }

    #[actix_web::test]
    async fn unverified_user_is_forbidden() {
        let resp = call(Ok(()), false).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "EMAIL_NOT_VERIFIED");
    }
}
