use crate::api::schemas::{ErrorResponse, SuccessResponse};
use crate::auth::application::use_cases::refresh_token::{RefreshTokenError, RefreshTokenRequest};
use crate::shared::api::ApiResponse;
use crate::AppState;
use actix_web::{post, web, Responder};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct RefreshTokenResponseBody {
    /// New JWT access token
    #[schema(example = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...")]
    access_token: String,

    /// New JWT refresh token
    #[schema(example = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...")]
    refresh_token: String,
}

#[derive(Deserialize, ToSchema)]
pub struct RefreshTokenRequestDto {
    /// Refresh token to exchange for new tokens
    #[schema(example = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...")]
    pub refresh_token: String,
}

/// Refresh access token
///
/// Exchanges a valid refresh token for a new access token and refresh token pair.
/// The old refresh token is revoked and cannot be reused.
#[utoipa::path(
    post,
    path = "/api/auth/refresh",
    tag = "auth",
    request_body = RefreshTokenRequestDto,  // Or RefreshTokenRequest if it has Deserialize + ToSchema
    responses(
        (
            status = 200,
            description = "Token refreshed successfully",
            body = inline(SuccessResponse<RefreshTokenResponseBody>),
            example = json!({
                "success": true,
                "data": {
                    "accessToken": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
                    "refreshToken": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
                }
            })
        ),
        (
            status = 400,
            description = "Bad request - invalid token type or token not yet valid",
            body = ErrorResponse,
            examples(
                ("Invalid token type" = (value = json!({
                    "success": false,
                    "error": {
                        "code": "INVALID_TOKEN_TYPE",
                        "message": "Invalid token type. Please use a refresh token."
                    }
                }))),
                ("Token not yet valid" = (value = json!({
                    "success": false,
                    "error": {
                        "code": "TOKEN_NOT_YET_VALID",
                        "message": "Token is not yet valid"
                    }
                })))
            )
        ),
        (
            status = 401,
            description = "Unauthorized - token expired or invalid",
            body = ErrorResponse,
            examples(
                ("Token expired" = (value = json!({
                    "success": false,
                    "error": {
                        "code": "TOKEN_EXPIRED",
                        "message": "Refresh token has expired. Please login again."
                    }
                }))),
                ("Token invalid" = (value = json!({
                    "success": false,
                    "error": {
                        "code": "TOKEN_INVALID",
                        "message": "Invalid refresh token"
                    }
                })))
            )
        ),
        (
            status = 500,
            description = "Internal server error",
            body = ErrorResponse,
            example = json!({
                "success": false,
                "error": {
                    "code": "INTERNAL_ERROR",
                    "message": "An unexpected error occurred"
                }
            })
        ),
    )
)]
#[post("/api/auth/refresh")]
pub async fn refresh_token_handler(
    req: web::Json<RefreshTokenRequestDto>,
    data: web::Data<AppState>,
) -> impl Responder {
    let use_case = &data.refresh_token_use_case;
    let dto = req.into_inner();

    info!("Token refresh attempt");
    let request = match RefreshTokenRequest::new(dto.refresh_token) {
        Ok(req) => req,
        Err(e) => {
            return ApiResponse::bad_request("VALIDATION_ERROR", &e.to_string());
        }
    };

    let result = use_case.execute(request).await;

    match result {
        Ok(response) => {
            info!("Token refreshed successfully");
            ApiResponse::success(RefreshTokenResponseBody {
                access_token: response.access_token,
                refresh_token: response.refresh_token,
            })
        }

        Err(RefreshTokenError::TokenExpired) => {
            warn!("Token refresh failed: Token expired");
            ApiResponse::unauthorized(
                "TOKEN_EXPIRED",
                "Refresh token has expired. Please login again.",
            )
        }

        Err(RefreshTokenError::TokenInvalid) | Err(RefreshTokenError::InvalidSignature) => {
            warn!("Token refresh failed: Invalid token");
            ApiResponse::unauthorized("TOKEN_INVALID", "Invalid refresh token")
        }

        Err(RefreshTokenError::InvalidTokenType) => {
            warn!("Token refresh failed: Wrong token type");
            ApiResponse::bad_request(
                "INVALID_TOKEN_TYPE",
                "Invalid token type. Please use a refresh token.",
            )
        }

        Err(RefreshTokenError::TokenNotYetValid) => {
            warn!("Token refresh failed: Token not yet valid");
            ApiResponse::bad_request("TOKEN_NOT_YET_VALID", "Token is not yet valid")
        }

        Err(RefreshTokenError::TokenGenerationFailed(ref e)) => {
            error!(error = %e, "Token generation failed during refresh");
            ApiResponse::internal_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::application::use_cases::refresh_token::{
        IRefreshTokenUseCase, RefreshTokenError, RefreshTokenRequest, RefreshTokenResponse,
    };
    use crate::shared::api::custom_json_config;
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
        assert_eq!(body["success"], true);
        assert!(body["data"]["access_token"].is_string());
        assert!(body["data"]["refresh_token"].is_string());
        assert!(!body["data"]["access_token"].as_str().unwrap().is_empty());
        assert!(!body["data"]["refresh_token"].as_str().unwrap().is_empty());
        assert!(body.get("error").is_none());
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
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "TOKEN_EXPIRED");
        assert_eq!(
            body["error"]["message"],
            "Refresh token has expired. Please login again."
        );
        assert!(body.get("data").is_none());
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
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "TOKEN_INVALID");
        assert_eq!(body["error"]["message"], "Invalid refresh token");
        assert!(body.get("data").is_none());
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
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "TOKEN_INVALID");
        assert_eq!(body["error"]["message"], "Invalid refresh token");
        assert!(body.get("data").is_none());
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
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "INVALID_TOKEN_TYPE");
        assert_eq!(
            body["error"]["message"],
            "Invalid token type. Please use a refresh token."
        );
        assert!(body.get("data").is_none());
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
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "TOKEN_NOT_YET_VALID");
        assert_eq!(body["error"]["message"], "Token is not yet valid");
        assert!(body.get("data").is_none());
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
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "INTERNAL_ERROR");
        assert_eq!(body["error"]["message"], "An unexpected error occurred");
        assert!(body.get("data").is_none());
    }

    #[actix_web::test]
    async fn test_refresh_token_with_empty_token() {
        let app_state = TestAppStateBuilder::default()
            .with_refresh_token(MockRefreshTokenSuccess)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(custom_json_config())
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

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "VALIDATION_ERROR");
        assert!(body.get("data").is_none());
    }

    #[actix_web::test]
    async fn test_refresh_token_with_whitespace_only_token() {
        let app_state = TestAppStateBuilder::default()
            .with_refresh_token(MockRefreshTokenSuccess)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(custom_json_config())
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

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "VALIDATION_ERROR");
        assert!(body.get("data").is_none());
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

        let req = test::TestRequest::post()
            .uri("/api/auth/refresh")
            .set_json(&serde_json::json!({
                "refresh_token": "  eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.valid_token  "
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert!(body.get("error").is_none());
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

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert!(body.get("error").is_none());
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
            assert_eq!(resp.status(), 200, "Failed for token: {}", token);

            let body: serde_json::Value = test::read_body_json(resp).await;
            assert_eq!(body["success"], true);
            assert!(body.get("error").is_none());
        }
    }
}
