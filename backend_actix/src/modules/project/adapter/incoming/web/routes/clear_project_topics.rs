use actix_web::{delete, web, Responder};
use tracing::error;
use uuid::Uuid;

use crate::{
    auth::adapter::incoming::web::extractors::auth::VerifiedUser,
    auth::application::domain::entities::UserId,
    modules::project::application::ports::incoming::use_cases::ClearProjectTopicsError,
    shared::api::ApiResponse, AppState,
};

#[delete("/api/projects/{project_id}/topics/all")]
pub async fn clear_project_topics_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> impl Responder {
    let owner = UserId::from(user.user_id);
    let project_id = path.into_inner();

    match data.project.clear_topics.execute(owner, project_id).await {
        Ok(_) => ApiResponse::no_content(),

        Err(ClearProjectTopicsError::ProjectNotFound) => {
            ApiResponse::not_found("PROJECT_NOT_FOUND", "Project not found")
        }

        Err(ClearProjectTopicsError::RepositoryError(msg)) => {
            error!("Failed to clear project topics: {}", msg);
            ApiResponse::internal_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use actix_web::{http::StatusCode, test, web, App};
    use async_trait::async_trait;
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::auth::adapter::outgoing::jwt::{JwtConfig, JwtTokenService};
    use crate::auth::application::ports::outgoing::token_provider::TokenProvider;
    use crate::modules::project::application::ports::incoming::use_cases::{
        ClearProjectTopicsError, ClearProjectTopicsUseCase,
    };
    use crate::tests::support::app_state_builder::TestAppStateBuilder;

    /* --------------------------------------------------
     * Mock ClearProjectTopicsUseCase
     * -------------------------------------------------- */

    #[derive(Clone)]
    struct MockClearProjectTopicsUseCase {
        result: Result<(), ClearProjectTopicsError>,
    }

    impl MockClearProjectTopicsUseCase {
        fn success() -> Self {
            Self { result: Ok(()) }
        }

        fn error(err: ClearProjectTopicsError) -> Self {
            Self { result: Err(err) }
        }
    }

    #[async_trait]
    impl ClearProjectTopicsUseCase for MockClearProjectTopicsUseCase {
        async fn execute(
            &self,
            _owner: UserId,
            _project_id: Uuid,
        ) -> Result<(), ClearProjectTopicsError> {
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
    async fn test_clear_project_topics_success_no_content() {
        let user_id = Uuid::new_v4();
        let project_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_clear_project_topics(MockClearProjectTopicsUseCase::success())
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(clear_project_topics_handler),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri(&format!("/api/projects/{}/topics/all", project_id))
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[actix_web::test]
    async fn test_clear_project_topics_project_not_found() {
        let user_id = Uuid::new_v4();
        let project_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_clear_project_topics(MockClearProjectTopicsUseCase::error(
                ClearProjectTopicsError::ProjectNotFound,
            ))
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(clear_project_topics_handler),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri(&format!("/api/projects/{}/topics/all", project_id))
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_clear_project_topics_repository_error_internal_error() {
        let user_id = Uuid::new_v4();
        let project_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_clear_project_topics(MockClearProjectTopicsUseCase::error(
                ClearProjectTopicsError::RepositoryError("db down".to_string()),
            ))
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(clear_project_topics_handler),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri(&format!("/api/projects/{}/topics/all", project_id))
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[actix_web::test]
    async fn test_clear_project_topics_unverified_user_forbidden() {
        let user_id = Uuid::new_v4();
        let project_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_clear_project_topics(MockClearProjectTopicsUseCase::success())
            .build();

        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(clear_project_topics_handler),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri(&format!("/api/projects/{}/topics/all", project_id))
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, false))))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }
}
