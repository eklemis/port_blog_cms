//! `PATCH /api/topics/{topic_id}`.

use actix_web::{patch, web, Responder};
use serde::Deserialize;
use tracing::error;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::api::schemas::{ErrorResponse, SuccessResponse};
use crate::auth::adapter::incoming::web::extractors::auth::VerifiedUser;
use crate::auth::application::domain::entities::UserId;
use crate::shared::api::{ApiResponse, ErrorCode};
use crate::topic::application::ports::incoming::use_cases::PatchTopicError;
use crate::topic::application::ports::outgoing::TopicResult;
use crate::AppState;

/// A partial topic edit. Omitted fields are left alone.
#[derive(Debug, Deserialize, ToSchema)]
pub struct PatchTopicRequest {
    /// New title. Trimmed, 1–100 characters, matching creation.
    #[schema(example = "Distributed Systems", min_length = 1, max_length = 100)]
    pub title: Option<String>,

    /// New description.
    #[schema(example = "Notes and projects on consensus and replication")]
    pub description: Option<String>,
}

/// Rename a topic
///
/// Topics supported create, list and soft delete only, so a typo in a title
/// was permanent and visible on every tagged post and project. The workaround
/// was create-retag-retire, by hand.
///
/// The topic keeps its id, so everything tagged with it follows the new name
/// automatically — nothing needs retagging.
#[utoipa::path(
    patch,
    path = "/api/topics/{topic_id}",
    tag = "topics",
    request_body = PatchTopicRequest,
    params(("topic_id" = Uuid, Path, description = "Topic identifier")),
    responses(
        (
            status = 200,
            description = "Updated",
            body = inline(SuccessResponse<TopicResult>)
        ),
        (
            status = 400,
            description = "Title empty or longer than 100 characters",
            body = ErrorResponse
        ),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 404, description = "Unknown, or owned by another user", body = ErrorResponse),
        (
            status = 409,
            description = "The owner already has a topic with that title",
            body = ErrorResponse
        ),
    ),
    security(("BearerAuth" = []))
)]
#[patch("/api/topics/{topic_id}")]
pub async fn patch_topic_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    body: web::Json<PatchTopicRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let owner = UserId::from(user.user_id);
    let body = body.into_inner();

    match data
        .patch_topic_use_case
        .execute(owner, path.into_inner(), body.title, body.description)
        .await
    {
        Ok(topic) => ApiResponse::success(topic),
        Err(PatchTopicError::EmptyTitle) => {
            ApiResponse::bad_request(ErrorCode::EmptyTitle, "Title cannot be empty")
        }
        Err(PatchTopicError::TitleTooLong) => ApiResponse::bad_request(
            ErrorCode::TitleTooLong,
            "Title must not exceed 100 characters",
        ),
        Err(PatchTopicError::TopicNotFound) => {
            ApiResponse::not_found(ErrorCode::TopicNotFound, "Topic not found")
        }
        Err(PatchTopicError::TopicAlreadyExists) => {
            ApiResponse::conflict(ErrorCode::TopicAlreadyExists, "Topic already exists")
        }
        Err(PatchTopicError::RepositoryError(e)) => {
            error!("Failed to patch topic: {}", e);
            ApiResponse::internal_error()
        }
    }
}
