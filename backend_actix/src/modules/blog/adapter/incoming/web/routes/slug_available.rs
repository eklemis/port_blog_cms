//! `GET /api/blog/slug-available`.

use actix_web::{get, web, Responder};
use serde::Deserialize;
use tracing::error;
use utoipa::{IntoParams, ToSchema};

use crate::api::schemas::{ErrorResponse, SuccessResponse};
use crate::auth::adapter::incoming::web::extractors::auth::VerifiedUser;
use crate::auth::application::domain::entities::UserId;
use crate::shared::api::{
    normalize_slug, suggest_free_slug, ApiResponse, ErrorCode, SlugAvailability,
};
use crate::AppState;

/// The slug to check.
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct SlugQuery {
    /// Candidate slug, before normalisation.
    #[param(example = "building-a-cms")]
    pub slug: String,
}

/// Check whether a blog slug is free
///
/// Collisions previously surfaced only at save, as `SLUG_ALREADY_EXISTS`, and
/// search covers title, excerpt and content rather than slug — so there was no
/// way to ask ahead, and an editor could not suggest a free variant without
/// risking that its own suggestion was taken.
///
/// **Scoped to the authenticated author**, because slugs are unique per author:
/// another user holding `building-a-cms` does not make it unavailable to you.
///
/// The suggestion is checked against the database rather than guessed, so it
/// cannot itself collide.
#[utoipa::path(
    get,
    path = "/api/blog/slug-available",
    tag = "blog",
    params(SlugQuery),
    responses(
        (
            status = 200,
            description = "Availability, with a free variant when taken",
            body = inline(SuccessResponse<SlugAvailability>),
            example = json!({
                "success": true,
                "data": {
                    "slug": "building-a-cms",
                    "available": false,
                    "suggestion": "building-a-cms-2"
                }
            })
        ),
        (status = 400, description = "Slug missing or blank", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[get("/api/blog/slug-available")]
pub async fn blog_slug_available_handler(
    user: VerifiedUser,
    query: web::Query<SlugQuery>,
    data: web::Data<AppState>,
) -> impl Responder {
    let owner = UserId::from(user.user_id);
    let slug = normalize_slug(&query.slug);

    if slug.is_empty() {
        return ApiResponse::bad_request(ErrorCode::InvalidSlug, "Slug cannot be empty");
    }

    let taken = match data.blog.slug_available.execute(owner, slug.clone()).await {
        Ok(taken) => taken,
        Err(e) => {
            error!("Failed to check blog slug availability: {}", e);
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

    let suggestion = suggest_free_slug(&slug, |candidate| {
        let uc = data.blog.slug_available.clone();
        async move { uc.execute(owner, candidate).await }
    })
    .await;

    match suggestion {
        Ok(suggestion) => ApiResponse::success(SlugAvailability {
            slug,
            available: false,
            suggestion,
        }),
        Err(e) => {
            error!("Failed to find a free blog slug: {}", e);
            ApiResponse::internal_error()
        }
    }
}
