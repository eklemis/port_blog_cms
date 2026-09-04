//! `GET /api/ai/quota` — how much generation is left this period.

use actix_web::{get, web, Responder};
use tracing::error;

use crate::{
    ai::application::ports::incoming::use_cases::QuotaError,
    ai::domain::quota::QuotaState,
    api::schemas::{ErrorResponse, SuccessResponse},
    auth::{
        adapter::incoming::web::extractors::auth::VerifiedUser,
        application::domain::entities::UserId,
    },
    shared::api::ApiResponse,
    AppState,
};

/// Read your generation allowance
///
/// **`limit` is `null` when generation is currently unmetered**, and `used` is
/// counted either way. That is deliberate: the number a sensible ceiling gets
/// chosen from should be real usage rather than a guess, and the screen that
/// shows a remaining count is far cheaper to build now than to retrofit onto
/// screens designed on the assumption that calls are free.
///
/// So render the remaining count when a limit exists and stay quiet when it
/// does not — but build the surface either way.
///
/// Reading this never refuses on account of the limit. Someone with nothing
/// left is exactly who looks.
#[utoipa::path(
    get,
    path = "/api/ai/quota",
    tag = "ai",
    responses(
        (
            status = 200,
            description = "Your standing for the current period",
            body = inline(SuccessResponse<QuotaState>),
            example = json!({
                "success": true,
                "data": { "used": 12, "limit": null, "resets_at": "2026-10-01T00:00:00Z" }
            })
        ),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
        (status = 503, description = "The counter could not be reached", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[get("/api/ai/quota")]
pub async fn get_ai_quota_handler(user: VerifiedUser, data: web::Data<AppState>) -> impl Responder {
    match data.ai.get_quota.execute(UserId::from(user.user_id)).await {
        Ok(state) => ApiResponse::success(state),
        Err(QuotaError::Unavailable(e)) => {
            error!("AI quota counter unavailable: {}", e);
            ApiResponse::internal_error()
        }
        // Reading a quota does not spend one, so this branch is unreachable
        // through this route. Matched rather than caught by a wildcard so
        // adding a variant to QuotaError fails the build here.
        Err(QuotaError::Exceeded(_)) => ApiResponse::internal_error(),
    }
}
