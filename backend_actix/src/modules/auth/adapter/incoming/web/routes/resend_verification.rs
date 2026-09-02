//! `POST /api/auth/email-verification/resend`.

use actix_web::{http::StatusCode, post, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use tracing::error;
use utoipa::ToSchema;

use crate::api::schemas::{ErrorResponse, SuccessResponse};
use crate::auth::application::use_cases::resend_verification_email::ResendVerificationEmailError;
use crate::shared::api::{ApiResponse, ErrorCode};
use crate::AppState;

/// Request or response shape for the HTTP layer.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ResendVerificationDto {
    /// The address to send a fresh verification link to.
    #[schema(example = "jane@example.com")]
    pub email: String,
}

/// Response body returned by this endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct ResendVerificationResponse {
    /// Deliberately non-committal: the same text comes back whether or not the
    /// address needed anything doing.
    #[schema(example = "If that address needs verifying, a new link is on its way.")]
    pub message: String,
}

/// Resend the email-verification link
///
/// Registration mails the link once and the token expires after
/// `JWT_VERIFICATION_EXPIRY`. Without this endpoint, an account whose owner
/// took a day to check their mail was permanently unusable: re-registering the
/// same address answers `USER_ALREADY_EXISTS`.
///
/// Always answers `202`, with the same body, whether the address is unknown,
/// deleted, already verified, or genuinely needed a new link — and also when
/// sending fails, which is logged server-side. Anything else would make this an
/// oracle for which addresses are registered and which are confirmed.
///
/// `202` rather than `200` because that is what the response means: the request
/// was accepted, and nothing about what followed is being reported.
///
/// Rate-limited at 5 per hour per caller, matching `password-reset` — each call
/// can cost a token mint and an outbound mail.
#[utoipa::path(
    post,
    path = "/api/auth/email-verification/resend",
    tag = "auth",
    request_body = ResendVerificationDto,
    responses(
        (
            status = 202,
            description = "Accepted. Says nothing about whether the address exists or needed verifying.",
            body = inline(SuccessResponse<ResendVerificationResponse>),
            example = json!({
                "success": true,
                "data": { "message": "If that address needs verifying, a new link is on its way." }
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
        (status = 429, description = "Rate limit exceeded", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    )
)]
#[post("/api/auth/email-verification/resend")]
pub async fn resend_verification_handler(
    req: web::Json<ResendVerificationDto>,
    data: web::Data<AppState>,
) -> impl Responder {
    match data
        .resend_verification_email_use_case
        .execute(&req.email)
        .await
    {
        Ok(()) => HttpResponse::build(StatusCode::ACCEPTED).json(SuccessResponse {
            success: true,
            data: ResendVerificationResponse {
                message: "If that address needs verifying, a new link is on its way.".to_string(),
            },
        }),
        Err(ResendVerificationEmailError::InvalidEmail(msg)) => {
            ApiResponse::bad_request(ErrorCode::InvalidEmail, &msg)
        }
        Err(ResendVerificationEmailError::QueryError(e)) => {
            error!(
                "Failed to look up an account for verification resend: {}",
                e
            );
            ApiResponse::internal_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::application::use_cases::resend_verification_email::IResendVerificationEmailUseCase;
    use crate::tests::support::app_state_builder::TestAppStateBuilder;
    use actix_web::{test, App};
    use async_trait::async_trait;

    struct Uc(Result<(), ResendVerificationEmailError>);

    #[async_trait]
    impl IResendVerificationEmailUseCase for Uc {
        async fn execute(&self, _email: &str) -> Result<(), ResendVerificationEmailError> {
            self.0.clone()
        }
    }

    async fn call(uc: Uc, body: serde_json::Value) -> actix_web::dev::ServiceResponse {
        let state = TestAppStateBuilder::default()
            .with_resend_verification_email(uc)
            .build();
        let app = test::init_service(
            App::new()
                .app_data(state)
                .service(resend_verification_handler),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/api/auth/email-verification/resend")
            .set_json(&body)
            .to_request();
        test::call_service(&app, req).await
    }

    /// 202, not 200: the response reports that the request was accepted and
    /// says nothing about what followed.
    #[actix_web::test]
    async fn a_successful_request_is_accepted_with_202() {
        let resp = call(Uc(Ok(())), serde_json::json!({"email": "jane@example.com"})).await;

        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert!(
            body["data"]["message"]
                .as_str()
                .unwrap()
                .contains("needs verifying"),
            "the message must stay non-committal: {body}"
        );
    }

    #[actix_web::test]
    async fn a_blank_address_is_a_400() {
        let resp = call(
            Uc(Err(ResendVerificationEmailError::InvalidEmail(
                "Email cannot be empty".into(),
            ))),
            serde_json::json!({"email": ""}),
        )
        .await;

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "INVALID_EMAIL");
    }

    #[actix_web::test]
    async fn a_store_failure_is_a_500_and_leaks_nothing() {
        let resp = call(
            Uc(Err(ResendVerificationEmailError::QueryError(
                "connection refused".into(),
            ))),
            serde_json::json!({"email": "jane@example.com"}),
        )
        .await;

        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "INTERNAL_ERROR");
        assert!(
            !body.to_string().contains("connection refused"),
            "the store's error text must not reach the caller: {body}"
        );
    }
}
