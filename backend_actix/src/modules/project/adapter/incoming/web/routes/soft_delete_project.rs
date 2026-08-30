use actix_web::{delete, web, Responder};
use tracing::error;
use uuid::Uuid;

use crate::{
    api::schemas::ErrorResponse,
    auth::adapter::incoming::web::extractors::auth::VerifiedUser,
    auth::application::domain::entities::UserId,
    modules::project::application::ports::incoming::use_cases::SoftDeleteProjectError,
    shared::api::ApiResponse, AppState,
};

/// Soft-delete a project
///
/// Marks the project deleted so it drops out of listings while the row and its
/// topic links survive. Use `DELETE /api/projects/{project_id}/hard` to remove
/// it outright.
#[utoipa::path(
    delete,
    path = "/api/projects/{project_id}",
    tag = "projects",
    params(
        ("project_id" = Uuid, Path, description = "Identifier of the project to archive")
    ),
    responses(
        (status = 204, description = "Project archived successfully"),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
        (
            status = 404,
            description = "Project not found, or owned by another user",
            body = ErrorResponse,
            example = json!({
                "success": false,
                "error": { "code": "PROJECT_NOT_FOUND", "message": "Project not found" }
            })
        ),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[delete("/api/projects/{project_id}")]
pub async fn soft_delete_project_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> impl Responder {
    let owner = UserId::from(user.user_id);
    let project_id = path.into_inner();

    match data.project.soft_delete.execute(owner, project_id).await {
        Ok(_) => ApiResponse::no_content(),

        Err(SoftDeleteProjectError::ProjectNotFound) => {
            ApiResponse::not_found("PROJECT_NOT_FOUND", "Project not found")
        }

        Err(SoftDeleteProjectError::RepositoryError(msg)) => {
            error!("Failed to soft delete project: {}", msg);
            ApiResponse::internal_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use actix_web::{http::StatusCode, test, web, App};
    use async_trait::async_trait;
    use serde_json::Value;
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::auth::adapter::outgoing::jwt::{JwtConfig, JwtTokenService};
    use crate::auth::application::ports::outgoing::token_provider::TokenProvider;
    use crate::modules::project::application::ports::incoming::use_cases::{
        SoftDeleteProjectError, SoftDeleteProjectUseCase,
    };
    use crate::tests::support::app_state_builder::TestAppStateBuilder;

    /* --------------------------------------------------
     * Mock SoftDeleteProjectUseCase
     * -------------------------------------------------- */

    #[derive(Clone)]
    struct MockSoftDeleteProjectUseCase {
        result: Result<(), SoftDeleteProjectError>,
    }

    impl MockSoftDeleteProjectUseCase {
        fn success() -> Self {
            Self { result: Ok(()) }
        }

        fn error(err: SoftDeleteProjectError) -> Self {
            Self { result: Err(err) }
        }
    }

    #[async_trait]
    impl SoftDeleteProjectUseCase for MockSoftDeleteProjectUseCase {
        async fn execute(
            &self,
            _owner: UserId,
            _project_id: Uuid,
        ) -> Result<(), SoftDeleteProjectError> {
            self.result.clone()
        }
    }

    /* --------------------------------------------------
     * Helpers
     * -------------------------------------------------- */

    fn jwt_service() -> JwtTokenService {
        JwtTokenService::new(JwtConfig {
            issuer: "Lotion".to_string(),
            secret_key: "test_secret_key_for_testing_purposes_only".to_string(),
            access_token_expiry: 3600,
            refresh_token_expiry: 86400,
            verification_token_expiry: 86400,
            password_reset_expiry: 3600,
        })
    }

    fn token(user_id: Uuid, verified: bool) -> String {
        jwt_service()
            .generate_access_token(user_id, verified)
            .unwrap()
    }

    async fn call(
        uc: MockSoftDeleteProjectUseCase,
        project_id: Uuid,
        user_id: Uuid,
        verified: bool,
    ) -> actix_web::dev::ServiceResponse {
        let app_state = TestAppStateBuilder::default()
            .with_soft_delete_project(uc)
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(soft_delete_project_handler),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri(&format!("/api/projects/{}", project_id))
            .insert_header((
                "Authorization",
                format!("Bearer {}", token(user_id, verified)),
            ))
            .to_request();

        test::call_service(&app, req).await
    }

    /* --------------------------------------------------
     * Tests
     * -------------------------------------------------- */

    #[actix_web::test]
    async fn test_soft_delete_project_success_no_content() {
        let resp = call(
            MockSoftDeleteProjectUseCase::success(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            true,
        )
        .await;

        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[actix_web::test]
    async fn test_soft_delete_project_project_not_found() {
        let resp = call(
            MockSoftDeleteProjectUseCase::error(SoftDeleteProjectError::ProjectNotFound),
            Uuid::new_v4(),
            Uuid::new_v4(),
            true,
        )
        .await;

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "PROJECT_NOT_FOUND");
    }

    #[actix_web::test]
    async fn test_soft_delete_project_repository_error_internal_error() {
        let resp = call(
            MockSoftDeleteProjectUseCase::error(SoftDeleteProjectError::RepositoryError(
                "db down".to_string(),
            )),
            Uuid::new_v4(),
            Uuid::new_v4(),
            true,
        )
        .await;

        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "INTERNAL_ERROR");
    }

    #[actix_web::test]
    async fn test_soft_delete_project_unverified_user_forbidden() {
        let resp = call(
            MockSoftDeleteProjectUseCase::success(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            false,
        )
        .await;

        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "EMAIL_NOT_VERIFIED");
    }
}
