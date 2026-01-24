use crate::auth::application::use_cases::verify_user_email::VerifyUserEmailError;

use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse, Responder};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::application::use_cases::verify_user_email::{
        IVerifyUserEmailUseCase, VerifyUserEmailError,
    };
    use crate::tests::support::app_state_builder::TestAppStateBuilder;
    use actix_web::{test, App};
    use async_trait::async_trait;

    // ========================================================================
    // Mock Use Cases for Different Scenarios
    // ========================================================================

    #[derive(Clone)]
    struct MockVerifyUserEmailSuccess;

    #[async_trait]
    impl IVerifyUserEmailUseCase for MockVerifyUserEmailSuccess {
        async fn execute(&self, _token: &str) -> Result<(), VerifyUserEmailError> {
            Ok(())
        }
    }

    #[derive(Clone)]
    struct MockVerifyUserEmailTokenExpired;

    #[async_trait]
    impl IVerifyUserEmailUseCase for MockVerifyUserEmailTokenExpired {
        async fn execute(&self, _token: &str) -> Result<(), VerifyUserEmailError> {
            Err(VerifyUserEmailError::TokenExpired)
        }
    }

    #[derive(Clone)]
    struct MockVerifyUserEmailTokenInvalid;

    #[async_trait]
    impl IVerifyUserEmailUseCase for MockVerifyUserEmailTokenInvalid {
        async fn execute(&self, _token: &str) -> Result<(), VerifyUserEmailError> {
            Err(VerifyUserEmailError::TokenInvalid)
        }
    }

    #[derive(Clone)]
    struct MockVerifyUserEmailUserNotFound;

    #[async_trait]
    impl IVerifyUserEmailUseCase for MockVerifyUserEmailUserNotFound {
        async fn execute(&self, _token: &str) -> Result<(), VerifyUserEmailError> {
            Err(VerifyUserEmailError::UserNotFound)
        }
    }

    #[derive(Clone)]
    struct MockVerifyUserEmailDatabaseError;

    #[async_trait]
    impl IVerifyUserEmailUseCase for MockVerifyUserEmailDatabaseError {
        async fn execute(&self, _token: &str) -> Result<(), VerifyUserEmailError> {
            Err(VerifyUserEmailError::DatabaseError)
        }
    }

    // ========================================================================
    // Tests
    // ========================================================================

    #[actix_web::test]
    async fn test_verify_user_email_success() {
        let app_state = TestAppStateBuilder::default()
            .with_verify_user_email(MockVerifyUserEmailSuccess)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(verify_user_email_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/auth/email-verification/valid-token-12345")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["message"], "Email verified successfully");
    }

    #[actix_web::test]
    async fn test_verify_user_email_token_expired() {
        let app_state = TestAppStateBuilder::default()
            .with_verify_user_email(MockVerifyUserEmailTokenExpired)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(verify_user_email_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/auth/email-verification/expired-token-12345")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "Token has expired");
    }

    #[actix_web::test]
    async fn test_verify_user_email_token_invalid() {
        let app_state = TestAppStateBuilder::default()
            .with_verify_user_email(MockVerifyUserEmailTokenInvalid)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(verify_user_email_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/auth/email-verification/invalid-token-12345")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "Invalid token");
    }

    #[actix_web::test]
    async fn test_verify_user_email_user_not_found() {
        let app_state = TestAppStateBuilder::default()
            .with_verify_user_email(MockVerifyUserEmailUserNotFound)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(verify_user_email_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/auth/email-verification/nonexistent-user-token")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "User not found");
    }

    #[actix_web::test]
    async fn test_verify_user_email_database_error() {
        let app_state = TestAppStateBuilder::default()
            .with_verify_user_email(MockVerifyUserEmailDatabaseError)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(verify_user_email_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/auth/email-verification/some-token-12345")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 500);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "Internal server error");
    }

    #[actix_web::test]
    async fn test_verify_user_email_with_special_characters_in_token() {
        let app_state = TestAppStateBuilder::default()
            .with_verify_user_email(MockVerifyUserEmailSuccess)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(verify_user_email_handler),
        )
        .await;

        // Test token with URL-safe base64 characters
        let req = test::TestRequest::get()
            .uri("/api/auth/email-verification/eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["message"], "Email verified successfully");
    }

    #[actix_web::test]
    async fn test_verify_user_email_with_long_token() {
        let app_state = TestAppStateBuilder::default()
            .with_verify_user_email(MockVerifyUserEmailSuccess)
            .build();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(verify_user_email_handler),
        )
        .await;

        // Test with a very long token (simulating JWT)
        let long_token = "a".repeat(500);
        let uri = format!("/api/auth/email-verification/{}", long_token);

        let req = test::TestRequest::get().uri(&uri).to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["message"], "Email verified successfully");
    }
}
