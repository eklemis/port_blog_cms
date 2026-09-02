//! `GET /api/topics/{topic_id}/usage`.

use actix_web::{get, web, Responder};
use tracing::error;
use uuid::Uuid;

use crate::api::schemas::{ErrorResponse, SuccessResponse};
use crate::auth::adapter::incoming::web::extractors::auth::VerifiedUser;
use crate::auth::application::domain::entities::UserId;
use crate::shared::api::ApiResponse;
use crate::topic::application::ports::incoming::use_cases::GetTopicUsageError;
use crate::topic::application::ports::outgoing::TopicUsage;
use crate::AppState;

/// Count what a topic is attached to
///
/// Answers the question a retire-confirmation needs: "Retire «Rust»? It's on 6
/// posts and 2 projects." Getting that number previously meant fetching every
/// post and project and their topics, so the console either warned generically
/// or invented a figure.
///
/// Counts live rows only — a soft-deleted post is not a reason to keep a topic.
/// An unused, unknown, or someone else's topic all report zeroes rather than
/// 404: this is a number for a dialog, not an existence check.
#[utoipa::path(
    get,
    path = "/api/topics/{topic_id}/usage",
    tag = "topics",
    params(("topic_id" = Uuid, Path, description = "Topic identifier")),
    responses(
        (
            status = 200,
            description = "Reference counts",
            body = inline(SuccessResponse<TopicUsage>),
            example = json!({ "success": true, "data": { "posts": 6, "projects": 2 } })
        ),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[get("/api/topics/{topic_id}/usage")]
pub async fn get_topic_usage_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> impl Responder {
    let owner = UserId::from(user.user_id);
    match data
        .get_topic_usage_use_case
        .execute(owner, path.into_inner())
        .await
    {
        Ok(usage) => ApiResponse::success(usage),
        Err(GetTopicUsageError::QueryFailed(e)) => {
            error!("Failed to count topic usage: {}", e);
            ApiResponse::internal_error()
        }
    }
}
