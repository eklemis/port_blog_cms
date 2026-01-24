use crate::auth::application::use_cases::refresh_token::{RefreshTokenError, RefreshTokenRequest};
use crate::AppState;
use actix_web::{post, web, HttpResponse, Responder};

use tracing::{error, info, warn};

/// **🔄 Refresh Access Token API Endpoint**
#[post("/api/auth/refresh")]
pub async fn refresh_token_handler(
    req: web::Json<RefreshTokenRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let use_case = &data.refresh_token_use_case;
    let request = req.into_inner();

    info!("Token refresh attempt");

    let result = use_case.execute(request).await;

    match result {
        Ok(response) => {
            info!("Token refreshed successfully");

            HttpResponse::Ok().json(serde_json::json!({
                "access_token": response.access_token,
                "refresh_token": response.refresh_token,
            }))
        }

        Err(RefreshTokenError::TokenExpired) => {
            warn!("Token refresh failed: Token expired");

            HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Refresh token has expired. Please login again."
            }))
        }

        Err(RefreshTokenError::TokenInvalid) | Err(RefreshTokenError::InvalidSignature) => {
            warn!("Token refresh failed: Invalid token");

            HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Invalid refresh token"
            }))
        }

        Err(RefreshTokenError::InvalidTokenType) => {
            warn!("Token refresh failed: Wrong token type");

            HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Invalid token type. Please use a refresh token."
            }))
        }

        Err(RefreshTokenError::TokenNotYetValid) => {
            warn!("Token refresh failed: Token not yet valid");

            HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Token is not yet valid"
            }))
        }

        Err(RefreshTokenError::TokenGenerationFailed(ref e)) => {
            error!(error = %e, "Token generation failed during refresh");

            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Internal server error"
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::application::use_cases::refresh_token::{
        IRefreshTokenUseCase, RefreshTokenError, RefreshTokenRequest, RefreshTokenResponse,
    };
    use crate::tests::support::app_state_builder::TestAppStateBuilder;
    use crate::tests::support::load_test_env;
    use actix_web::{test, App};
    use async_trait::async_trait;

    // ========================================================================
    // Mock Use Cases for Different Scenarios
    // ========================================================================

    #[derive(Clone)]
    struct MockRefreshTokenSuccess;

    #[async_trait]
    impl IRefreshTokenUseCase for MockRefreshTokenSuccess {
        async fn execute(
            &self,
            _request: RefreshTokenRequest,
        ) -> Result<RefreshTokenResponse, RefreshTokenError> {
            Ok(RefreshTokenResponse {
                access_token: std::env::var("TEST_ACCESS_TOKEN")
                    .unwrap_or_else(|_| "FAKE_TEST_ACCESS_TOKEN_DO_NOT_USE".to_string()),

                refresh_token: std::env::var("TEST_REFRESH_TOKEN")
                    .unwrap_or_else(|_| "FAKE_TEST_REFRESH_TOKEN_DO_NOT_USE".to_string()),
            })
        }
    }

    #[derive(Clone)]
    struct MockRefreshTokenExpired;

    #[async_trait]
    impl IRefreshTokenUseCase for MockRefreshTokenExpired {
        async fn execute(
            &self,
            _request: RefreshTokenRequest,
        ) -> Result<RefreshTokenResponse, RefreshTokenError> {
            Err(RefreshTokenError::TokenExpired)
        }
    }

    #[derive(Clone)]
    struct MockRefreshTokenInvalid;

    #[async_trait]
    impl IRefreshTokenUseCase for MockRefreshTokenInvalid {
        async fn execute(
            &self,
            _request: RefreshTokenRequest,
        ) -> Result<RefreshTokenResponse, RefreshTokenError> {
            Err(RefreshTokenError::TokenInvalid)
        }
    }

    #[derive(Clone)]
    struct MockRefreshTokenInvalidSignature;

    #[async_trait]
    impl IRefreshTokenUseCase for MockRefreshTokenInvalidSignature {
        async fn execute(
            &self,
            _request: RefreshTokenRequest,
        ) -> Result<RefreshTokenResponse, RefreshTokenError> {
            Err(RefreshTokenError::InvalidSignature)
        }
    }

    #[derive(Clone)]
    struct MockRefreshTokenInvalidType;

    #[async_trait]
    impl IRefreshTokenUseCase for MockRefreshTokenInvalidType {
        async fn execute(
            &self,
            _request: RefreshTokenRequest,
        ) -> Result<RefreshTokenResponse, RefreshTokenError> {
            Err(RefreshTokenError::InvalidTokenType)
        }
    }

    #[derive(Clone)]
    struct MockRefreshTokenNotYetValid;

    #[async_trait]
    impl IRefreshTokenUseCase for MockRefreshTokenNotYetValid {
        async fn execute(
            &self,
            _request: RefreshTokenRequest,
        ) -> Result<RefreshTokenResponse, RefreshTokenError> {
            Err(RefreshTokenError::TokenNotYetValid)
        }
    }

    #[derive(Clone)]
    struct MockRefreshTokenGenerationFailed;

    #[async_trait]
    impl IRefreshTokenUseCase for MockRefreshTokenGenerationFailed {
        async fn execute(
            &self,
            _request: RefreshTokenRequest,
        ) -> Result<RefreshTokenResponse, RefreshTokenError> {
            Err(RefreshTokenError::TokenGenerationFailed(
                "JWT signing failed".to_string(),
            ))
        }
    }

    // ========================================================================
    // Helper Functions
    // ========================================================================

    fn create_test_refresh_token_request_json() -> serde_json::Value {
        serde_json::json!({
            "refresh_token": std::env::var("TEST_REFRESH_TOKEN")
                .unwrap_or_else(|_| "FAKE_TEST_REFRESH_TOKEN_DO_NOT_USE".to_string())
        })
    }

    // ========================================================================
    // Tests
    // ========================================================================

    #[actix_web::test]
    async fn test_refresh_token_success() {
        load_test_env();
        let app_state = TestAppStateBuilder::default()
            .with_refresh_token(MockRefreshTokenSuccess)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(refresh_token_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/auth/refresh")
            .set_json(&create_test_refresh_token_request_json())
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert!(body["access_token"].is_string());
        assert!(body["refresh_token"].is_string());
        assert_eq!(body["access_token"].to_string().is_empty(), false);
        assert_eq!(body["refresh_token"].to_string().is_empty(), false);
    }

    #[actix_web::test]
    async fn test_refresh_token_expired() {
        let app_state = TestAppStateBuilder::default()
            .with_refresh_token(MockRefreshTokenExpired)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(refresh_token_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/auth/refresh")
            .set_json(&create_test_refresh_token_request_json())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 401);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(
            body["error"],
            "Refresh token has expired. Please login again."
        );
    }

    #[actix_web::test]
    async fn test_refresh_token_invalid() {
        let app_state = TestAppStateBuilder::default()
            .with_refresh_token(MockRefreshTokenInvalid)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(refresh_token_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/auth/refresh")
            .set_json(&create_test_refresh_token_request_json())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 401);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "Invalid refresh token");
    }

    #[actix_web::test]
    async fn test_refresh_token_invalid_signature() {
        let app_state = TestAppStateBuilder::default()
            .with_refresh_token(MockRefreshTokenInvalidSignature)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(refresh_token_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/auth/refresh")
            .set_json(&create_test_refresh_token_request_json())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 401);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "Invalid refresh token");
    }

    #[actix_web::test]
    async fn test_refresh_token_invalid_type() {
        let app_state = TestAppStateBuilder::default()
            .with_refresh_token(MockRefreshTokenInvalidType)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(refresh_token_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/auth/refresh")
            .set_json(&create_test_refresh_token_request_json())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(
            body["error"],
            "Invalid token type. Please use a refresh token."
        );
    }

    #[actix_web::test]
    async fn test_refresh_token_not_yet_valid() {
        let app_state = TestAppStateBuilder::default()
            .with_refresh_token(MockRefreshTokenNotYetValid)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(refresh_token_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/auth/refresh")
            .set_json(&create_test_refresh_token_request_json())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "Token is not yet valid");
    }

    #[actix_web::test]
    async fn test_refresh_token_generation_failed() {
        let app_state = TestAppStateBuilder::default()
            .with_refresh_token(MockRefreshTokenGenerationFailed)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(refresh_token_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/auth/refresh")
            .set_json(&create_test_refresh_token_request_json())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 500);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "Internal server error");
    }

    #[actix_web::test]
    async fn test_refresh_token_with_empty_token() {
        let app_state = TestAppStateBuilder::default()
            .with_refresh_token(MockRefreshTokenSuccess)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(refresh_token_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/auth/refresh")
            .set_json(&serde_json::json!({
                "refresh_token": ""
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    #[actix_web::test]
    async fn test_refresh_token_with_whitespace_only_token() {
        let app_state = TestAppStateBuilder::default()
            .with_refresh_token(MockRefreshTokenSuccess)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(refresh_token_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/auth/refresh")
            .set_json(&serde_json::json!({
                "refresh_token": "   "
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    #[actix_web::test]
    async fn test_refresh_token_with_leading_trailing_whitespace() {
        let app_state = TestAppStateBuilder::default()
            .with_refresh_token(MockRefreshTokenSuccess)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(refresh_token_handler),
        )
        .await;

        // Token should be trimmed
        let req = test::TestRequest::post()
            .uri("/api/auth/refresh")
            .set_json(&serde_json::json!({
                "refresh_token": "  eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.valid_token  "
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
    }

    #[actix_web::test]
    async fn test_refresh_token_with_very_long_token() {
        let app_state = TestAppStateBuilder::default()
            .with_refresh_token(MockRefreshTokenSuccess)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(refresh_token_handler),
        )
        .await;

        // Test with a very long JWT-like token
        let long_token = format!(
            "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.{}.signature",
            "a".repeat(1000)
        );

        let req = test::TestRequest::post()
            .uri("/api/auth/refresh")
            .set_json(&serde_json::json!({
                "refresh_token": long_token
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
    }

    #[actix_web::test]
    async fn test_refresh_token_with_malformed_jwt_structure() {
        let app_state = TestAppStateBuilder::default()
            .with_refresh_token(MockRefreshTokenSuccess)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(refresh_token_handler),
        )
        .await;

        // Various malformed JWT structures - all should be accepted by the handler
        // (validation happens in the use case)
        let malformed_tokens = vec![
            "not.a.jwt",
            "onlyonepart",
            "two.parts",
            "invalid_base64!@#$",
        ];

        for token in malformed_tokens {
            let req = test::TestRequest::post()
                .uri("/api/auth/refresh")
                .set_json(&serde_json::json!({
                    "refresh_token": token
                }))
                .to_request();

            let resp = test::call_service(&app, req).await;
            // Handler accepts it (validation happens in use case),
            // so we get 200 from our mock
            assert_eq!(resp.status(), 200, "Failed for token: {}", token);
        }
    }
}
