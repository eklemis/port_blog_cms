use crate::api::schemas::{ErrorResponse, SuccessResponse};
use crate::auth::application::orchestrator::user_registration::UserRegistrationError;
use crate::auth::application::use_cases::create_user::CreateUserInput;
use crate::modules::auth::application::use_cases::create_user::CreateUserError;
use crate::shared::api::ApiResponse;
use crate::AppState;
use actix_web::{post, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};
use utoipa::ToSchema;

/// Request body for user registration
#[derive(Serialize, Deserialize, ToSchema)]
pub struct CreateUserRequest {
    /// Username (unique identifier)
    #[schema(example = "johndoe")]
    pub username: String,

    /// Email address
    #[schema(example = "john@example.com")]
    pub email: String,

    /// Password (minimum 8 characters)
    #[schema(example = "SecurePass123!")]
    pub password: String,

    /// Full name of the user
    #[schema(example = "John Doe")]
    pub full_name: String,
}

#[derive(Serialize, ToSchema)]
pub struct RegisterUserResponse {
    /// Success message
    #[schema(
        example = "User created successfully. Please check your email to verify your account."
    )]
    message: String,

    /// Created user details
    user: RegisteredUser,
}

#[derive(Serialize, ToSchema)]
pub struct RegisteredUser {
    /// User ID (UUID)
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    id: String,

    /// Username
    #[schema(example = "johndoe")]
    username: String,

    /// Email address
    #[schema(example = "john@example.com")]
    email: String,

    /// Full name
    #[schema(example = "John Doe")]
    full_name: String,
}

fn map_create_user_error(err: CreateUserError, req: &CreateUserRequest) -> HttpResponse {
    match &err {
        CreateUserError::InvalidUsername(msg) => {
            warn!(
                username = %req.username,
                email = %req.email,
                error = %err,
                "Invalid registration input"
            );
            ApiResponse::bad_request("INVALID_USERNAME", &msg)
        }

        CreateUserError::InvalidEmail(msg) => {
            warn!(
                username = %req.username,
                email = %req.email,
                error = %err,
                "Invalid registration input"
            );
            ApiResponse::bad_request("INVALID_EMAIL", &msg)
        }

        CreateUserError::InvalidPassword(msg) => {
            warn!(
                username = %req.username,
                email = %req.email,
                error = %err,
                "Invalid registration input"
            );
            ApiResponse::bad_request("INVALID_PASSWORD", &msg)
        }

        CreateUserError::InvalidFullName(msg) => {
            warn!(
                username = %req.username,
                email = %req.email,
                error = %err,
                "Invalid registration input"
            );
            ApiResponse::bad_request("INVALID_FULL_NAME", &msg)
        }

        CreateUserError::UserAlreadyExists => {
            warn!(
                username = %req.username,
                email = %req.email,
                "User already exists"
            );
            ApiResponse::conflict("USER_ALREADY_EXISTS", "User already exists")
        }

        other => {
            error!(
                username = %req.username,
                email = %req.email,
                error = %other,
                "Unhandled user creation error"
            );
            ApiResponse::internal_error()
        }
    }
}

