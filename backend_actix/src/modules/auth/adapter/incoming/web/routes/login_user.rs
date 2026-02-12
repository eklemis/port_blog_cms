use crate::api::schemas::{ErrorResponse, SuccessResponse};
use crate::auth::application::use_cases::login_user::LoginError;
use crate::auth::application::use_cases::login_user::LoginRequest;
use crate::shared::api::ApiResponse;
use crate::AppState;
use actix_web::{post, web, Responder};
use serde::Deserialize;
use serde::Serialize;
use tracing::{error, info, warn};

use utoipa::ToSchema;

/// Login request from client
#[derive(Deserialize, ToSchema)]
pub struct LoginRequestDto {
    /// Email address
    #[schema(example = "john@example.com")]
    pub email: String,

    /// Password
    #[schema(example = "SecurePass123!")]
    pub password: String,
}

#[derive(Serialize, ToSchema)]
pub struct LoginResponse {
    /// JWT access token (short-lived)
    #[schema(example = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...")]
    access_token: String,

    /// JWT refresh token (long-lived)
    #[schema(example = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...")]
    refresh_token: String,

    /// Authenticated user information
    user: LoginUserInfo,
}

#[derive(Serialize, ToSchema)]
pub struct LoginUserInfo {
    /// User ID (UUID)
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    id: String,

    /// Username
    #[schema(example = "johndoe")]
    username: String,

    /// Email address
    #[schema(example = "john@example.com")]
    email: String,

    /// Whether the user has verified their email
    #[schema(example = true)]
    is_verified: bool,
}

