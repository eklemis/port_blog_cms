//! Editing, restoring, purging and inspecting the usage of media.

use actix_web::{delete, get, patch, post, web, Responder};
use serde::Deserialize;
use tracing::error;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::api::schemas::{ErrorResponse, SuccessResponse};
use crate::auth::adapter::incoming::web::extractors::auth::VerifiedUser;
use crate::auth::application::domain::entities::UserId;
use crate::multimedia::application::ports::incoming::use_cases::{MediaLifecycleError, MediaUsage};
use crate::multimedia::application::ports::outgoing::db::PatchAttachmentData;
use crate::shared::api::{ApiResponse, ErrorCode};
use crate::AppState;

/// A partial update to a media item's attachment metadata.
///
/// Tri-state, matching the patch DTOs in `blog` and `project`: omit a key to
/// leave the field alone, send `null` to clear it, send a value to replace it.
/// Only the attachment metadata is mutable — not the file, its MIME type or its
/// dimensions, which describe bytes already in the bucket.
#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct PatchMediaRequest {
    /// New alternative text. `null` clears it.
    #[serde(default, deserialize_with = "double_option")]
    #[schema(example = "Corrected description")]
    pub alt_text: Option<Option<String>>,

    /// New caption. `null` clears it.
    #[serde(default, deserialize_with = "double_option")]
    pub caption: Option<Option<String>>,

    /// New display position within the role, from 0.
    #[serde(default)]
    #[schema(example = 2)]
    pub position: Option<i32>,
}

/// Distinguishes "key absent" from "key present and null".
///
/// `Option<Option<T>>` alone is not enough: serde deserialises both a missing
/// key and an explicit `null` to `None` unless the field is annotated. With
/// `#[serde(default)]` plus this, absent stays `None` and `null` becomes
/// `Some(None)`.
fn double_option<'de, D, T>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    serde::Deserialize::deserialize(de).map(Some)
}

impl From<PatchMediaRequest> for PatchAttachmentData {
    fn from(r: PatchMediaRequest) -> Self {
        Self {
            alt_text: r.alt_text,
            caption: r.caption,
            position: r.position,
        }
    }
}

fn map_err(e: MediaLifecycleError) -> actix_web::HttpResponse {
    match e {
        MediaLifecycleError::NotFound => {
            ApiResponse::not_found(ErrorCode::MediaNotFound, "Media not found")
        }
        MediaLifecycleError::RepositoryError(msg) => {
            error!("Media lifecycle operation failed: {}", msg);
            ApiResponse::internal_error()
        }
    }
}

/// Correct a media item's attachment metadata
///
/// Alt text, caption and position are set at upload and were not editable, so
/// a missing or wrong alt text was a permanent accessibility defect and a
/// gallery could not be reordered without re-uploading every image.
#[utoipa::path(
    patch,
    path = "/api/media/{media_id}",
    tag = "media",
    request_body = PatchMediaRequest,
    params(("media_id" = Uuid, Path, description = "Media identifier")),
    responses(
        (status = 204, description = "Updated"),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 404, description = "Unknown, or owned by another user", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[patch("/api/media/{media_id}")]
pub async fn patch_media_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    body: web::Json<PatchMediaRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let owner = UserId::from(user.user_id);
    match data
        .multimedia
        .patch_media
        .execute(owner, path.into_inner(), body.into_inner().into())
        .await
    {
        Ok(()) => ApiResponse::<()>::no_content(),
        Err(e) => map_err(e),
    }
}

