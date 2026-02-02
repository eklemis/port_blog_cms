use actix_web::{delete, web, Responder};
use tracing::error;
use uuid::Uuid;

use crate::{
    auth::adapter::incoming::web::extractors::auth::VerifiedUser,
    auth::application::domain::entities::UserId,
    modules::project::application::ports::incoming::use_cases::HardDeleteProjectError,
    shared::api::ApiResponse, AppState,
};

#[delete("/api/projects/{project_id}/hard")]
pub async fn hard_delete_project_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> impl Responder {
    let owner = UserId::from(user.user_id);
    let project_id = path.into_inner();

    match data.project.hard_delete.execute(owner, project_id).await {
        Ok(_) => ApiResponse::no_content(),

        Err(HardDeleteProjectError::ProjectNotFound) => {
            ApiResponse::not_found("PROJECT_NOT_FOUND", "Project not found")
        }

        Err(HardDeleteProjectError::RepositoryError(msg)) => {
            error!("Failed to hard delete project: {}", msg);
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
        HardDeleteProjectError, HardDeleteProjectUseCase,
    };
    use crate::tests::support::app_state_builder::TestAppStateBuilder;

    /* --------------------------------------------------
     * Mock HardDeleteProjectUseCase
     * -------------------------------------------------- */

    #[derive(Clone)]
    struct MockHardDeleteProjectUseCase {
        result: Result<(), HardDeleteProjectError>,
    }

    impl MockHardDeleteProjectUseCase {
        fn success() -> Self {
            Self { result: Ok(()) }
        }

        fn error(err: HardDeleteProjectError) -> Self {
            Self { result: Err(err) }
        }
    }

    #[async_trait]
    impl HardDeleteProjectUseCase for MockHardDeleteProjectUseCase {
        async fn execute(
            &self,
            _owner: UserId,
            _project_id: Uuid,
        ) -> Result<(), HardDeleteProjectError> {
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
        })
    }

    fn token(user_id: Uuid, verified: bool) -> String {
        jwt_service()
            .generate_access_token(user_id, verified)
            .unwrap()
    }

    /* --------------------------------------------------
     * Tests
     * -------------------------------------------------- */

    #[actix_web::test]
    async fn test_hard_delete_project_success_no_content() {
        let user_id = Uuid::new_v4();
        let project_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_hard_delete_project(MockHardDeleteProjectUseCase::success())
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(hard_delete_project_handler),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri(&format!("/api/projects/{}/hard", project_id))
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        // no body expected, but Actix may return empty; don't parse JSON here
    }

    #[actix_web::test]
    async fn test_hard_delete_project_project_not_found() {
        let user_id = Uuid::new_v4();
        let project_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_hard_delete_project(MockHardDeleteProjectUseCase::error(
                HardDeleteProjectError::ProjectNotFound,
            ))
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(hard_delete_project_handler),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri(&format!("/api/projects/{}/hard", project_id))
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "PROJECT_NOT_FOUND");
    }

    #[actix_web::test]
    async fn test_hard_delete_project_repository_error_internal_error() {
        let user_id = Uuid::new_v4();
        let project_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_hard_delete_project(MockHardDeleteProjectUseCase::error(
                HardDeleteProjectError::RepositoryError("db down".to_string()),
            ))
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(hard_delete_project_handler),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri(&format!("/api/projects/{}/hard", project_id))
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "INTERNAL_ERROR");
    }

    #[actix_web::test]
    async fn test_hard_delete_project_unverified_user_forbidden() {
        let user_id = Uuid::new_v4();
        let project_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_hard_delete_project(MockHardDeleteProjectUseCase::success())
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(hard_delete_project_handler),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri(&format!("/api/projects/{}/hard", project_id))
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, false))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "EMAIL_NOT_VERIFIED");
    }
}
