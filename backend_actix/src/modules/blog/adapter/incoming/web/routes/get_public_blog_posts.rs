use actix_web::{get, web, Responder};
use tracing::error;

use crate::{
    api::schemas::{ErrorResponse, SuccessResponse},
    auth::adapter::incoming::web::extractors::auth::resolve_owner_id_or_response,
    auth::application::domain::entities::UserId,
    blog::adapter::incoming::web::dto::BlogPostCardResponse,
    blog::adapter::incoming::web::routes::get_blog_posts::GetBlogPostsQuery,
    blog::application::ports::incoming::use_cases::GetBlogPostsError,
    blog::application::ports::outgoing::BlogPageResult,
    shared::api::ApiResponse,
    AppState,
};

/// List an author's published posts
///
/// Public: no authentication required. Drafts and scheduled posts are never
/// returned, regardless of the `published` query parameter — the public path
/// forces published-only rather than reading it from the request.
#[utoipa::path(
    get,
    path = "/api/public/blog/{username}",
    tag = "blog",
    params(
        ("username" = String, Path, description = "Author whose posts to list"),
        GetBlogPostsQuery
    ),
    responses(
        (
            status = 200,
            description = "Posts retrieved",
            body = inline(SuccessResponse<BlogPageResult<BlogPostCardResponse>>)
        ),
        (
            status = 404,
            description = "No such username",
            body = ErrorResponse,
            example = json!({
                "success": false,
                "error": { "code": "USER_NOT_FOUND", "message": "User not found" }
            })
        ),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    )
)]
#[get("/api/public/blog/{username}")]
pub async fn get_public_blog_posts_handler(
    path: web::Path<String>,
    query: web::Query<GetBlogPostsQuery>,
    data: web::Data<AppState>,
) -> impl Responder {
    let username = path.into_inner();
    let (filter, page, sort) = query.into_inner().into();

    let owner_id = match resolve_owner_id_or_response(&data, &username).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    match data
        .blog
        .list_public
        .execute(UserId::from(owner_id), filter, sort, page)
        .await
    {
        Ok(result) => ApiResponse::success(BlogPageResult {
            items: result.items.into_iter().map(Into::into).collect::<Vec<BlogPostCardResponse>>(),
            page: result.page,
            per_page: result.per_page,
            total: result.total,
        }),
        Err(GetBlogPostsError::QueryFailed(e)) => {
            error!("Failed to list public blog posts: {}", e);
            ApiResponse::internal_error()
        }
    }
}
