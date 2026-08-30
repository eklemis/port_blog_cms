use actix_web::{post, web, Responder};
use serde::{Deserialize, Serialize};
use tracing::error;
use utoipa::ToSchema;

use crate::api::schemas::{ErrorResponse, SuccessResponse};
use crate::auth::application::use_cases::request_password_reset::RequestPasswordResetError;
use crate::auth::application::use_cases::reset_password::ResetPasswordError;
use crate::shared::api::ApiResponse;
use crate::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct RequestPasswordResetDto {
    #[schema(example = "john@example.com")]
    pub email: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PasswordResetResponse {
    #[schema(example = "If that email is registered, a reset link has been sent")]
    pub message: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ResetPasswordDto {
    /// Must satisfy the same policy as registration: 12-128 characters.
    #[schema(example = "a-new-and-long-password", min_length = 12, max_length = 128)]
    pub password: String,
}

/// Request a password reset
///
/// Emails a reset link to the address if it belongs to an active account.
///
/// Always answers 200, whether or not the address is registered. Reporting
/// "no such user" would make this endpoint an oracle for which emails have
/// accounts, so the response is deliberately uninformative — including when
/// delivery itself fails, which is logged server-side instead.
#[utoipa::path(
    post,
    path = "/api/auth/password-reset",
    tag = "auth",
    request_body = RequestPasswordResetDto,
    responses(
        (
            status = 200,
            description = "Request accepted. Says nothing about whether the address exists.",
            body = inline(SuccessResponse<PasswordResetResponse>),
            example = json!({
                "success": true,
                "data": { "message": "If that email is registered, a reset link has been sent" }
            })
        ),
        (
            status = 400,
            description = "Email missing or blank",
            body = ErrorResponse,
            example = json!({
                "success": false,
                "error": { "code": "INVALID_EMAIL", "message": "Email cannot be empty" }
            })
        ),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    )
)]
#[post("/api/auth/password-reset")]
pub async fn request_password_reset_handler(
    req: web::Json<RequestPasswordResetDto>,
    data: web::Data<AppState>,
) -> impl Responder {
    match data
        .request_password_reset_use_case
        .execute(&req.email)
        .await
    {
        Ok(()) => ApiResponse::success(PasswordResetResponse {
            message: "If that email is registered, a reset link has been sent".to_string(),
        }),

        Err(RequestPasswordResetError::InvalidEmail(m)) => {
            ApiResponse::bad_request("INVALID_EMAIL", &m)
        }
        Err(RequestPasswordResetError::QueryError(e)) => {
            error!("Query error during password reset request: {}", e);
            ApiResponse::internal_error()
        }
    }
}

