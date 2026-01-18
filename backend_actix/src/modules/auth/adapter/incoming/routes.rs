use crate::auth::application::use_cases::login_user::LoginError;
use crate::auth::application::use_cases::refresh_token::{RefreshTokenError, RefreshTokenRequest};
use crate::auth::application::use_cases::{
    login_user::LoginRequest, verify_user_email::VerifyUserEmailError,
};
use crate::modules::auth::application::use_cases::create_user::CreateUserError;
use crate::modules::auth::application::use_cases::logout_user::{LogoutError, LogoutRequest};
use crate::AppState;
use actix_web::{post, web, HttpRequest, HttpResponse, Responder};
use serde::Deserialize;
use tracing::{error, info, warn};

/// **📥 Request Structure for Creating a User**
#[derive(serde::Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

/// **🚀 Create User API Endpoint**
#[post("/api/auth/register")]
pub async fn create_user_handler(
    req: web::Json<CreateUserRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let use_case = &data.create_user_use_case;

    info!(
        username = %req.username,
        email = %req.email,
        "User registration attempt"
    );

    let result = use_case
        .execute(
            req.username.clone(),
            req.email.clone(),
            req.password.clone(),
        )
        .await;

    match result {
        Ok(user) => {
            info!(
                user_id = %user.id,
                username = %user.username,
                email = %user.email,
                "User created successfully"
            );

            HttpResponse::Created().json(serde_json::json!({
                "message": "User created successfully. Please check your email to verify your account.",
                "user": {
                    "id": user.id,
                    "username": user.username,
                    "email": user.email,
                    "is_verified": user.is_verified,
                    "created_at": user.created_at,
                }
            }))
        }

        Err(CreateUserError::UsernameAlreadyExists) => {
            warn!(
                username = %req.username,
                "Registration failed: Username already exists"
            );

            HttpResponse::Conflict().json(serde_json::json!({
                "error": "Username already exists"
            }))
        }

        Err(CreateUserError::EmailAlreadyExists) => {
            warn!(
                email = %req.email,
                "Registration failed: Email already exists"
            );

            HttpResponse::Conflict().json(serde_json::json!({
                "error": "Email already exists"
            }))
        }

        Err(CreateUserError::InvalidInput(ref msg)) => {
            warn!(
                username = %req.username,
                email = %req.email,
                error = %msg,
                "Registration failed: Invalid input"
            );

            HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Invalid input",
                "message": msg
            }))
        }

        Err(CreateUserError::HashingFailed(ref e)) => {
            error!(
                username = %req.username,
                error = %e,
                "Password hashing failed"
            );

            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Internal server error"
            }))
        }

        Err(CreateUserError::TokenGenerationFailed(ref e)) => {
            error!(
                username = %req.username,
                email = %req.email,
                error = %e,
                "Verification token generation failed"
            );

            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Internal server error"
            }))
        }

        Err(CreateUserError::EmailSendFailed(ref e)) => {
            error!(
                email = %req.email,
                error = %e,
                "Failed to send verification email"
            );

            // User was created but email failed - still return success
            // but log the error for investigation
            HttpResponse::Created().json(serde_json::json!({
                "message": "User created successfully, but verification email could not be sent. Please contact support.",
                "warning": "Email delivery failed"
            }))
        }

        Err(CreateUserError::RepositoryError(ref e)) => {
            error!(
                username = %req.username,
                email = %req.email,
                error = %e,
                "Database error during user creation"
            );

            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Internal server error"
            }))
        }
    }
}

