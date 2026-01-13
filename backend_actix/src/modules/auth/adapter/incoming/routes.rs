use crate::auth::application::use_cases::verify_user_email::VerifyUserEmailError;
use crate::modules::auth::application::use_cases::create_user::CreateUserError;
use crate::AppState;
use actix_web::{post, web, HttpRequest, HttpResponse, Responder};
use serde::Deserialize;

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

    let result = use_case
        .execute(
            req.username.clone(),
            req.email.clone(),
            req.password.clone(),
        )
        .await;

    match result {
        Ok(user) => HttpResponse::Created().json(user),
        Err(CreateUserError::UsernameAlreadyExists) => {
            HttpResponse::Conflict().body("Username already exists")
        }
        Err(CreateUserError::EmailAlreadyExists) => {
            HttpResponse::Conflict().body("Email already exists")
        }
        Err(CreateUserError::HashingFailed(e)) => {
            HttpResponse::InternalServerError().body(format!("Password hashing failed: {}", e))
        }
        Err(CreateUserError::RepositoryError(e)) => {
            HttpResponse::InternalServerError().body(format!("Database error: {}", e))
        }
    }
}

/// **🚀 Verify User Email API Endpoint**
#[actix_web::post("/api/auth/email-verification/{token}")]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cv::application::ports::outgoing::{CreateCVData, UpdateCVData};
    use crate::cv::domain::entities::CVInfo;
    use crate::modules::auth::application::domain::entities::User;
    use crate::modules::auth::application::use_cases::create_user::{
        CreateUserError, ICreateUserUseCase,
    };
    use crate::modules::auth::application::use_cases::verify_user_email::{
        IVerifyUserEmailUseCase, VerifyUserEmailError,
    };
    use crate::modules::cv::application::use_cases::{
        create_cv::{CreateCVError, ICreateCVUseCase},
        fetch_cv::{FetchCVError, IFetchCVUseCase},
        update_cv::{IUpdateCVUseCase, UpdateCVError},
    };
    use actix_web::{test, web, App};
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
    // Stub implementations for other use cases (just to satisfy AppState)
    #[derive(Default)]
    struct StubFetchCVUseCase;
    #[async_trait]
    impl IFetchCVUseCase for StubFetchCVUseCase {
        async fn execute(&self, _id: String) -> Result<Vec<CVInfo>, FetchCVError> {
            unimplemented!("Not used in these tests")
        }
    }

    #[derive(Default)]
    struct StubCreateCVUseCase;
    #[async_trait]
    impl ICreateCVUseCase for StubCreateCVUseCase {
        async fn execute(&self, _id: String, _cv: CreateCVData) -> Result<CVInfo, CreateCVError> {
            unimplemented!("Not used in these tests")
        }
    }

    #[derive(Default)]
    struct StubUpdateCVUseCase;
    #[async_trait]
    impl IUpdateCVUseCase for StubUpdateCVUseCase {
        async fn execute(&self, _id: String, _cv: UpdateCVData) -> Result<CVInfo, UpdateCVError> {
            unimplemented!("Not used in these tests")
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

    // Helper to create test AppState
    fn create_test_app_state(
        create_user_uc: MockCreateUserUseCase,
        verify_email_uc: MockVerifyUserEmailUseCase,
    ) -> web::Data<AppState> {
        web::Data::new(AppState {
            fetch_cv_use_case: Arc::new(StubFetchCVUseCase::default()),
            create_cv_use_case: Arc::new(StubCreateCVUseCase::default()),
            update_cv_use_case: Arc::new(StubUpdateCVUseCase::default()),
            create_user_use_case: Arc::new(create_user_uc),
            verify_user_email_use_case: Arc::new(verify_email_uc),
        })
    }

    // ==================== CREATE USER TESTS ====================
    #[derive(Deserialize, Serialize)]
    struct UserResponse {
        pub id: Uuid,
        pub username: String,
        pub email: String,
        pub password_hash: String,
        pub created_at: DateTime<Utc>,
        pub updated_at: DateTime<Utc>,
        pub is_verified: bool,
        pub is_deleted: bool,
    }
    #[actix_web::test]
    async fn test_create_user_handler_success() {
        let mock_uc = MockCreateUserUseCase::new();
        let verify_uc = MockVerifyUserEmailUseCase::new();

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
        let app_state = create_test_app_state(mock_uc, verify_uc);

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

        let body: UserResponse = test::read_body_json(resp).await;
        assert_eq!(body.username, expected_user.username);
        assert_eq!(body.email, expected_user.email);
    }

    #[actix_web::test]
    async fn test_create_user_handler_username_already_exists() {
        let mock_uc = MockCreateUserUseCase::new();
        let verify_uc = MockVerifyUserEmailUseCase::new();

        mock_uc
            .set_error(CreateUserError::UsernameAlreadyExists)
            .await;
        let app_state = create_test_app_state(mock_uc, verify_uc);

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

        let body = test::read_body(resp).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert_eq!(body_str, "Username already exists");
    }

    #[actix_web::test]
    async fn test_create_user_handler_email_already_exists() {
        let mock_uc = MockCreateUserUseCase::new();
        let verify_uc = MockVerifyUserEmailUseCase::new();

        mock_uc.set_error(CreateUserError::EmailAlreadyExists).await;
        let app_state = create_test_app_state(mock_uc, verify_uc);

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

        let body = test::read_body(resp).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert_eq!(body_str, "Email already exists");
    }

    #[actix_web::test]
    async fn test_create_user_handler_hashing_failed() {
        let mock_uc = MockCreateUserUseCase::new();
        let verify_uc = MockVerifyUserEmailUseCase::new();

        mock_uc
            .set_error(CreateUserError::HashingFailed("bcrypt error".to_string()))
            .await;
        let app_state = create_test_app_state(mock_uc, verify_uc);

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

        let body = test::read_body(resp).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("Password hashing failed"));
    }

    #[actix_web::test]
    async fn test_create_user_handler_repository_error() {
        let mock_uc = MockCreateUserUseCase::new();
        let verify_uc = MockVerifyUserEmailUseCase::new();

        mock_uc
            .set_error(CreateUserError::RepositoryError(
                "Database connection failed".to_string(),
            ))
            .await;
        let app_state = create_test_app_state(mock_uc, verify_uc);

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

        let body = test::read_body(resp).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("Database error"));
    }

    // ==================== VERIFY EMAIL TESTS ====================

    #[actix_web::test]
    async fn test_verify_user_email_handler_success() {
        let create_uc = MockCreateUserUseCase::new();
        let mock_uc = MockVerifyUserEmailUseCase::new();

        mock_uc.set_success().await;
        let app_state = create_test_app_state(create_uc, mock_uc);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(verify_user_email_handler),
        )
        .await;

        let req = test::TestRequest::post()
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
        let mock_uc = MockVerifyUserEmailUseCase::new();

        mock_uc.set_error(VerifyUserEmailError::TokenExpired).await;
        let app_state = create_test_app_state(create_uc, mock_uc);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(verify_user_email_handler),
        )
        .await;

        let req = test::TestRequest::post()
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
        let mock_uc = MockVerifyUserEmailUseCase::new();

        mock_uc.set_error(VerifyUserEmailError::TokenInvalid).await;
        let app_state = create_test_app_state(create_uc, mock_uc);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(verify_user_email_handler),
        )
        .await;

        let req = test::TestRequest::post()
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
        let mock_uc = MockVerifyUserEmailUseCase::new();

        mock_uc.set_error(VerifyUserEmailError::UserNotFound).await;
        let app_state = create_test_app_state(create_uc, mock_uc);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(verify_user_email_handler),
        )
        .await;

        let req = test::TestRequest::post()
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
        let mock_uc = MockVerifyUserEmailUseCase::new();

        mock_uc.set_error(VerifyUserEmailError::DatabaseError).await;
        let app_state = create_test_app_state(create_uc, mock_uc);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(verify_user_email_handler),
        )
        .await;

        let req = test::TestRequest::post()
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
        let mock_uc = MockVerifyUserEmailUseCase::new();

        let app_state = create_test_app_state(create_uc, mock_uc);

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
}