/// User login
///
/// Authenticates a user with email and password, returns JWT access and refresh tokens.
#[utoipa::path(
    post,
    path = "/api/auth/login",
    tag = "auth",
    request_body = LoginRequestDto,  // ✅ Use the DTO
    responses(
        (
            status = 200,
            description = "Login successful",
            body = inline(SuccessResponse<LoginResponse>),
            example = json!({
                "success": true,
                "data": {
                    "accessToken": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
                    "refreshToken": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
                    "user": {
                        "id": "123e4567-e89b-12d3-a456-426614174000",
                        "username": "johndoe",
                        "email": "john@example.com",
                        "isVerified": true
                    }
                }
            })
        ),
        (
            status = 401,
            description = "Invalid credentials",
            body = ErrorResponse,
            example = json!({
                "success": false,
                "error": {
                    "code": "INVALID_CREDENTIALS",
                    "message": "Invalid email or password"
                }
            })
        ),
        (
            status = 403,
            description = "Account has been deleted",
            body = ErrorResponse,
            example = json!({
                "success": false,
                "error": {
                    "code": "USER_DELETED",
                    "message": "This account has been deleted"
                }
            })
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
#[post("/api/auth/login")]
pub async fn login_user_handler(
    req: web::Json<LoginRequestDto>,
    data: web::Data<AppState>,
) -> impl Responder {
    let use_case = &data.login_user_use_case;
    let dto = req.into_inner();

    info!(email = %dto.email, "Login attempt");

    // ✅ Convert DTO to domain LoginRequest
    let request = match LoginRequest::new(dto.email, dto.password) {
        Ok(req) => req,
        Err(e) => {
            // Handle validation error if LoginRequest::new validates
            return ApiResponse::bad_request("VALIDATION_ERROR", &e.to_string());
        }
    };

    let result = use_case.execute(request).await;

    match result {
        Ok(response) => {
            info!(
                user_id = %response.user.id,
                email = %response.user.email,
                "User logged in successfully"
            );

            ApiResponse::success(LoginResponse {
                access_token: response.access_token,
                refresh_token: response.refresh_token,
                user: LoginUserInfo {
                    id: response.user.id.to_string(),
                    username: response.user.username,
                    email: response.user.email,
                    is_verified: response.user.is_verified,
                },
            })
        }

        Err(LoginError::InvalidCredentials) => {
            warn!("Login failed: Invalid credentials");
            ApiResponse::unauthorized("INVALID_CREDENTIALS", "Invalid email or password")
        }

        Err(LoginError::UserDeleted) => {
            warn!("Login failed: User deleted");
            ApiResponse::forbidden("USER_DELETED", "This account has been deleted")
        }

        Err(LoginError::PasswordVerificationFailed(ref e)) => {
            error!(error = %e, "Password verification failed");
            ApiResponse::internal_error()
        }

        Err(LoginError::TokenGenerationFailed(ref e)) => {
            error!(error = %e, "Token generation failed");
            ApiResponse::internal_error()
        }

        Err(LoginError::QueryError(ref e)) => {
            error!(error = %e, "Database query failed");
            ApiResponse::internal_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::application::use_cases::login_user::{
        ILoginUserUseCase, LoginError, LoginRequest, LoginUserResponse, UserInfo,
    };
    use crate::tests::support::app_state_builder::TestAppStateBuilder;
    use crate::tests::support::load_test_env;
    use actix_web::{test, App};
    use async_trait::async_trait;
    use uuid::Uuid;

    // ========================================================================
    // Mock Response Types
    // ========================================================================

    fn create_mock_login_response() -> LoginUserResponse {
        LoginUserResponse {
            access_token: "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.access".to_string(),
            refresh_token: "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.refresh".to_string(),
            user: UserInfo {
                id: Uuid::new_v4(),
                username: "testuser".to_string(),
                email: "test@example.com".to_string(),
                is_verified: true,
            },
        }
    }

    // ========================================================================
    // Mock Use Cases for Different Scenarios
    // ========================================================================

    #[derive(Clone)]
    struct MockLoginUserSuccess;

    #[async_trait]
    impl ILoginUserUseCase for MockLoginUserSuccess {
        async fn execute(&self, _request: LoginRequest) -> Result<LoginUserResponse, LoginError> {
            Ok(create_mock_login_response())
        }
    }

    #[derive(Clone)]
    struct MockLoginUserSuccessUnverified;

    #[async_trait]
    impl ILoginUserUseCase for MockLoginUserSuccessUnverified {
        async fn execute(&self, _request: LoginRequest) -> Result<LoginUserResponse, LoginError> {
            Ok(LoginUserResponse {
                access_token: std::env::var("TEST_ACCESS_TOKEN")
                    .unwrap_or_else(|_| "FAKE_TEST_ACCESS_TOKEN_DO_NOT_USE".to_string()),
                refresh_token: std::env::var("TEST_REFRESH_TOKEN")
                    .unwrap_or_else(|_| "FAKE_TEST_REFRESH_TOKEN_DO_NOT_USE".to_string()),
                user: UserInfo {
                    id: Uuid::new_v4(),
                    username: "unverified".to_string(),
                    email: "unverified@example.com".to_string(),
                    is_verified: false,
                },
            })
        }
    }

    #[derive(Clone)]
    struct MockLoginUserInvalidCredentials;

    #[async_trait]
    impl ILoginUserUseCase for MockLoginUserInvalidCredentials {
        async fn execute(&self, _request: LoginRequest) -> Result<LoginUserResponse, LoginError> {
            Err(LoginError::InvalidCredentials)
        }
    }

    #[derive(Clone)]
    struct MockLoginUserDeleted;

    #[async_trait]
    impl ILoginUserUseCase for MockLoginUserDeleted {
        async fn execute(&self, _request: LoginRequest) -> Result<LoginUserResponse, LoginError> {
            Err(LoginError::UserDeleted)
        }
    }

    #[derive(Clone)]
    struct MockLoginPasswordVerificationFailed;

    #[async_trait]
    impl ILoginUserUseCase for MockLoginPasswordVerificationFailed {
        async fn execute(&self, _request: LoginRequest) -> Result<LoginUserResponse, LoginError> {
            Err(LoginError::PasswordVerificationFailed(
                "Argon2 verification failed".to_string(),
            ))
        }
    }

    #[derive(Clone)]
    struct MockLoginTokenGenerationFailed;

    #[async_trait]
    impl ILoginUserUseCase for MockLoginTokenGenerationFailed {
        async fn execute(&self, _request: LoginRequest) -> Result<LoginUserResponse, LoginError> {
            Err(LoginError::TokenGenerationFailed(
                "JWT signing failed".to_string(),
            ))
        }
    }

    #[derive(Clone)]
    struct MockLoginQueryError;

    #[async_trait]
    impl ILoginUserUseCase for MockLoginQueryError {
        async fn execute(&self, _request: LoginRequest) -> Result<LoginUserResponse, LoginError> {
            Err(LoginError::QueryError(
                "Connection pool exhausted".to_string(),
            ))
        }
    }

    // ========================================================================
    // Helper Functions
    // ========================================================================

    fn create_test_login_request_json() -> serde_json::Value {
        serde_json::json!({
            "email": "test@example.com",
            "password": "SecurePass123!"
        })
    }

    // ========================================================================
    // Tests
    // ========================================================================

    #[actix_web::test]
    async fn test_login_user_success() {
        load_test_env();
        let app_state = TestAppStateBuilder::default()
            .with_login_user(MockLoginUserSuccess)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(login_user_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(&create_test_login_request_json())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert!(body["data"]["access_token"].is_string());
        assert!(body["data"]["refresh_token"].is_string());
        assert!(body["data"]["user"]["id"].is_string());
        assert_eq!(body["data"]["user"]["username"], "testuser");
        assert_eq!(body["data"]["user"]["email"], "test@example.com");
        assert_eq!(body["data"]["user"]["is_verified"], true);
        assert!(body.get("error").is_none());
    }

    #[actix_web::test]
    async fn test_login_user_success_with_unverified_user() {
        let app_state = TestAppStateBuilder::default()
            .with_login_user(MockLoginUserSuccessUnverified)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(login_user_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(&create_test_login_request_json())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert_eq!(body["data"]["user"]["is_verified"], false);
        assert_eq!(body["data"]["user"]["email"], "unverified@example.com");
        assert!(body.get("error").is_none());
    }

    #[actix_web::test]
    async fn test_login_user_invalid_credentials() {
        let app_state = TestAppStateBuilder::default()
            .with_login_user(MockLoginUserInvalidCredentials)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(login_user_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(&create_test_login_request_json())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 401);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "INVALID_CREDENTIALS");
        assert_eq!(body["error"]["message"], "Invalid email or password");
        assert!(body.get("data").is_none());
    }

    #[actix_web::test]
    async fn test_login_user_deleted() {
        let app_state = TestAppStateBuilder::default()
            .with_login_user(MockLoginUserDeleted)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(login_user_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(&create_test_login_request_json())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 403);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "USER_DELETED");
        assert_eq!(body["error"]["message"], "This account has been deleted");
        assert!(body.get("data").is_none());
    }

    #[actix_web::test]
    async fn test_login_password_verification_failed() {
        let app_state = TestAppStateBuilder::default()
            .with_login_user(MockLoginPasswordVerificationFailed)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(login_user_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(&create_test_login_request_json())
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
    async fn test_login_token_generation_failed() {
        let app_state = TestAppStateBuilder::default()
            .with_login_user(MockLoginTokenGenerationFailed)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(login_user_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(&create_test_login_request_json())
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
    async fn test_login_query_error() {
        let app_state = TestAppStateBuilder::default()
            .with_login_user(MockLoginQueryError)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(login_user_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(&create_test_login_request_json())
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
    async fn test_login_with_different_email_formats() {
        let app_state = TestAppStateBuilder::default()
            .with_login_user(MockLoginUserSuccess)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(login_user_handler)).await;

        let test_cases = vec![
            "user@example.com",
            "user.name@example.com",
            "user+tag@example.co.uk",
            "user_name@subdomain.example.com",
        ];

        for email in test_cases {
            let req = test::TestRequest::post()
                .uri("/api/auth/login")
                .set_json(&serde_json::json!({
                    "email": email,
                    "password": "password123"
                }))
                .to_request();

            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200, "Failed for email: {}", email);

            let body: serde_json::Value = test::read_body_json(resp).await;
            assert_eq!(body["success"], true);
        }
    }

    #[actix_web::test]
    async fn test_login_with_special_characters_in_password() {
        let app_state = TestAppStateBuilder::default()
            .with_login_user(MockLoginUserSuccess)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(login_user_handler)).await;

        let special_passwords = vec![
            "P@ssw0rd!",
            "Test#123$",
            "Complex&Pass%2024",
            "Spëcial€Chàrs",
        ];

        for password in special_passwords {
            let req = test::TestRequest::post()
                .uri("/api/auth/login")
                .set_json(&serde_json::json!({
                    "email": "test@example.com",
                    "password": password
                }))
                .to_request();

            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 200, "Failed for password: {}", password);

            let body: serde_json::Value = test::read_body_json(resp).await;
            assert_eq!(body["success"], true);
        }
    }

    #[actix_web::test]
    async fn test_login_with_uppercase_email() {
        let app_state = TestAppStateBuilder::default()
            .with_login_user(MockLoginUserSuccess)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(login_user_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(&serde_json::json!({
                "email": "TEST@EXAMPLE.COM",
                "password": "password123"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
    }

    #[actix_web::test]
    async fn test_login_with_whitespace_in_email() {
        let app_state = TestAppStateBuilder::default()
            .with_login_user(MockLoginUserSuccess)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(login_user_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(&serde_json::json!({
                "email": "  test@example.com  ",
                "password": "password123"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
    }

    #[actix_web::test]
    async fn test_login_with_invalid_email_format() {
        let app_state = TestAppStateBuilder::default()
            .with_login_user(MockLoginUserSuccess)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(crate::shared::api::custom_json_config())
                .service(login_user_handler),
        )
        .await;

        let invalid_emails = vec!["notanemail", "missing@", "@nodomain.com", ""];

        for email in invalid_emails {
            let req = test::TestRequest::post()
                .uri("/api/auth/login")
                .set_json(&serde_json::json!({
                    "email": email,
                    "password": "password123"
                }))
                .to_request();

            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 400, "Should reject invalid email: {}", email);

            let body: serde_json::Value = test::read_body_json(resp).await;
            assert_eq!(body["success"], false);
            assert_eq!(body["error"]["code"], "VALIDATION_ERROR");
            assert!(body.get("data").is_none());
        }
    }

    #[actix_web::test]
    async fn test_login_with_empty_password() {
        let app_state = TestAppStateBuilder::default()
            .with_login_user(MockLoginUserSuccess)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(crate::shared::api::custom_json_config())
                .service(login_user_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(&serde_json::json!({
                "email": "test@example.com",
                "password": ""
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
    async fn test_login_with_whitespace_only_password() {
        let app_state = TestAppStateBuilder::default()
            .with_login_user(MockLoginUserSuccess)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(crate::shared::api::custom_json_config())
                .service(login_user_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(&serde_json::json!({
                "email": "test@example.com",
                "password": "   "
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "VALIDATION_ERROR");
        assert!(body.get("data").is_none());
    }
}
