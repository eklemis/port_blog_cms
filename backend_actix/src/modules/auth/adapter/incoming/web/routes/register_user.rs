use crate::auth::application::orchestrator::user_registration::UserRegistrationError;
use crate::auth::application::use_cases::create_user::CreateUserInput;

use crate::modules::auth::application::use_cases::create_user::CreateUserError;
use crate::AppState;
use actix_web::{post, web, HttpResponse, Responder};
use serde::Deserialize;
use tracing::{error, info, warn};

/// **📥 Request Structure for Creating a User**
#[derive(serde::Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub full_name: String,
}

fn map_create_user_error(err: CreateUserError, req: &CreateUserRequest) -> HttpResponse {
    match err {
        CreateUserError::InvalidUsername(_)
        | CreateUserError::InvalidEmail(_)
        | CreateUserError::InvalidPassword(_)
        | CreateUserError::InvalidFullName(_) => {
            warn!(
                username = %req.username,
                email = %req.email,
                error = %err,
                "Invalid registration input"
            );

            HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Invalid input",
                "message": err.to_string()
            }))
        }

        CreateUserError::UserAlreadyExists => {
            warn!(
                username = %req.username,
                email = %req.email,
                "User already exists"
            );

            HttpResponse::Conflict().json(serde_json::json!({
                "error": "User already exists"
            }))
        }

        other => {
            error!(
                username = %req.username,
                email = %req.email,
                error = %other,
                "Unhandled user creation error"
            );

            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Internal server error"
            }))
        }
    }
}

/// **🚀 Register User API Endpoint**
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

            HttpResponse::Created().json(serde_json::json!({
                "message": "User created successfully. Please check your email to verify your account.",
                "user": {
                    "id": user.user_id,
                    "username": user.username,
                    "email": user.email,
                    "full_name": user.full_name,
                    "message": user.message
                }
            }))
        }

        Err(UserRegistrationError::CreateUserFailed(e)) => {
            // Delegate classification to a helper
            map_create_user_error(e, &req)
        }

        Err(e) => {
            // Any orchestration-level failure
            error!(
                username = %req.username,
                email = %req.email,
                error = %e,
                "User registration failed"
            );

            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Internal server error"
            }))
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
        assert_eq!(
            body["message"],
            "User created successfully. Please check your email to verify your account."
        );
        assert!(body["user"]["id"].is_string());
        assert_eq!(body["user"]["username"], "testuser");
        assert_eq!(body["user"]["email"], "test@example.com");
        assert_eq!(body["user"]["full_name"], "Test User");
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
        assert_eq!(body["error"], "Invalid input");
        assert!(body["message"].as_str().unwrap().contains("Username"));
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
        assert_eq!(body["error"], "Invalid input");
        assert!(body["message"].as_str().unwrap().contains("email"));
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
        assert_eq!(body["error"], "Invalid input");
        assert!(body["message"].as_str().unwrap().contains("Password"));
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
        assert_eq!(body["error"], "Invalid input");
        assert!(body["message"].as_str().unwrap().contains("name"));
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
        assert_eq!(body["error"], "User already exists");
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
        assert_eq!(body["error"], "Internal server error");
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
        assert_eq!(body["error"], "Internal server error");
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
        assert_eq!(body["error"], "Internal server error");
    }

    #[actix_web::test]
    async fn test_register_user_email_notification_failure() {
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
        // Email failure doesn't fail registration, still returns 201
        assert_eq!(resp.status(), 201);

        let body: serde_json::Value = test::read_body_json(resp).await;
        // Check for the alternative message when email fails
        assert!(body["user"]["message"]
            .as_str()
            .unwrap()
            .contains("trouble sending"));
    }
}