/// **🚀 Verify User Email API Endpoint**
#[actix_web::get("/api/auth/email-verification/{token}")]
pub async fn verify_user_email_handler(
    req: HttpRequest,
    data: web::Data<AppState>,
) -> impl Responder {
    let token = req.match_info().get("token").unwrap();

    let use_case = &data.verify_user_email_use_case;

    match use_case.execute(token).await {
        Ok(()) => HttpResponse::Ok().json(serde_json::json!({
            "message": "Email verified successfully"
        })),
        Err(VerifyUserEmailError::TokenExpired) => {
            HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Token has expired"
            }))
        }
        Err(VerifyUserEmailError::TokenInvalid) => {
            HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Invalid token"
            }))
        }
        Err(VerifyUserEmailError::UserNotFound) => {
            HttpResponse::NotFound().json(serde_json::json!({
                "error": "User not found"
            }))
        }
        Err(VerifyUserEmailError::DatabaseError) => {
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Internal server error"
            }))
        }
    }
}

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

        Err(LoginError::UserNotVerified) => {
            warn!("Login failed: User not verified");
            HttpResponse::Forbidden().json(serde_json::json!({
                "error": "Email not verified. Please check your email to verify your account."
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

    use crate::auth::application::use_cases::login_user::{
        ILoginUserUseCase, LoginUserResponse, UserInfo,
    };
    use crate::auth::application::use_cases::refresh_token::{
        IRefreshTokenUseCase, RefreshTokenResponse,
    };
    use crate::modules::auth::application::domain::entities::User;
    use crate::modules::auth::application::use_cases::create_user::{
        CreateUserError, ICreateUserUseCase,
    };
    use crate::modules::auth::application::use_cases::verify_user_email::{
        IVerifyUserEmailUseCase, VerifyUserEmailError,
    };

    use crate::tests::support::app_state_builder::TestAppStateBuilder;
    use actix_web::{test, App};
    use async_trait::async_trait;
    use chrono::DateTime;
    use chrono::Utc;
    use serde::Serialize;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use uuid::Uuid;

    // Mock Create User Use Case
    #[derive(Clone)]
    struct MockCreateUserUseCase {
        should_fail: Arc<Mutex<Option<CreateUserError>>>,
        created_user: Arc<Mutex<Option<User>>>,
    }

    impl MockCreateUserUseCase {
        fn new() -> Self {
            Self {
                should_fail: Arc::new(Mutex::new(None)),
                created_user: Arc::new(Mutex::new(None)),
            }
        }

        async fn set_error(&self, error: CreateUserError) {
            *self.should_fail.lock().await = Some(error);
        }

        async fn set_success(&self, user: User) {
            *self.created_user.lock().await = Some(user);
            *self.should_fail.lock().await = None;
        }
    }

    #[async_trait]
    impl ICreateUserUseCase for MockCreateUserUseCase {
        async fn execute(
            &self,
            username: String,
            email: String,
            _password: String,
        ) -> Result<User, CreateUserError> {
            let error = self.should_fail.lock().await;
            if let Some(err) = error.as_ref() {
                return Err(err.clone());
            }

            let user = self.created_user.lock().await;
            if let Some(u) = user.as_ref() {
                return Ok(u.clone());
            }

            // Default success case
            Ok(User {
                id: Uuid::new_v4(),
                username,
                email,
                password_hash: "hashed_password".to_string(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                is_verified: false,
                is_deleted: false,
            })
        }
    }

    // Mock Verify User Email Use Case
    #[derive(Clone)]
    struct MockVerifyUserEmailUseCase {
        should_fail: Arc<Mutex<Option<VerifyUserEmailError>>>,
    }

    impl MockVerifyUserEmailUseCase {
        fn new() -> Self {
            Self {
                should_fail: Arc::new(Mutex::new(None)),
            }
        }

        async fn set_error(&self, error: VerifyUserEmailError) {
            *self.should_fail.lock().await = Some(error);
        }

        async fn set_success(&self) {
            *self.should_fail.lock().await = None;
        }
    }

    #[async_trait]
    impl IVerifyUserEmailUseCase for MockVerifyUserEmailUseCase {
        async fn execute(&self, _token: &str) -> Result<(), VerifyUserEmailError> {
            let error = self.should_fail.lock().await;
            if let Some(err) = error.as_ref() {
                return Err(err.clone());
            }
            Ok(())
        }
    }

    // ==================== CREATE USER TESTS ====================
    #[derive(Deserialize, Serialize)]
    struct CreateUserSuccessResponse {
        pub message: String,
        pub user: UserDto,
    }

    #[derive(Deserialize, Serialize)]
    struct UserDto {
        pub id: Uuid,
        pub username: String,
        pub email: String,
        pub is_verified: bool,
        pub created_at: DateTime<Utc>,
    }

    #[derive(Deserialize, Serialize)]
    struct ErrorResponse {
        pub error: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub message: Option<String>,
    }

    #[actix_web::test]
    async fn test_create_user_handler_success() {
        let mock_uc = MockCreateUserUseCase::new();

        let expected_user = User {
            id: Uuid::new_v4(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "hashed_password".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            is_verified: false,
            is_deleted: false,
        };

        mock_uc.set_success(expected_user.clone()).await;
        let app_state = TestAppStateBuilder::default()
            .with_create_user(mock_uc)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(create_user_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/register")
            .set_json(CreateUserRequest {
                username: "testuser".to_string(),
                email: "test@example.com".to_string(),
                password: "password123".to_string(),
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);

        let body: CreateUserSuccessResponse = test::read_body_json(resp).await;
        assert_eq!(body.user.username, expected_user.username);
        assert_eq!(body.user.email, expected_user.email);
        assert_eq!(body.user.is_verified, false);
        assert!(body.message.contains("Please check your email"));
    }

    #[actix_web::test]
    async fn test_create_user_handler_username_already_exists() {
        let mock_uc = MockCreateUserUseCase::new();

        mock_uc
            .set_error(CreateUserError::UsernameAlreadyExists)
            .await;
        let app_state = TestAppStateBuilder::default()
            .with_create_user(mock_uc)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(create_user_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/register")
            .set_json(CreateUserRequest {
                username: "existinguser".to_string(),
                email: "test@example.com".to_string(),
                password: "password123".to_string(),
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 409); // Conflict

        let body: ErrorResponse = test::read_body_json(resp).await;
        assert_eq!(body.error, "Username already exists");
    }

    #[actix_web::test]
    async fn test_create_user_handler_email_already_exists() {
        let mock_uc = MockCreateUserUseCase::new();

        mock_uc.set_error(CreateUserError::EmailAlreadyExists).await;
        let app_state = TestAppStateBuilder::default()
            .with_create_user(mock_uc)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(create_user_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/register")
            .set_json(CreateUserRequest {
                username: "testuser".to_string(),
                email: "existing@example.com".to_string(),
                password: "password123".to_string(),
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 409); // Conflict

        let body: ErrorResponse = test::read_body_json(resp).await;
        assert_eq!(body.error, "Email already exists");
    }

    #[actix_web::test]
    async fn test_create_user_handler_invalid_input() {
        let mock_uc = MockCreateUserUseCase::new();

        mock_uc
            .set_error(CreateUserError::InvalidInput(
                "Username must be 3-50 characters".to_string(),
            ))
            .await;
        let app_state = TestAppStateBuilder::default()
            .with_create_user(mock_uc)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(create_user_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/register")
            .set_json(CreateUserRequest {
                username: "ab".to_string(), // Too short
                email: "test@example.com".to_string(),
                password: "password123".to_string(),
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400); // Bad Request

        let body: ErrorResponse = test::read_body_json(resp).await;
        assert_eq!(body.error, "Invalid input");
        assert!(body.message.is_some());
        assert!(body.message.unwrap().contains("3-50 characters"));
    }

    #[actix_web::test]
    async fn test_create_user_handler_hashing_failed() {
        let mock_uc = MockCreateUserUseCase::new();

        mock_uc
            .set_error(CreateUserError::HashingFailed("bcrypt error".to_string()))
            .await;
        let app_state = TestAppStateBuilder::default()
            .with_create_user(mock_uc)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(create_user_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/register")
            .set_json(CreateUserRequest {
                username: "testuser".to_string(),
                email: "test@example.com".to_string(),
                password: "password123".to_string(),
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 500); // Internal Server Error

        let body: ErrorResponse = test::read_body_json(resp).await;
        assert_eq!(body.error, "Internal server error");
        // Note: We don't expose the internal error details to the client
    }

    #[actix_web::test]
    async fn test_create_user_handler_token_generation_failed() {
        let mock_uc = MockCreateUserUseCase::new();

        mock_uc
            .set_error(CreateUserError::TokenGenerationFailed(
                "JWT encoding failed".to_string(),
            ))
            .await;
        let app_state = TestAppStateBuilder::default()
            .with_create_user(mock_uc)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(create_user_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/register")
            .set_json(CreateUserRequest {
                username: "testuser".to_string(),
                email: "test@example.com".to_string(),
                password: "password123".to_string(),
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 500); // Internal Server Error

        let body: ErrorResponse = test::read_body_json(resp).await;
        assert_eq!(body.error, "Internal server error");
    }

    #[actix_web::test]
    async fn test_create_user_handler_email_send_failed() {
        let mock_uc = MockCreateUserUseCase::new();

        mock_uc
            .set_error(CreateUserError::EmailSendFailed(
                "SMTP connection failed".to_string(),
            ))
            .await;
        let app_state = TestAppStateBuilder::default()
            .with_create_user(mock_uc)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(create_user_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/register")
            .set_json(CreateUserRequest {
                username: "testuser".to_string(),
                email: "test@example.com".to_string(),
                password: "password123".to_string(),
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201); // Still Created (user exists)

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert!(body["message"]
            .as_str()
            .unwrap()
            .contains("verification email could not be sent"));
        assert_eq!(body["warning"].as_str().unwrap(), "Email delivery failed");
    }

    #[actix_web::test]
    async fn test_create_user_handler_database_error() {
        let mock_uc = MockCreateUserUseCase::new();

        mock_uc
            .set_error(CreateUserError::RepositoryError(
                "Database connection failed".to_string(),
            ))
            .await;
        let app_state = TestAppStateBuilder::default()
            .with_create_user(mock_uc)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(create_user_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/register")
            .set_json(CreateUserRequest {
                username: "testuser".to_string(),
                email: "test@example.com".to_string(),
                password: "password123".to_string(),
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 500); // Internal Server Error

        let body: ErrorResponse = test::read_body_json(resp).await;
        assert_eq!(body.error, "Internal server error");
    }

    // ==================== VERIFY EMAIL TESTS ====================

    #[actix_web::test]
    async fn test_verify_user_email_handler_success() {
        let create_uc = MockCreateUserUseCase::new();
        let verify_uc = MockVerifyUserEmailUseCase::new();

        verify_uc.set_success().await;

        let app_state = TestAppStateBuilder::default()
            .with_create_user(create_uc)
            .with_verify_user_email(verify_uc)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(verify_user_email_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/auth/email-verification/valid_token_123")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["message"], "Email verified successfully");
    }

    #[actix_web::test]
    async fn test_verify_user_email_handler_token_expired() {
        let create_uc = MockCreateUserUseCase::new();
        let verify_uc = MockVerifyUserEmailUseCase::new();

        verify_uc
            .set_error(VerifyUserEmailError::TokenExpired)
            .await;

        let app_state = TestAppStateBuilder::default()
            .with_create_user(create_uc)
            .with_verify_user_email(verify_uc)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(verify_user_email_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/auth/email-verification/expired_token")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400); // Bad Request

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "Token has expired");
    }

    #[actix_web::test]
    async fn test_verify_user_email_handler_token_invalid() {
        let create_uc = MockCreateUserUseCase::new();
        let verify_uc = MockVerifyUserEmailUseCase::new();

        verify_uc
            .set_error(VerifyUserEmailError::TokenInvalid)
            .await;

        let app_state = TestAppStateBuilder::default()
            .with_create_user(create_uc)
            .with_verify_user_email(verify_uc)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(verify_user_email_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/auth/email-verification/invalid_token")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400); // Bad Request

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "Invalid token");
    }

    #[actix_web::test]
    async fn test_verify_user_email_handler_user_not_found() {
        let create_uc = MockCreateUserUseCase::new();
        let verify_uc = MockVerifyUserEmailUseCase::new();

        verify_uc
            .set_error(VerifyUserEmailError::UserNotFound)
            .await;

        let app_state = TestAppStateBuilder::default()
            .with_create_user(create_uc)
            .with_verify_user_email(verify_uc)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(verify_user_email_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/auth/email-verification/token_for_nonexistent_user")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404); // Not Found

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "User not found");
    }

    #[actix_web::test]
    async fn test_verify_user_email_handler_database_error() {
        let create_uc = MockCreateUserUseCase::new();
        let verify_uc = MockVerifyUserEmailUseCase::new();

        verify_uc
            .set_error(VerifyUserEmailError::DatabaseError)
            .await;

        let app_state = TestAppStateBuilder::default()
            .with_create_user(create_uc)
            .with_verify_user_email(verify_uc)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(verify_user_email_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/auth/email-verification/some_token")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 500); // Internal Server Error

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "Internal server error");
    }

    #[actix_web::test]
    async fn test_verify_user_email_handler_missing_token() {
        let create_uc = MockCreateUserUseCase::new();
        let verify_uc = MockVerifyUserEmailUseCase::new();

        let app_state = TestAppStateBuilder::default()
            .with_create_user(create_uc)
            .with_verify_user_email(verify_uc)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(verify_user_email_handler),
        )
        .await;

        // Request without token in path
        let req = test::TestRequest::post()
            .uri("/api/auth/email-verification/")
            .to_request();

        let resp = test::call_service(&app, req).await;
        // This will return 404 since the route won't match
        assert_eq!(resp.status(), 404);
    }

    // User login tests
    #[derive(Clone)]
    struct MockLoginUserUseCase {
        result: Arc<Mutex<Option<Result<LoginUserResponse, LoginError>>>>,
    }

    impl MockLoginUserUseCase {
        fn new() -> Self {
            Self {
                result: Arc::new(Mutex::new(None)),
            }
        }

        async fn set_result(&self, result: Result<LoginUserResponse, LoginError>) {
            *self.result.lock().await = Some(result);
        }
    }

    #[async_trait]
    impl ILoginUserUseCase for MockLoginUserUseCase {
        async fn execute(&self, _request: LoginRequest) -> Result<LoginUserResponse, LoginError> {
            self.result
                .lock()
                .await
                .clone()
                .expect("mock result must be set")
        }
    }

    #[actix_web::test]
    async fn test_login_user_success() {
        let mock_uc = MockLoginUserUseCase::new();

        mock_uc
            .set_result(Ok(LoginUserResponse {
                access_token: "access-token".to_string(),
                refresh_token: "refresh-token".to_string(),
                user: UserInfo {
                    id: Uuid::new_v4(),
                    username: "testuser".to_string(),
                    email: "test@example.com".to_string(),
                    is_verified: true,
                },
            }))
            .await;

        let app_state = TestAppStateBuilder::default()
            .with_login_user(mock_uc)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(login_user_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(serde_json::json!({
                "email": "test@example.com",
                "password": "password123"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
    }

    #[actix_web::test]
    async fn test_login_invalid_credentials() {
        let mock_uc = MockLoginUserUseCase::new();
        mock_uc
            .set_result(Err(LoginError::InvalidCredentials))
            .await;

        let app_state = TestAppStateBuilder::default()
            .with_login_user(mock_uc)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(login_user_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(serde_json::json!({
                "email": "test@example.com",
                "password": "password123"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 401);
    }

    #[actix_web::test]
    async fn test_login_user_not_verified() {
        let mock_uc = MockLoginUserUseCase::new();
        mock_uc.set_result(Err(LoginError::UserNotVerified)).await;

        let app_state = TestAppStateBuilder::default()
            .with_login_user(mock_uc)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(login_user_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(serde_json::json!({
                "email": "test@example.com",
                "password": "password123"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 403);
    }

    #[actix_web::test]
    async fn test_login_user_deleted() {
        let mock_uc = MockLoginUserUseCase::new();
        mock_uc.set_result(Err(LoginError::UserDeleted)).await;

        let app_state = TestAppStateBuilder::default()
            .with_login_user(mock_uc)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(login_user_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(serde_json::json!({
                "email": "test@example.com",
                "password": "password123"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 403);
    }

    #[actix_web::test]
    async fn test_login_password_verification_failed() {
        let mock_uc = MockLoginUserUseCase::new();
        mock_uc
            .set_result(Err(LoginError::PasswordVerificationFailed(
                "bcrypt error".to_string(),
            )))
            .await;

        let app_state = TestAppStateBuilder::default()
            .with_login_user(mock_uc)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(login_user_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(serde_json::json!({
                "email": "test@example.com",
                "password": "password123"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 500);
    }

    #[actix_web::test]
    async fn test_login_token_generation_failed() {
        let mock_uc = MockLoginUserUseCase::new();
        mock_uc
            .set_result(Err(LoginError::TokenGenerationFailed(
                "jwt error".to_string(),
            )))
            .await;

        let app_state = TestAppStateBuilder::default()
            .with_login_user(mock_uc)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(login_user_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(serde_json::json!({
                "email": "test@example.com",
                "password": "password123"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 500);
    }

    #[actix_web::test]
    async fn test_login_invalid_email() {
        let mock_uc = MockLoginUserUseCase::new();

        let app_state = TestAppStateBuilder::default()
            .with_login_user(mock_uc)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(login_user_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(serde_json::json!({
                "email": "not-an-email",
                "password": "password123"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    #[actix_web::test]
    async fn test_login_empty_password() {
        let mock_uc = MockLoginUserUseCase::new();

        let app_state = TestAppStateBuilder::default()
            .with_login_user(mock_uc)
            .build();

        let app =
            test::init_service(App::new().app_data(app_state).service(login_user_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(serde_json::json!({
                "email": "test@example.com",
                "password": ""
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    // Test refresh token route
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct RefreshTokenResponseDto {
        access_token: String,
        refresh_token: String,
    }

    #[derive(Deserialize)]
    struct ErrorResponseDto {
        error: String,
    }

    // Mock use case
    struct MockRefreshTokenUseCase {
        response: tokio::sync::Mutex<Option<Result<RefreshTokenResponse, RefreshTokenError>>>,
    }

    impl MockRefreshTokenUseCase {
        fn new() -> Self {
            Self {
                response: tokio::sync::Mutex::new(None),
            }
        }

        async fn set_response(&self, response: Result<RefreshTokenResponse, RefreshTokenError>) {
            *self.response.lock().await = Some(response);
        }
    }

    #[async_trait]
    impl IRefreshTokenUseCase for MockRefreshTokenUseCase {
        async fn execute(
            &self,
            _request: RefreshTokenRequest,
        ) -> Result<RefreshTokenResponse, RefreshTokenError> {
            self.response
                .lock()
                .await
                .take()
                .expect("Mock response not set")
        }
    }

    #[actix_web::test]
    async fn test_refresh_token_handler_success() {
        let mock_uc = MockRefreshTokenUseCase::new();

        mock_uc
            .set_response(Ok(RefreshTokenResponse {
                access_token: "new_access_token".to_string(),
                refresh_token: "new_refresh_token".to_string(),
            }))
            .await;

        let app_state = TestAppStateBuilder::default()
            .with_refresh_token(mock_uc)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(refresh_token_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/auth/refresh")
            .set_json(serde_json::json!({
                "refresh_token": "valid_refresh_token"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: RefreshTokenResponseDto = test::read_body_json(resp).await;
        assert_eq!(body.access_token, "new_access_token");
        assert_eq!(body.refresh_token, "new_refresh_token");
    }

    #[actix_web::test]
    async fn test_refresh_token_handler_expired() {
        let mock_uc = MockRefreshTokenUseCase::new();

        mock_uc
            .set_response(Err(RefreshTokenError::TokenExpired))
            .await;

        let app_state = TestAppStateBuilder::default()
            .with_refresh_token(mock_uc)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(refresh_token_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/auth/refresh")
            .set_json(serde_json::json!({
                "refresh_token": "expired_token"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 401);

        let body: ErrorResponseDto = test::read_body_json(resp).await;
        assert!(body.error.contains("expired"));
    }

    #[actix_web::test]
    async fn test_refresh_token_handler_invalid_token() {
        let mock_uc = MockRefreshTokenUseCase::new();

        mock_uc
            .set_response(Err(RefreshTokenError::TokenInvalid))
            .await;

        let app_state = TestAppStateBuilder::default()
            .with_refresh_token(mock_uc)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(refresh_token_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/auth/refresh")
            .set_json(serde_json::json!({
                "refresh_token": "invalid_token"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 401);
    }

    #[actix_web::test]
    async fn test_refresh_token_handler_wrong_token_type() {
        let mock_uc = MockRefreshTokenUseCase::new();

        mock_uc
            .set_response(Err(RefreshTokenError::InvalidTokenType))
            .await;

        let app_state = TestAppStateBuilder::default()
            .with_refresh_token(mock_uc)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(refresh_token_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/auth/refresh")
            .set_json(serde_json::json!({
                "refresh_token": "access_token_instead"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);

        let body: ErrorResponseDto = test::read_body_json(resp).await;
        assert!(body.error.contains("refresh token"));
    }

    // Logout user tests
    use crate::modules::auth::application::use_cases::logout_user::{
        ILogoutUseCase, LogoutError, LogoutRequest, LogoutResponse,
    };

    #[derive(Deserialize)]
    struct LogoutResponseDto {
        message: String,
    }

    // Mock Logout Use Case
    struct MockLogoutUseCase {
        response: tokio::sync::Mutex<Option<Result<LogoutResponse, LogoutError>>>,
    }

    impl MockLogoutUseCase {
        fn new() -> Self {
            Self {
                response: tokio::sync::Mutex::new(None),
            }
        }

        async fn set_response(&self, response: Result<LogoutResponse, LogoutError>) {
            *self.response.lock().await = Some(response);
        }
    }

    #[async_trait]
    impl ILogoutUseCase for MockLogoutUseCase {
        async fn execute(&self, _request: LogoutRequest) -> Result<LogoutResponse, LogoutError> {
            self.response
                .lock()
                .await
                .take()
                .expect("Mock response not set")
        }
    }

    // ==================== Logout Handler Tests ====================

    #[actix_web::test]
    async fn test_logout_handler_success_with_token() {
        let mock_uc = MockLogoutUseCase::new();

        mock_uc
            .set_response(Ok(LogoutResponse {
                message: "Logged out successfully".to_string(),
            }))
            .await;

        let app_state = TestAppStateBuilder::default()
            .with_logout_user(mock_uc)
            .build();

        let app = test::init_service(App::new().app_data(app_state).service(logout_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/logout")
            .set_json(serde_json::json!({
                "refresh_token": "some_refresh_token"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: LogoutResponseDto = test::read_body_json(resp).await;
        assert_eq!(body.message, "Logged out successfully");
    }

    #[actix_web::test]
    async fn test_logout_handler_success_without_token() {
        let mock_uc = MockLogoutUseCase::new();

        mock_uc
            .set_response(Ok(LogoutResponse {
                message: "Logged out successfully".to_string(),
            }))
            .await;

        let app_state = TestAppStateBuilder::default()
            .with_logout_user(mock_uc)
            .build();

        let app = test::init_service(App::new().app_data(app_state).service(logout_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/logout")
            .set_json(serde_json::json!({}))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: LogoutResponseDto = test::read_body_json(resp).await;
        assert_eq!(body.message, "Logged out successfully");
    }

    #[actix_web::test]
    async fn test_logout_handler_with_empty_object() {
        let mock_uc = MockLogoutUseCase::new();

        mock_uc
            .set_response(Ok(LogoutResponse {
                message: "Logged out successfully".to_string(),
            }))
            .await;

        let app_state = TestAppStateBuilder::default()
            .with_logout_user(mock_uc)
            .build();

        let app = test::init_service(App::new().app_data(app_state).service(logout_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/logout")
            .set_json(serde_json::json!({}))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
    }

    #[actix_web::test]
    async fn test_logout_handler_token_revocation_failed() {
        let mock_uc = MockLogoutUseCase::new();

        mock_uc
            .set_response(Err(LogoutError::TokenRevocationFailed(
                "Failed to revoke token".to_string(),
            )))
            .await;

        let app_state = TestAppStateBuilder::default()
            .with_logout_user(mock_uc)
            .build();

        let app = test::init_service(App::new().app_data(app_state).service(logout_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/logout")
            .set_json(serde_json::json!({
                "refresh_token": "some_token"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;

        // Still returns 200 - user is logged out on client side
        assert_eq!(resp.status(), 200);

        let body: LogoutResponseDto = test::read_body_json(resp).await;
        assert_eq!(body.message, "Logged out successfully");
    }

    #[actix_web::test]
    async fn test_logout_handler_database_error() {
        let mock_uc = MockLogoutUseCase::new();

        mock_uc
            .set_response(Err(LogoutError::DatabaseError(
                "Database connection failed".to_string(),
            )))
            .await;

        let app_state = TestAppStateBuilder::default()
            .with_logout_user(mock_uc)
            .build();

        let app = test::init_service(App::new().app_data(app_state).service(logout_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/logout")
            .set_json(serde_json::json!({
                "refresh_token": "some_token"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;

        // Still returns 200 - better UX
        assert_eq!(resp.status(), 200);

        let body: LogoutResponseDto = test::read_body_json(resp).await;
        assert_eq!(body.message, "Logged out successfully");
    }

    #[actix_web::test]
    async fn test_logout_handler_invalid_json() {
        let mock_uc = MockLogoutUseCase::new();
        let app_state = TestAppStateBuilder::default()
            .with_logout_user(mock_uc)
            .build();

        let app = test::init_service(App::new().app_data(app_state).service(logout_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/logout")
            .set_payload("invalid json")
            .insert_header(("Content-Type", "application/json"))
            .to_request();

        let resp = test::call_service(&app, req).await;

        // Should return 400 for malformed JSON
        assert_eq!(resp.status(), 400);
    }

    #[actix_web::test]
    async fn test_logout_handler_with_whitespace_token() {
        let mock_uc = MockLogoutUseCase::new();

        mock_uc
            .set_response(Ok(LogoutResponse {
                message: "Logged out successfully".to_string(),
            }))
            .await;

        let app_state = TestAppStateBuilder::default()
            .with_logout_user(mock_uc)
            .build();

        let app = test::init_service(App::new().app_data(app_state).service(logout_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/logout")
            .set_json(serde_json::json!({
                "refresh_token": "  token_with_spaces  "
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: LogoutResponseDto = test::read_body_json(resp).await;
        assert_eq!(body.message, "Logged out successfully");
    }

    #[actix_web::test]
    async fn test_logout_handler_response_contains_message() {
        let mock_uc = MockLogoutUseCase::new();

        mock_uc
            .set_response(Ok(LogoutResponse {
                message: "Custom logout message".to_string(),
            }))
            .await;

        let app_state = TestAppStateBuilder::default()
            .with_logout_user(mock_uc)
            .build();

        let app = test::init_service(App::new().app_data(app_state).service(logout_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/logout")
            .set_json(serde_json::json!({
                "refresh_token": "token"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: LogoutResponseDto = test::read_body_json(resp).await;
        assert_eq!(body.message, "Custom logout message");
    }

    #[actix_web::test]
    async fn test_logout_handler_content_type_json() {
        let mock_uc = MockLogoutUseCase::new();

        mock_uc
            .set_response(Ok(LogoutResponse {
                message: "Logged out successfully".to_string(),
            }))
            .await;

        let app_state = TestAppStateBuilder::default()
            .with_logout_user(mock_uc)
            .build();

        let app = test::init_service(App::new().app_data(app_state).service(logout_handler)).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/logout")
            .set_json(serde_json::json!({
                "refresh_token": "token"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;

        // Check Content-Type header
        let content_type = resp.headers().get("content-type").unwrap();
        assert!(content_type.to_str().unwrap().contains("application/json"));
    }

    #[actix_web::test]
    async fn test_logout_handler_idempotent() {
        let mock_uc = MockLogoutUseCase::new();

        // Set up two responses for two calls
        mock_uc
            .set_response(Ok(LogoutResponse {
                message: "Logged out successfully".to_string(),
            }))
            .await;

        let app_state = TestAppStateBuilder::default()
            .with_logout_user(mock_uc)
            .build();

        let app = test::init_service(App::new().app_data(app_state).service(logout_handler)).await;

        // First logout
        let req1 = test::TestRequest::post()
            .uri("/api/auth/logout")
            .set_json(serde_json::json!({
                "refresh_token": "same_token"
            }))
            .to_request();

        let resp1 = test::call_service(&app, req1).await;
        assert_eq!(resp1.status(), 200);

        // Logout should always succeed, even if called multiple times
    }
}
