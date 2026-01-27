use crate::auth::adapter::incoming::web::extractors::auth::AuthenticatedUser;
use crate::auth::application::use_cases::soft_delete_user::{
    SoftDeleteUserError, SoftDeleteUserRequest,
};
use crate::shared::api::ApiResponse;
use crate::AppState;
use actix_web::{delete, web, Responder};
use tracing::error;

#[delete("/api/users/me")]
pub async fn soft_delete_user_handler(
    user: AuthenticatedUser,
    data: web::Data<AppState>,
) -> impl Responder {
    let request = SoftDeleteUserRequest::new(user.user_id);

    match data.soft_delete_user_use_case.execute(request).await {
        Ok(_) => ApiResponse::no_content(),

        Err(SoftDeleteUserError::Unauthorized) => ApiResponse::unauthorized(
            "USER_UNAUTHORIZED",
            "You are not authorized to delete this account",
        ),

        Err(SoftDeleteUserError::DatabaseError(e)) => {
            error!("Database error soft deleting user: {}", e);

            if e.contains("User not found") {
                ApiResponse::not_found("USER_NOT_FOUND", "User not found")
            } else {
                ApiResponse::internal_error()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use actix_web::{test, App};
    use async_trait::async_trait;

    use crate::auth::application::ports::outgoing::token_provider::TokenProvider;
    use crate::auth::application::use_cases::soft_delete_user::{
        ISoftDeleteUserUseCase, SoftDeleteUserError, SoftDeleteUserRequest,
    };
    use crate::tests::support::app_state_builder::TestAppStateBuilder;
    use crate::tests::support::auth_helper::test_helpers::create_test_jwt_service;

    // ==========================================================
    // Mocks
    // ==========================================================

    #[derive(Clone)]
    struct MockSoftDeleteUserSuccess;

    #[async_trait]
    impl ISoftDeleteUserUseCase for MockSoftDeleteUserSuccess {
        async fn execute(
            &self,
            _request: SoftDeleteUserRequest,
        ) -> Result<(), SoftDeleteUserError> {
            Ok(())
        }
    }

    #[derive(Clone)]
    struct MockSoftDeleteUserUnauthorized;

    #[async_trait]
    impl ISoftDeleteUserUseCase for MockSoftDeleteUserUnauthorized {
        async fn execute(
            &self,
            _request: SoftDeleteUserRequest,
        ) -> Result<(), SoftDeleteUserError> {
            Err(SoftDeleteUserError::Unauthorized)
        }
    }

    #[derive(Clone)]
    struct MockSoftDeleteUserDatabaseError;

    #[async_trait]
    impl ISoftDeleteUserUseCase for MockSoftDeleteUserDatabaseError {
        async fn execute(
            &self,
            _request: SoftDeleteUserRequest,
        ) -> Result<(), SoftDeleteUserError> {
            Err(SoftDeleteUserError::DatabaseError(
                "Failed to soft delete user".to_string(),
            ))
        }
    }

    // ==========================================================
    // Tests
    // ==========================================================

    #[actix_web::test]
    async fn test_soft_delete_user_success() {
        let app_state = TestAppStateBuilder::default()
            .with_soft_delete_user(MockSoftDeleteUserSuccess)
            .build();

        let jwt_service = create_test_jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service.clone());

        let token = jwt_service
            .generate_access_token(uuid::Uuid::new_v4(), true)
            .unwrap();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(actix_web::web::Data::new(token_provider))
                .service(soft_delete_user_handler),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri("/api/users/me")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), actix_web::http::StatusCode::NO_CONTENT);
    }

    #[actix_web::test]
    async fn test_soft_delete_user_unauthorized_from_use_case() {
        let app_state = TestAppStateBuilder::default()
            .with_soft_delete_user(MockSoftDeleteUserUnauthorized)
            .build();

        let jwt_service = create_test_jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service.clone());

        let token = jwt_service
            .generate_access_token(uuid::Uuid::new_v4(), true)
            .unwrap();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(actix_web::web::Data::new(token_provider))
                .service(soft_delete_user_handler),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri("/api/users/me")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), actix_web::http::StatusCode::UNAUTHORIZED);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "USER_UNAUTHORIZED");
        assert!(body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("not authorized"));
        assert!(body.get("data").is_none());
    }

    #[actix_web::test]
    async fn test_soft_delete_user_database_error() {
        let app_state = TestAppStateBuilder::default()
            .with_soft_delete_user(MockSoftDeleteUserDatabaseError)
            .build();

        let jwt_service = create_test_jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service.clone());

        let token = jwt_service
            .generate_access_token(uuid::Uuid::new_v4(), true)
            .unwrap();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(actix_web::web::Data::new(token_provider))
                .service(soft_delete_user_handler),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri("/api/users/me")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(
            resp.status(),
            actix_web::http::StatusCode::INTERNAL_SERVER_ERROR
        );

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "INTERNAL_ERROR");
        assert_eq!(body["error"]["message"], "An unexpected error occurred");
        assert!(body.get("data").is_none());
    }
}