/// Restore a soft-deleted media item
///
/// `DELETE /api/media/{id}` has always been a soft delete; this is the way
/// back, which did not previously exist.
#[utoipa::path(
    post,
    path = "/api/media/{media_id}/restore",
    tag = "media",
    params(("media_id" = Uuid, Path, description = "Media identifier")),
    responses(
        (status = 204, description = "Restored. Idempotent — restoring a live item succeeds."),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 404, description = "Unknown, or owned by another user", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[post("/api/media/{media_id}/restore")]
pub async fn restore_media_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> impl Responder {
    let owner = UserId::from(user.user_id);
    match data
        .multimedia
        .restore_media
        .execute(owner, path.into_inner())
        .await
    {
        Ok(()) => ApiResponse::<()>::no_content(),
        Err(e) => map_err(e),
    }
}

/// Permanently remove a media item
///
/// Deletes the media, attachment and variant rows. **The stored objects are
/// not removed** — reclaiming those is the bucket's lifecycle policy, so this
/// is not a way to make bytes unreachable in a hurry.
///
/// Check `GET /api/media/{id}/usage` first: this will happily remove an image
/// that is on a live page.
#[utoipa::path(
    delete,
    path = "/api/media/{media_id}/hard",
    tag = "media",
    params(("media_id" = Uuid, Path, description = "Media identifier")),
    responses(
        (status = 204, description = "Deleted permanently"),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 404, description = "Unknown, or owned by another user", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[delete("/api/media/{media_id}/hard")]
pub async fn hard_delete_media_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> impl Responder {
    let owner = UserId::from(user.user_id);
    match data
        .multimedia
        .hard_delete_media
        .execute(owner, path.into_inner())
        .await
    {
        Ok(()) => ApiResponse::<()>::no_content(),
        Err(e) => map_err(e),
    }
}

/// Report where a media item is used
///
/// Answers the question a delete confirmation needs to ask. `is_published` is
/// what earns the endpoint: "used on 3 posts" is mildly useful, "used on a post
/// that is live right now" is what stops someone breaking their own page.
///
/// An unused item returns an empty list, not a 404.
#[utoipa::path(
    get,
    path = "/api/media/{media_id}/usage",
    tag = "media",
    params(("media_id" = Uuid, Path, description = "Media identifier")),
    responses(
        (
            status = 200,
            description = "Where the media is attached",
            body = inline(SuccessResponse<Vec<MediaUsage>>),
            example = json!({
                "success": true,
                "data": [{
                    "target": "blog_post",
                    "target_id": "8f1b2c3d-4e5f-6a7b-8c9d-0e1f2a3b4c5d",
                    "role": "cover",
                    "is_published": true
                }]
            })
        ),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 404, description = "Unknown, or owned by another user", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[get("/api/media/{media_id}/usage")]
pub async fn get_media_usage_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> impl Responder {
    let owner = UserId::from(user.user_id);
    match data
        .multimedia
        .get_media_usage
        .execute(owner, path.into_inner())
        .await
    {
        Ok(usage) => ApiResponse::success(usage),
        Err(e) => map_err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tri-state is the whole contract of this DTO, and it is easy to break
    /// by dropping the `deserialize_with`: serde would then fold an explicit
    /// `null` into the same `None` as an absent key, silently turning "clear
    /// this" into "leave it alone".
    #[test]
    fn an_absent_key_and_an_explicit_null_deserialise_differently() {
        let absent: PatchMediaRequest = serde_json::from_str(r#"{}"#).unwrap();
        assert!(absent.alt_text.is_none(), "absent must stay None");

        let null: PatchMediaRequest = serde_json::from_str(r#"{"alt_text": null}"#).unwrap();
        assert_eq!(
            null.alt_text,
            Some(None),
            "an explicit null must survive as Some(None), meaning 'clear it'"
        );

        let set: PatchMediaRequest = serde_json::from_str(r#"{"alt_text": "hi"}"#).unwrap();
        assert_eq!(set.alt_text, Some(Some("hi".to_string())));
    }

    #[test]
    fn the_three_states_survive_conversion_to_the_port_type() {
        let req: PatchMediaRequest =
            serde_json::from_str(r#"{"alt_text": "new", "caption": null, "position": 2}"#).unwrap();
        let data: PatchAttachmentData = req.into();

        assert_eq!(data.alt_text, Some(Some("new".into())), "set");
        assert_eq!(data.caption, Some(None), "cleared");
        assert_eq!(data.position, Some(2));
        assert!(!data.is_empty());
    }

    /// An empty body is a no-op rather than an error — the repository checks
    /// the item exists and returns without writing, so `updated_at` is not
    /// bumped for a request that changed nothing.
    /// The parsing rule that makes a poll robust: a client whose set contains
    /// a deleted or malformed id should still get answers for the rest.
    /// Erroring the whole batch would make one bad id blind the grid.
    #[test]
    fn unparseable_ids_are_skipped_rather_than_failing_the_batch() {
        let good = uuid::Uuid::new_v4();
        let raw = format!("{good}, not-a-uuid, ,{good}");

        let parsed: Vec<uuid::Uuid> = raw
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse::<uuid::Uuid>().ok())
            .collect();

        assert_eq!(parsed, vec![good, good], "the valid ids must survive");
    }

    #[test]
    fn the_batch_size_is_bounded() {
        assert_eq!(
            MAX_STATUS_IDS, 100,
            "a change here widens how large an IN list one request can build"
        );
    }

    #[test]
    fn an_empty_patch_is_recognised_as_empty() {
        let req: PatchMediaRequest = serde_json::from_str(r#"{}"#).unwrap();
        let data: PatchAttachmentData = req.into();

        assert!(data.is_empty());
    }
}

/// The ids to poll.
#[derive(Debug, Deserialize, utoipa::IntoParams, ToSchema)]
pub struct MediaStatusQuery {
    /// Comma-separated media ids.
    #[param(example = "a1b2…,c3d4…")]
    pub ids: String,
}

/// How many ids one call may ask about.
///
/// Bounded so a single request cannot turn into an unbounded `IN` list. Well
/// past a realistic upload grid; a client with more than this to poll should
/// page rather than widen the request.
const MAX_STATUS_IDS: usize = 100;

/// Poll several media items' processing state at once
///
/// A grid with twelve uploads in flight otherwise polls twelve times every two
/// seconds, per client. This collapses that into one call.
///
/// **Ids that do not resolve are absent from the response rather than an
/// error** — a client polling a set should not lose the whole batch because
/// one item was deleted between polls, and can treat an absent id as gone.
///
/// Unparseable ids are skipped for the same reason. An empty or all-invalid
/// list returns an empty array.
#[utoipa::path(
    get,
    path = "/api/media/statuses",
    tag = "media",
    params(MediaStatusQuery),
    responses(
        (
            status = 200,
            description = "States for the ids that resolved",
            body = inline(SuccessResponse<Vec<crate::multimedia::application::ports::incoming::use_cases::MediaStatus>>)
        ),
        (status = 400, description = "Too many ids", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[get("/api/media/statuses")]
pub async fn get_media_statuses_handler(
    user: VerifiedUser,
    query: web::Query<MediaStatusQuery>,
    data: web::Data<AppState>,
) -> impl Responder {
    let owner = UserId::from(user.user_id);

    let ids: Vec<Uuid> = query
        .ids
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<Uuid>().ok())
        .collect();

    if ids.len() > MAX_STATUS_IDS {
        return ApiResponse::bad_request(ErrorCode::InvalidRequest, "Too many ids in one request");
    }

    match data.multimedia.get_media_statuses.execute(owner, ids).await {
        Ok(statuses) => ApiResponse::success(statuses),
        Err(e) => map_err(e),
    }
}
