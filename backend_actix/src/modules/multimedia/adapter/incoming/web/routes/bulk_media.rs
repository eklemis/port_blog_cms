//! `POST /api/media/bulk` — one operation applied to many media items.

use actix_web::{post, web, Responder};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    api::schemas::{ErrorResponse, SuccessResponse},
    auth::{
        adapter::incoming::web::extractors::auth::VerifiedUser,
        application::domain::entities::UserId,
    },
    multimedia::application::ports::incoming::use_cases::MediaBulkOp,
    shared::api::{ApiResponse, BulkOutcome},
    AppState,
};

/// Request body: the operation, then the media items to apply it to.
///
/// `op` is flattened into this object, so a request reads
/// `{"op": "archive", "ids": [...]}`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct BulkMediaRequest {
    /// What to do. One of `archive`, `restore`, `hard_delete`. Media carries
    /// no topics, so there are no topic operations here.
    #[serde(flatten)]
    #[schema(value_type = Object)]
    pub op: MediaBulkOp,

    /// The media items to apply it to. Duplicates are collapsed.
    pub ids: Vec<Uuid>,
}

/// Apply one operation to many media items
///
/// A batch is many operations, not one: each media succeeds or fails on its own
/// and the response says which. **A 200 means the batch ran, not that every
/// media succeeded** — read `failed`.
///
/// Items belonging to another author are reported as `MEDIA_NOT_FOUND`, the same
/// as the single-item routes, so this cannot be used to discover or modify
/// anyone else's uploads.
#[utoipa::path(
    post,
    path = "/api/media/bulk",
    tag = "media",
    request_body = BulkMediaRequest,
    responses(
        (
            status = 200,
            description = "The batch ran. Check `failed` for items that did not.",
            body = inline(SuccessResponse<BulkOutcome>),
            example = json!({
                "success": true,
                "data": {
                    "succeeded": ["3f1b2c3d-4e5f-6a7b-8c9d-0e1f2a3b4c5d"],
                    "failed": [{
                        "id": "9a8b7c6d-5e4f-3a2b-1c0d-9e8f7a6b5c4d",
                        "code": "MEDIA_NOT_FOUND",
                        "message": "Media not found"
                    }]
                }
            })
        ),
        (
            status = 400,
            description = "The id list was empty or longer than the cap",
            body = ErrorResponse,
            example = json!({
                "success": false,
                "error": { "code": "BULK_TOO_LARGE", "message": "A bulk request carries at most 100 ids, got 250" }
            })
        ),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[post("/api/media/bulk")]
pub async fn bulk_media_handler(
    user: VerifiedUser,
    body: web::Json<BulkMediaRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let body = body.into_inner();

    match data
        .multimedia
        .bulk
        .execute(UserId::from(user.user_id), body.op, body.ids)
        .await
    {
        Ok(outcome) => ApiResponse::success(outcome),
        Err(e) => ApiResponse::bad_request(e.code(), &e.to_string()),
    }
}
