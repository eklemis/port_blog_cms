//! `GET /api/blog/summary` — how many posts, by state.

use actix_web::{get, web, Responder};
use tracing::error;

use crate::{
    api::schemas::ErrorResponse, auth::adapter::incoming::web::extractors::auth::VerifiedUser,
    auth::application::domain::entities::UserId,
    blog::application::ports::outgoing::BlogPostCounts, shared::api::ApiResponse, AppState,
};

/// Count the author's posts by state
///
/// The numbers behind a dashboard line like "12 live · 9 drafts · 3 archived".
///
/// Three counts in one round trip. Two of them could be had by listing with a
/// filter and reading `total`, at the cost of two requests that also transfer
/// rows nobody renders; `archived` could not be had at all until the archive
/// listing existed, and even then only as a third request.
///
/// A post scheduled for the future counts as a draft, not as live, matching
/// what the public listing will actually show.
#[utoipa::path(
    get,
    path = "/api/blog/summary",
    tag = "blog",
    responses(
        (status = 200, description = "Counts by state", body = BlogPostCounts),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[get("/api/blog/summary")]
pub async fn get_blog_post_counts_handler(
    user: VerifiedUser,
    data: web::Data<AppState>,
) -> impl Responder {
    match data.blog.counts.execute(UserId::from(user.user_id)).await {
        Ok(counts) => ApiResponse::success(counts),
        Err(e) => {
            error!("Could not count posts: {e}");
            ApiResponse::internal_error()
        }
    }
}