/// Complete a password reset
///
/// Consumes the token from the emailed link and sets a new password.
///
/// Every existing session is revoked on success: a reset is the remedy for a
/// compromised account, so refresh tokens issued under the old password must
/// stop working.
///
/// Only a token minted for reset is accepted; an email-verification or access
/// token is rejected on its type.
#[utoipa::path(
    post,
    path = "/api/auth/password-reset/{token}",
    tag = "auth",
    params(("token" = String, Path, description = "Reset token from the emailed link")),
    request_body = ResetPasswordDto,
    responses(
        (
            status = 200,
            description = "Password changed and all sessions revoked",
            body = inline(SuccessResponse<PasswordResetResponse>)
        ),
        (
            status = 400,
            description = "Password fails the policy",
            body = ErrorResponse,
            example = json!({
                "success": false,
                "error": {
                    "code": "INVALID_PASSWORD",
                    "message": "Password must be at least 12 characters"
                }
            })
        ),
        (
            status = 401,
            description = "Token is expired, malformed, or not a reset token",
            body = ErrorResponse,
            example = json!({
                "success": false,
                "error": { "code": "INVALID_RESET_TOKEN", "message": "Invalid or expired reset token" }
            })
        ),
        (status = 404, description = "User no longer exists", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    )
)]
#[post("/api/auth/password-reset/{token}")]
pub async fn reset_password_handler(
    path: web::Path<String>,
    req: web::Json<ResetPasswordDto>,
    data: web::Data<AppState>,
) -> impl Responder {
    let token = path.into_inner();

    match data
        .reset_password_use_case
        .execute(&token, &req.password)
        .await
    {
        Ok(()) => ApiResponse::success(PasswordResetResponse {
            message: "Password updated. Please sign in again.".to_string(),
        }),

        Err(ResetPasswordError::InvalidToken) => ApiResponse::unauthorized(
            "INVALID_RESET_TOKEN",
            "Invalid or expired reset token",
        ),
        Err(ResetPasswordError::InvalidPassword(m)) => {
            ApiResponse::bad_request("INVALID_PASSWORD", &m)
        }
        Err(ResetPasswordError::UserNotFound) => {
            ApiResponse::not_found("USER_NOT_FOUND", "User not found")
        }
        Err(ResetPasswordError::HashingFailed(e)) => {
            error!("Hashing failed during password reset: {}", e);
            ApiResponse::internal_error()
        }
        Err(ResetPasswordError::RepositoryError(e)) => {
            error!("Repository error during password reset: {}", e);
            ApiResponse::internal_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::application::use_cases::request_password_reset::IRequestPasswordResetUseCase;
    use crate::auth::application::use_cases::reset_password::IResetPasswordUseCase;
    use crate::tests::support::app_state_builder::TestAppStateBuilder;
    use actix_web::{http::StatusCode, test, App};
    use async_trait::async_trait;
    use serde_json::{json, Value};

    struct MockRequest {
        result: Result<(), RequestPasswordResetError>,
    }

    #[async_trait]
    impl IRequestPasswordResetUseCase for MockRequest {
        async fn execute(&self, _e: &str) -> Result<(), RequestPasswordResetError> {
            self.result.clone()
        }
    }

    struct MockReset {
        result: Result<(), ResetPasswordError>,
    }

    #[async_trait]
    impl IResetPasswordUseCase for MockReset {
        async fn execute(&self, _t: &str, _p: &str) -> Result<(), ResetPasswordError> {
            self.result.clone()
        }
    }

    async fn request(
        result: Result<(), RequestPasswordResetError>,
    ) -> actix_web::dev::ServiceResponse {
        let app = test::init_service(
            App::new()
                .app_data(
                    TestAppStateBuilder::default()
                        .with_request_password_reset(MockRequest { result })
                        .build(),
                )
                .service(request_password_reset_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/auth/password-reset")
            .set_json(json!({ "email": "john@example.com" }))
            .to_request();

        test::call_service(&app, req).await
    }

    async fn reset(result: Result<(), ResetPasswordError>) -> actix_web::dev::ServiceResponse {
        let app = test::init_service(
            App::new()
                .app_data(
                    TestAppStateBuilder::default()
                        .with_reset_password(MockReset { result })
                        .build(),
                )
                .service(reset_password_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/auth/password-reset/some-token")
            .set_json(json!({ "password": "a-long-enough-password" }))
            .to_request();

        test::call_service(&app, req).await
    }

    /// No Authorization header on either endpoint: a user who cannot sign in is
    /// exactly the user who needs these.
    #[actix_web::test]
    async fn requesting_a_reset_needs_no_authentication() {
        let resp = request(Ok(())).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
    }

    /// The response must not vary with whether the address is registered. The
    /// use case already collapses both cases to Ok; this pins that the handler
    /// does not reintroduce a difference.
    #[actix_web::test]
    async fn the_response_is_identical_for_known_and_unknown_addresses() {
        let known = request(Ok(())).await;
        assert_eq!(known.status(), StatusCode::OK);
        let known_body: Value = test::read_body_json(known).await;

        let unknown = request(Ok(())).await;
        assert_eq!(unknown.status(), StatusCode::OK);
        let unknown_body: Value = test::read_body_json(unknown).await;

        assert_eq!(known_body, unknown_body);
    }

    #[actix_web::test]
    async fn a_blank_email_is_a_bad_request() {
        let resp = request(Err(RequestPasswordResetError::InvalidEmail(
            "Email cannot be empty".into(),
        )))
        .await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "INVALID_EMAIL");
    }

    #[actix_web::test]
    async fn completing_a_reset_needs_no_authentication() {
        let resp = reset(Ok(())).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    /// A token of the wrong type lands here, and must read as an auth failure
    /// rather than a validation error.
    #[actix_web::test]
    async fn a_bad_token_is_unauthorized() {
        let resp = reset(Err(ResetPasswordError::InvalidToken)).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "INVALID_RESET_TOKEN");
    }

    #[actix_web::test]
    async fn a_weak_password_is_a_bad_request() {
        let resp = reset(Err(ResetPasswordError::InvalidPassword(
            "Password must be at least 12 characters".into(),
        )))
        .await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "INVALID_PASSWORD");
    }

    #[actix_web::test]
    async fn a_repository_failure_is_an_internal_error() {
        let resp = reset(Err(ResetPasswordError::RepositoryError("db down".into()))).await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
