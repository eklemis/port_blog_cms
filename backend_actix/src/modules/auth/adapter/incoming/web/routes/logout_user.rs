use crate::modules::auth::application::use_cases::logout_user::{LogoutError, LogoutRequest};
use crate::AppState;
use actix_web::{post, web, HttpResponse, Responder};

use tracing::{error, info};

/// **🚪 Logout User API Endpoint**
#[post("/api/auth/logout")]
pub async fn logout_user_handler(
    req: web::Json<LogoutRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let use_case = &data.logout_user_use_case;
    let request = req.into_inner();

    info!("User logout attempt");

    let result = use_case.execute(request).await;

    match result {
        Ok(response) => {
            info!("User logged out successfully");

            HttpResponse::Ok().json(serde_json::json!({
                "message": response.message
            }))
        }

        Err(LogoutError::TokenRevocationFailed(ref e)) => {
            error!(error = %e, "Token revocation failed during logout");

            // Still return success to user - they're logged out on client side
            // Log the error for investigation
            HttpResponse::Ok().json(serde_json::json!({
                "message": "Logged out successfully"
            }))
        }

        Err(LogoutError::DatabaseError(ref e)) => {
            error!(error = %e, "Database error during logout");

            // Still return success to user
            HttpResponse::Ok().json(serde_json::json!({
                "message": "Logged out successfully"
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::application::use_cases::logout_user::{
        ILogoutUseCase, LogoutError, LogoutRequest, LogoutResponse,
    };
    use crate::tests::support::app_state_builder::TestAppStateBuilder;
    use actix_web::{test, App};
    use async_trait::async_trait;

    // ========================================================================
    // Mock Use Cases for Different Scenarios
    // ========================================================================

    #[derive(Clone)]
    struct MockLogoutSuccess;

    #[async_trait]
    impl ILogoutUseCase for MockLogoutSuccess {
        async fn execute(&self, _request: LogoutRequest) -> Result<LogoutResponse, LogoutError> {
            Ok(LogoutResponse {
                message: "Logged out successfully".to_string(),
            })
        }
    }

    #[derive(Clone)]
    struct MockLogoutTokenRevocationFailed;

    #[async_trait]
    impl ILogoutUseCase for MockLogoutTokenRevocationFailed {
        async fn execute(&self, _request: LogoutRequest) -> Result<LogoutResponse, LogoutError> {
            Err(LogoutError::TokenRevocationFailed(
                "Failed to blacklist token".to_string(),
            ))
        }
    }

    #[derive(Clone)]
    struct MockLogoutDatabaseError;

    #[async_trait]
    impl ILogoutUseCase for MockLogoutDatabaseError {
        async fn execute(&self, _request: LogoutRequest) -> Result<LogoutResponse, LogoutError> {
            Err(LogoutError::DatabaseError(
                "Redis connection failed".to_string(),
            ))
        }
    }

    // ========================================================================
    // Tests
    // ========================================================================

    #[actix_web::test]
    async fn test_logout_success_with_refresh_token() {
        let app_state = TestAppStateBuilder::default()
            .with_logout_user(MockLogoutSuccess)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(logout_user_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/logout")
            .set_json(&serde_json::json!({
                "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.valid_token"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["message"], "Logged out successfully");
    }

    #[actix_web::test]
    async fn test_logout_success_without_refresh_token() {
        let app_state = TestAppStateBuilder::default()
            .with_logout_user(MockLogoutSuccess)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(logout_user_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/logout")
            .set_json(&serde_json::json!({}))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["message"], "Logged out successfully");
    }

    #[actix_web::test]
    async fn test_logout_success_with_null_refresh_token() {
        let app_state = TestAppStateBuilder::default()
            .with_logout_user(MockLogoutSuccess)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(logout_user_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/logout")
            .set_json(&serde_json::json!({
                "refresh_token": null
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["message"], "Logged out successfully");
    }

    #[actix_web::test]
    async fn test_logout_token_revocation_failed_still_returns_success() {
        let app_state = TestAppStateBuilder::default()
            .with_logout_user(MockLogoutTokenRevocationFailed)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(logout_user_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/logout")
            .set_json(&serde_json::json!({
                "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.valid_token"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        // Handler returns 200 even on token revocation failure (graceful degradation)
        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["message"], "Logged out successfully");
    }

    #[actix_web::test]
    async fn test_logout_database_error_still_returns_success() {
        let app_state = TestAppStateBuilder::default()
            .with_logout_user(MockLogoutDatabaseError)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(logout_user_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/logout")
            .set_json(&serde_json::json!({
                "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.valid_token"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        // Handler returns 200 even on database error (graceful degradation)
        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["message"], "Logged out successfully");
    }

    #[actix_web::test]
    async fn test_logout_with_empty_string_refresh_token() {
        let app_state = TestAppStateBuilder::default()
            .with_logout_user(MockLogoutSuccess)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(logout_user_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/logout")
            .set_json(&serde_json::json!({
                "refresh_token": ""
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["message"], "Logged out successfully");
    }

    #[actix_web::test]
    async fn test_logout_with_whitespace_only_refresh_token() {
        let app_state = TestAppStateBuilder::default()
            .with_logout_user(MockLogoutSuccess)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(logout_user_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/logout")
            .set_json(&serde_json::json!({
                "refresh_token": "   "
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["message"], "Logged out successfully");
    }

    #[actix_web::test]
    async fn test_logout_with_leading_trailing_whitespace_in_token() {
        let app_state = TestAppStateBuilder::default()
            .with_logout_user(MockLogoutSuccess)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(logout_user_handler)).await;

        // Token should be trimmed
        let req = test::TestRequest::post()
            .uri("/api/auth/logout")
            .set_json(&serde_json::json!({
                "refresh_token": "  eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.valid_token  "
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["message"], "Logged out successfully");
    }

    #[actix_web::test]
    async fn test_logout_with_very_long_token() {
        let app_state = TestAppStateBuilder::default()
            .with_logout_user(MockLogoutSuccess)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(logout_user_handler)).await;

        // Test with a very long JWT-like token
        let long_token = format!(
            "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.{}.signature",
            "a".repeat(1000)
        );

        let req = test::TestRequest::post()
            .uri("/api/auth/logout")
            .set_json(&serde_json::json!({
                "refresh_token": long_token
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["message"], "Logged out successfully");
    }

    #[actix_web::test]
    async fn test_logout_idempotency() {
        let app_state = TestAppStateBuilder::default()
            .with_logout_user(MockLogoutSuccess)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(logout_user_handler)).await;

        let token = "THIS_IS_JUST_TESTING_TOKEN_WITH_NO_HARM.valid_token";

        // First logout
        let req = test::TestRequest::post()
            .uri("/api/auth/logout")
            .set_json(&serde_json::json!({
                "refresh_token": token
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        // Second logout with same token (should still succeed - idempotent)
        let req = test::TestRequest::post()
            .uri("/api/auth/logout")
            .set_json(&serde_json::json!({
                "refresh_token": token
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
    }

    #[actix_web::test]
    async fn test_logout_with_malformed_token() {
        let app_state = TestAppStateBuilder::default()
            .with_logout_user(MockLogoutSuccess)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(logout_user_handler)).await;

        // Malformed tokens should still result in successful logout
        let malformed_tokens = vec![
            "not.a.jwt",
            "onlyonepart",
            "two.parts",
            "invalid_base64!@#$",
        ];

        for token in malformed_tokens {
            let req = test::TestRequest::post()
                .uri("/api/auth/logout")
                .set_json(&serde_json::json!({
                    "refresh_token": token
                }))
                .to_request();

            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200, "Failed for token: {}", token);

            let body: serde_json::Value = test::read_body_json(resp).await;
            assert_eq!(body["message"], "Logged out successfully");
        }
    }
}
