//! `GET /api/projects/slug-available`.

use actix_web::{get, web, Responder};
use tracing::error;

use crate::api::schemas::{ErrorResponse, SuccessResponse};
use crate::auth::adapter::incoming::web::extractors::auth::VerifiedUser;
use crate::auth::application::domain::entities::UserId;
use crate::blog::adapter::incoming::web::routes::SlugQuery;
use crate::shared::api::{
    normalize_slug, suggest_free_slug, ApiResponse, ErrorCode, SlugAvailability,
};
use crate::AppState;

/// Check whether a project slug is free
///
/// Same shape and semantics as the blog equivalent, and scoped to the
/// authenticated owner for the same reason: the unique index is `(user_id,
/// lower(slug))`, so another author holding a slug does not make it
/// unavailable to you.
///
/// The suggestion is checked against the database rather than guessed.
#[utoipa::path(
    get,
    path = "/api/projects/slug-available",
    tag = "projects",
    params(SlugQuery),
    responses(
        (
            status = 200,
            description = "Availability, with a free variant when taken",
            body = inline(SuccessResponse<SlugAvailability>)
        ),
        (status = 400, description = "Slug missing or blank", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[get("/api/projects/slug-available")]
pub async fn project_slug_available_handler(
    user: VerifiedUser,
    query: web::Query<SlugQuery>,
    data: web::Data<AppState>,
) -> impl Responder {
    let owner = UserId::from(user.user_id);
    let slug = normalize_slug(&query.slug);

    if slug.is_empty() {
        return ApiResponse::bad_request(ErrorCode::InvalidSlug, "Slug cannot be empty");
    }

    let taken = match data
        .project
        .slug_available
        .execute(owner, slug.clone())
        .await
    {
        Ok(taken) => taken,
        Err(e) => {
            error!("Failed to check project slug availability: {}", e);
            return ApiResponse::internal_error();
        }
    };

    if !taken {
        return ApiResponse::success(SlugAvailability {
            slug,
            available: true,
            suggestion: None,
        });
    }

    match suggest_free_slug(&slug, |candidate| {
        let uc = data.project.slug_available.clone();
        async move { uc.execute(owner, candidate).await }
    })
    .await
    {
        Ok(suggestion) => ApiResponse::success(SlugAvailability {
            slug,
            available: false,
            suggestion,
        }),
        Err(e) => {
            error!("Failed to find a free project slug: {}", e);
            ApiResponse::internal_error()
        }
    }
}