/// Register a new user
///
/// Creates a new user account and sends a verification email.
/// The user must verify their email before they can access protected endpoints.
#[utoipa::path(
    post,
    path = "/api/auth/register",
    tag = "auth",
    request_body = CreateUserRequest,
    responses(
        (
            status = 201,
            description = "User created successfully",
            body = inline(SuccessResponse<RegisterUserResponse>),
            example = json!({
                "success": true,
                "data": {
                    "message": "User created successfully. Please check your email to verify your account.",
                    "user": {
                        "id": "123e4567-e89b-12d3-a456-426614174000",
                        "username": "johndoe",
                        "email": "john@example.com",
                        "fullName": "John Doe"
                    }
                }
            })
        ),
        (
            status = 400,
            description = "Validation error",
            body = ErrorResponse,
            examples(
                ("Invalid username" = (value = json!({
                    "success": false,
                    "error": {
                        "code": "INVALID_USERNAME",
                        "message": "Username must be between 3 and 30 characters"
                    }
                }))),
                ("Invalid email" = (value = json!({
                    "success": false,
                    "error": {
                        "code": "INVALID_EMAIL",
                        "message": "Invalid email format"
                    }
                }))),
                ("Invalid password" = (value = json!({
                    "success": false,
                    "error": {
                        "code": "INVALID_PASSWORD",
                        "message": "Password must be at least 8 characters"
                    }
                }))),
                ("Invalid full name" = (value = json!({
                    "success": false,
                    "error": {
                        "code": "INVALID_FULL_NAME",
                        "message": "Full name is required"
                    }
                })))
            )
        ),
        (
            status = 409,
            description = "User already exists",
            body = ErrorResponse,
            example = json!({
                "success": false,
                "error": {
                    "code": "USER_ALREADY_EXISTS",
                    "message": "User already exists"
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
#[post("/api/auth/register")]
pub async fn register_user_handler(
    req: web::Json<CreateUserRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let orchestrator = &data.register_user_orchestrator;

    info!(
        username = %req.username,
        email = %req.email,
        "User registration attempt"
    );

    let user_input = CreateUserInput {
        username: req.username.clone(),
        email: req.email.clone(),
        password: req.password.clone(),
        full_name: req.full_name.clone(),
    };
    let result = orchestrator.register_user(user_input).await;

    match result {
        Ok(user) => {
            info!(
                user_id = %user.user_id,
                username = %user.username,
                email = %user.email,
                "User created successfully"
            );

            ApiResponse::created(RegisterUserResponse {
                message:
                    "User created successfully. Please check your email to verify your account."
                        .to_string(),
                user: RegisteredUser {
                    id: user.user_id.to_string(),
                    username: user.username,
                    email: user.email,
                    full_name: user.full_name,
                },
            })
        }

        Err(UserRegistrationError::CreateUserFailed(e)) => map_create_user_error(e, &req),

        Err(e) => {
            error!(
                username = %req.username,
                email = %req.email,
                error = %e,
                "User registration failed"
            );
            ApiResponse::internal_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::auth::application::orchestrator::user_registration::UserRegistrationOrchestrator;
    use crate::auth::application::ports::outgoing::{
        user_query::UserQueryError, user_repository::UserRepositoryError,
    };
    use crate::auth::application::use_cases::create_user::{
        CreateUserError, CreateUserInput, CreateUserOutput, ICreateUserUseCase,
    };
    use crate::email::application::ports::outgoing::user_email_notifier::{
        UserEmailNotificationError, UserEmailNotifier,
    };
    use crate::tests::support::app_state_builder::TestAppStateBuilder;
    use actix_web::{test, App};
    use async_trait::async_trait;
    use uuid::Uuid;

    // ========================================================================
    // Mock Use Cases for Different Error Scenarios
    // ========================================================================

    #[derive(Clone)]
    struct MockCreateUserSuccess;

    #[async_trait]
    impl ICreateUserUseCase for MockCreateUserSuccess {
        async fn execute(
            &self,
            input: CreateUserInput,
        ) -> Result<CreateUserOutput, CreateUserError> {
            Ok(CreateUserOutput {
                user_id: Uuid::new_v4(),
                username: input.username,
                email: input.email,
                full_name: input.full_name,
            })
        }
    }

    #[derive(Clone)]
    struct MockCreateUserInvalidUsername;

    #[async_trait]
    impl ICreateUserUseCase for MockCreateUserInvalidUsername {
        async fn execute(&self, _: CreateUserInput) -> Result<CreateUserOutput, CreateUserError> {
            Err(CreateUserError::InvalidUsername(
                "Username must be 3-20 characters".to_string(),
            ))
        }
    }

    #[derive(Clone)]
    struct MockCreateUserInvalidEmail;

    #[async_trait]
    impl ICreateUserUseCase for MockCreateUserInvalidEmail {
        async fn execute(&self, _: CreateUserInput) -> Result<CreateUserOutput, CreateUserError> {
            Err(CreateUserError::InvalidEmail(
                "Invalid email format".to_string(),
            ))
        }
    }

    #[derive(Clone)]
    struct MockCreateUserInvalidPassword;

    #[async_trait]
    impl ICreateUserUseCase for MockCreateUserInvalidPassword {
        async fn execute(&self, _: CreateUserInput) -> Result<CreateUserOutput, CreateUserError> {
            Err(CreateUserError::InvalidPassword(
                "Password must be at least 8 characters".to_string(),
            ))
        }
    }

    #[derive(Clone)]
    struct MockCreateUserInvalidFullName;

    #[async_trait]
    impl ICreateUserUseCase for MockCreateUserInvalidFullName {
        async fn execute(&self, _: CreateUserInput) -> Result<CreateUserOutput, CreateUserError> {
            Err(CreateUserError::InvalidFullName(
                "Full name cannot be empty".to_string(),
            ))
        }
    }

    #[derive(Clone)]
    struct MockCreateUserAlreadyExists;

    #[async_trait]
    impl ICreateUserUseCase for MockCreateUserAlreadyExists {
        async fn execute(&self, _: CreateUserInput) -> Result<CreateUserOutput, CreateUserError> {
            Err(CreateUserError::UserAlreadyExists)
        }
    }

    #[derive(Clone)]
    struct MockCreateUserHashingFailed;

    #[async_trait]
    impl ICreateUserUseCase for MockCreateUserHashingFailed {
        async fn execute(&self, _: CreateUserInput) -> Result<CreateUserOutput, CreateUserError> {
            Err(CreateUserError::HashingFailed(
                "Argon2 hashing failed".to_string(),
            ))
        }
    }

    #[derive(Clone)]
    struct MockCreateUserRepositoryError;

    #[async_trait]
    impl ICreateUserUseCase for MockCreateUserRepositoryError {
        async fn execute(&self, _: CreateUserInput) -> Result<CreateUserOutput, CreateUserError> {
            Err(CreateUserError::RepositoryError(
                UserRepositoryError::DatabaseError("Connection failed".to_string()),
            ))
        }
    }

    #[derive(Clone)]
    struct MockCreateUserQueryError;

    #[async_trait]
    impl ICreateUserUseCase for MockCreateUserQueryError {
        async fn execute(&self, _: CreateUserInput) -> Result<CreateUserOutput, CreateUserError> {
            Err(CreateUserError::QueryError(UserQueryError::DatabaseError(
                "Query failed".to_string(),
            )))
        }
    }

    #[derive(Clone)]
    struct MockEmailNotifierSuccess;

    #[async_trait]
    impl UserEmailNotifier for MockEmailNotifierSuccess {
        async fn send_verification_email(
            &self,
            _: CreateUserOutput,
        ) -> Result<(), UserEmailNotificationError> {
            Ok(())
        }
    }

    #[derive(Clone)]
    struct MockEmailNotifierFailure;

    #[async_trait]
    impl UserEmailNotifier for MockEmailNotifierFailure {
        async fn send_verification_email(
            &self,
            _: CreateUserOutput,
        ) -> Result<(), UserEmailNotificationError> {
            Err(UserEmailNotificationError::EmailSendingFailed(
                "SMTP connection failed".to_string(),
            ))
        }
    }

    // ========================================================================
    // Helper Functions
    // ========================================================================

    fn create_test_request() -> CreateUserRequest {
        CreateUserRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "SecurePass123!".to_string(),
            full_name: "Test User".to_string(),
        }
    }

    fn create_orchestrator(
        create_user: impl ICreateUserUseCase + Send + Sync + 'static,
        email_notifier: impl UserEmailNotifier + Send + Sync + 'static,
    ) -> Arc<UserRegistrationOrchestrator> {
        Arc::new(UserRegistrationOrchestrator::new(
            Arc::new(create_user),
            Arc::new(email_notifier),
        ))
    }

    // ========================================================================
    // Tests
    // ========================================================================

    #[actix_web::test]
    async fn test_register_user_success() {
        let orchestrator = create_orchestrator(MockCreateUserSuccess, MockEmailNotifierSuccess);

        let app_state = TestAppStateBuilder::default()
            .with_register_user_orchestrator(orchestrator)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(register_user_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/auth/register")
            .set_json(&create_test_request())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert_eq!(
            body["data"]["message"],
            "User created successfully. Please check your email to verify your account."
        );
        assert!(body["data"]["user"]["id"].is_string());
        assert_eq!(body["data"]["user"]["username"], "testuser");
        assert_eq!(body["data"]["user"]["email"], "test@example.com");
        assert_eq!(body["data"]["user"]["full_name"], "Test User");
        assert!(body.get("error").is_none());
    }

    #[actix_web::test]
    async fn test_register_user_invalid_username() {
        let orchestrator =
            create_orchestrator(MockCreateUserInvalidUsername, MockEmailNotifierSuccess);

        let app_state = TestAppStateBuilder::default()
            .with_register_user_orchestrator(orchestrator)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(register_user_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/auth/register")
            .set_json(&create_test_request())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "INVALID_USERNAME");
        assert!(body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Username"));
        assert!(body.get("data").is_none());
    }

    #[actix_web::test]
    async fn test_register_user_invalid_email() {
        let orchestrator =
            create_orchestrator(MockCreateUserInvalidEmail, MockEmailNotifierSuccess);

        let app_state = TestAppStateBuilder::default()
            .with_register_user_orchestrator(orchestrator)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(register_user_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/auth/register")
            .set_json(&create_test_request())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "INVALID_EMAIL");
        assert!(body["error"]["message"].as_str().unwrap().contains("email"));
        assert!(body.get("data").is_none());
    }

    #[actix_web::test]
    async fn test_register_user_invalid_password() {
        let orchestrator =
            create_orchestrator(MockCreateUserInvalidPassword, MockEmailNotifierSuccess);

        let app_state = TestAppStateBuilder::default()
            .with_register_user_orchestrator(orchestrator)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(register_user_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/auth/register")
            .set_json(&create_test_request())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "INVALID_PASSWORD");
        assert!(body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Password"));
        assert!(body.get("data").is_none());
    }

    #[actix_web::test]
    async fn test_register_user_invalid_full_name() {
        let orchestrator =
            create_orchestrator(MockCreateUserInvalidFullName, MockEmailNotifierSuccess);

        let app_state = TestAppStateBuilder::default()
            .with_register_user_orchestrator(orchestrator)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(register_user_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/auth/register")
            .set_json(&create_test_request())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "INVALID_FULL_NAME");
        assert!(body["error"]["message"].as_str().unwrap().contains("name"));
        assert!(body.get("data").is_none());
    }

    #[actix_web::test]
    async fn test_register_user_already_exists() {
        let orchestrator =
            create_orchestrator(MockCreateUserAlreadyExists, MockEmailNotifierSuccess);

        let app_state = TestAppStateBuilder::default()
            .with_register_user_orchestrator(orchestrator)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(register_user_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/auth/register")
            .set_json(&create_test_request())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 409);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "USER_ALREADY_EXISTS");
        assert_eq!(body["error"]["message"], "User already exists");
        assert!(body.get("data").is_none());
    }

    #[actix_web::test]
    async fn test_register_user_hashing_failed() {
        let orchestrator =
            create_orchestrator(MockCreateUserHashingFailed, MockEmailNotifierSuccess);

        let app_state = TestAppStateBuilder::default()
            .with_register_user_orchestrator(orchestrator)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(register_user_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/auth/register")
            .set_json(&create_test_request())
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
    async fn test_register_user_repository_error() {
        let orchestrator =
            create_orchestrator(MockCreateUserRepositoryError, MockEmailNotifierSuccess);

        let app_state = TestAppStateBuilder::default()
            .with_register_user_orchestrator(orchestrator)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(register_user_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/auth/register")
            .set_json(&create_test_request())
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
    async fn test_register_user_query_error() {
        let orchestrator = create_orchestrator(MockCreateUserQueryError, MockEmailNotifierSuccess);

        let app_state = TestAppStateBuilder::default()
            .with_register_user_orchestrator(orchestrator)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(register_user_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/auth/register")
            .set_json(&create_test_request())
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
    async fn test_register_user_succeeds_even_when_email_fails() {
        let orchestrator = create_orchestrator(MockCreateUserSuccess, MockEmailNotifierFailure);

        let app_state = TestAppStateBuilder::default()
            .with_register_user_orchestrator(orchestrator)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(register_user_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/auth/register")
            .set_json(&create_test_request())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert!(body["data"]["message"]
            .as_str()
            .unwrap()
            .contains("check your email"));
    }
}
