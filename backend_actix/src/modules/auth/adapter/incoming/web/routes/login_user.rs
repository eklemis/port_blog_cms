use crate::auth::application::use_cases::login_user::LoginError;
use crate::auth::application::use_cases::login_user::LoginRequest;

use crate::AppState;
use actix_web::{post, web, HttpResponse, Responder};

use tracing::{error, info, warn};

/// **🔐 Login User API Endpoint**
#[post("/api/auth/login")]
pub async fn login_user_handler(
    req: web::Json<LoginRequest>, // ✅ Directly deserialize validated request!
    data: web::Data<AppState>,
) -> impl Responder {
    let use_case = &data.login_user_use_case;
    let request = req.into_inner();

    info!(email = %request.email(), "Login attempt");

    let result = use_case.execute(request).await;

    match result {
        Ok(response) => {
            info!(
                user_id = %response.user.id,
                email = %response.user.email,
                "User logged in successfully"
            );

            HttpResponse::Ok().json(serde_json::json!({
                "access_token": response.access_token,
                "refresh_token": response.refresh_token,
                "user": {
                    "id": response.user.id,
                    "username": response.user.username,
                    "email": response.user.email,
                    "is_verified": response.user.is_verified,
                }
            }))
        }

        Err(LoginError::InvalidCredentials) => {
            warn!("Login failed: Invalid credentials");
            HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Invalid email or password"
            }))
        }

        Err(LoginError::UserDeleted) => {
            warn!("Login failed: User deleted");
            HttpResponse::Forbidden().json(serde_json::json!({
                "error": "This account has been deleted"
            }))
        }

        Err(LoginError::PasswordVerificationFailed(ref e)) => {
            error!(error = %e, "Password verification failed");
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Internal server error"
            }))
        }

        Err(LoginError::TokenGenerationFailed(ref e)) => {
            error!(error = %e, "Token generation failed");
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Internal server error"
            }))
        }

        Err(LoginError::QueryError(ref e)) => {
            error!(error = %e, "Database query failed");
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Internal server error"
            }))
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
        assert!(body["access_token"].is_string());
        assert!(body["refresh_token"].is_string());
        assert!(body["user"]["id"].is_string());
        assert_eq!(body["user"]["username"], "testuser");
        assert_eq!(body["user"]["email"], "test@example.com");
        assert_eq!(body["user"]["is_verified"], true);
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
        assert_eq!(body["user"]["is_verified"], false);
        assert_eq!(body["user"]["email"], "unverified@example.com");
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
        assert_eq!(body["error"], "Invalid email or password");
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
        assert_eq!(body["error"], "This account has been deleted");
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
        assert_eq!(body["error"], "Internal server error");
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
        assert_eq!(body["error"], "Internal server error");
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
        assert_eq!(body["error"], "Internal server error");
    }

    #[actix_web::test]
    async fn test_login_with_different_email_formats() {
        let app_state = TestAppStateBuilder::default()
            .with_login_user(MockLoginUserSuccess)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(login_user_handler)).await;

        // Test with different valid email formats
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
        }
    }

    #[actix_web::test]
    async fn test_login_with_uppercase_email() {
        let app_state = TestAppStateBuilder::default()
            .with_login_user(MockLoginUserSuccess)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(login_user_handler)).await;

        // Email should be normalized to lowercase
        let req = test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(&serde_json::json!({
                "email": "TEST@EXAMPLE.COM",
                "password": "password123"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
    }

    #[actix_web::test]
    async fn test_login_with_whitespace_in_email() {
        let app_state = TestAppStateBuilder::default()
            .with_login_user(MockLoginUserSuccess)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(login_user_handler)).await;

        // Email should be trimmed
        let req = test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(&serde_json::json!({
                "email": "  test@example.com  ",
                "password": "password123"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
    }

    #[actix_web::test]
    async fn test_login_with_invalid_email_format() {
        let app_state = TestAppStateBuilder::default()
            .with_login_user(MockLoginUserSuccess)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(login_user_handler)).await;

        // Invalid email formats should fail validation
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
        }
    }

    #[actix_web::test]
    async fn test_login_with_empty_password() {
        let app_state = TestAppStateBuilder::default()
            .with_login_user(MockLoginUserSuccess)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(login_user_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(&serde_json::json!({
                "email": "test@example.com",
                "password": ""
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    #[actix_web::test]
    async fn test_login_with_whitespace_only_password() {
        let app_state = TestAppStateBuilder::default()
            .with_login_user(MockLoginUserSuccess)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(login_user_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(&serde_json::json!({
                "email": "test@example.com",
                "password": "   "
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }
}
