//! Sharing a draft with someone who has no account.
//!
//! Three author-facing routes under `/api/blog/{post_id}/preview` — share,
//! read, revoke — and one public route, `GET /api/public/blog/preview/{token}`,
//! that the link itself points at.

use actix_web::{delete, get, http::header, post, web, HttpResponse, Responder};
use serde::Serialize;
use tracing::error;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    api::schemas::{ErrorResponse, SuccessResponse},
    auth::{
        adapter::incoming::web::extractors::auth::VerifiedUser,
        application::domain::entities::UserId,
    },
    blog::adapter::incoming::web::dto::BlogPostDetailResponse,
    blog::application::ports::incoming::use_cases::{
        DraftPreviewError, DraftPreviewState, PreviewResolution,
    },
    shared::api::{ApiResponse, ErrorCode},
    AppState,
};

/// What a reader holding a preview link is served.
#[derive(Debug, Serialize, ToSchema)]
pub struct DraftPreviewResponse {
    /// Always `true`. Present so a client can render the "not published"
    /// banner from the payload rather than from which URL it happened to call
    /// — without it a reviewer cannot tell a draft from the live post, and may
    /// link to it as though it were public.
    pub preview: bool,

    /// The post as it stands right now. Not a snapshot taken when the link was
    /// minted: the author keeps editing after sharing, and a reviewer who
    /// refreshes should see the current draft.
    ///
    /// Flattened, so the payload is exactly what
    /// `GET /api/public/blog/{username}/{slug}` returns with `preview` added.
    /// A client renders a draft with the component it already has.
    #[serde(flatten)]
    pub post: BlogPostDetailResponse,
}

fn map_error(e: DraftPreviewError) -> HttpResponse {
    match e {
        DraftPreviewError::PostNotFound => {
            ApiResponse::not_found(ErrorCode::PostNotFound, "Blog post not found")
        }
        DraftPreviewError::NotShared => {
            ApiResponse::not_found(ErrorCode::PostNotFound, "This post is not shared")
        }
        DraftPreviewError::RepositoryError(e) => {
            error!("Repository error on a draft preview: {}", e);
            ApiResponse::internal_error()
        }
    }
}

/// Share a draft, or extend the link it already has
///
/// Mints a link the holder can read this post with, without an account. Calling
/// it again **renews the same link** rather than minting a new one, so a
/// reviewer's bookmark survives the renewal.
#[utoipa::path(
    post,
    path = "/api/blog/{post_id}/preview",
    tag = "blog",
    params(("post_id" = Uuid, Path, description = "Identifier of the post")),
    responses(
        (
            status = 200,
            description = "The link, and when it expires",
            body = inline(SuccessResponse<DraftPreviewState>),
            example = json!({
                "success": true,
                "data": {
                    "token": "9f8e7d6c…",
                    "expires_at": "2026-09-16T09:00:00Z",
                    "created_at": "2026-09-02T09:00:00Z",
                    "expired": false
                }
            })
        ),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
        (status = 404, description = "Post not found, or owned by another author", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[post("/api/blog/{post_id}/preview")]
pub async fn share_draft_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> impl Responder {
    match data
        .blog_preview
        .share
        .execute(UserId::from(user.user_id), path.into_inner())
        .await
    {
        Ok(state) => ApiResponse::success(state),
        Err(e) => map_error(e),
    }
}

/// Read a post's sharing state
///
/// Backs the sharing panel: the link, when it expires, and whether it already
/// has. An expired link is reported rather than hidden — the author needs to
/// see that it lapsed, which is the difference between a TTL that is safe and
/// one that surprises people.
#[utoipa::path(
    get,
    path = "/api/blog/{post_id}/preview",
    tag = "blog",
    params(("post_id" = Uuid, Path, description = "Identifier of the post")),
    responses(
        (status = 200, description = "The post is shared", body = inline(SuccessResponse<DraftPreviewState>)),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
        (status = 404, description = "The post is not shared, or is not yours", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[get("/api/blog/{post_id}/preview")]
pub async fn get_draft_preview_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> impl Responder {
    match data
        .blog_preview
        .get
        .execute(UserId::from(user.user_id), path.into_inner())
        .await
    {
        Ok(state) => ApiResponse::success(state),
        Err(e) => map_error(e),
    }
}

/// Withdraw a draft's preview link
///
/// The link stops working immediately. Revoking a post that is not shared
/// succeeds — the author ends up where they wanted to be either way.
#[utoipa::path(
    delete,
    path = "/api/blog/{post_id}/preview",
    tag = "blog",
    params(("post_id" = Uuid, Path, description = "Identifier of the post")),
    responses(
        (status = 204, description = "The post is no longer shared"),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
        (status = 404, description = "Post not found, or owned by another author", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[delete("/api/blog/{post_id}/preview")]
pub async fn revoke_draft_preview_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> impl Responder {
    match data
        .blog_preview
        .revoke
        .execute(UserId::from(user.user_id), path.into_inner())
        .await
    {
        Ok(()) => ApiResponse::no_content(),
        Err(e) => map_error(e),
    }
}

/// Read a draft through its preview link
///
/// Public: the token is the authorisation, so no account is needed.
///
/// **If the post has since been published, this redirects to its public page**
/// rather than reporting a dead link. A reviewer opening a bookmark for
/// something the world can now read should land on the real thing, not on an
/// error.
///
/// Unknown, revoked and expired tokens are all 404, and deliberately
/// indistinguishable.
///
/// The response carries `X-Robots-Tag: noindex, nofollow`: the token is a
/// bearer credential on a public route, and an indexed preview link is a
/// published draft.
#[utoipa::path(
    get,
    path = "/api/public/blog/preview/{token}",
    tag = "blog",
    params(("token" = String, Path, description = "The preview token from the shared link")),
    responses(
        (
            status = 200,
            description = "The draft, as it stands now",
            body = inline(SuccessResponse<DraftPreviewResponse>)
        ),
        (
            status = 302,
            description = "The post is published; Location carries its public path"
        ),
        (
            status = 404,
            description = "Unknown, revoked or expired token",
            body = ErrorResponse,
            example = json!({
                "success": false,
                "error": { "code": "POST_NOT_FOUND", "message": "Blog post not found" }
            })
        ),
    )
)]
#[get("/api/public/blog/preview/{token}")]
pub async fn read_draft_preview_handler(
    path: web::Path<String>,
    data: web::Data<AppState>,
) -> impl Responder {
    match data.blog_preview.read.execute(&path.into_inner()).await {
        Ok(PreviewResolution::Draft(view)) => {
            let body = DraftPreviewResponse {
                preview: true,
                post: BlogPostDetailResponse::public(
                    view.post.into(),
                    view.topics.into_iter().map(Into::into).collect(),
                    view.media,
                ),
            };

            let mut response = ApiResponse::success(body);
            response.headers_mut().insert(
                header::HeaderName::from_static("x-robots-tag"),
                header::HeaderValue::from_static("noindex, nofollow"),
            );
            response
        }
        Ok(PreviewResolution::Published { username, slug }) => HttpResponse::Found()
            .insert_header((header::LOCATION, format!("/{username}/{slug}")))
            // Not cached: the post could be unpublished again, and a cached
            // redirect would strand the reviewer on a 404.
            .insert_header((header::CACHE_CONTROL, "no-store"))
            .finish(),
        Err(e) => map_error(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blog::application::ports::incoming::use_cases::ReadDraftPreviewUseCase;
    use crate::blog::application::ports::outgoing::BlogPostView;
    use crate::blog::domain::entities::BlogPost;
    use crate::tests::support::app_state_builder::TestAppStateBuilder;
    use actix_web::{http::StatusCode, test, App};
    use async_trait::async_trait;
    use chrono::Utc;

    struct StubRead(std::sync::Mutex<Option<Result<PreviewResolution, DraftPreviewError>>>);

    #[async_trait]
    impl ReadDraftPreviewUseCase for StubRead {
        async fn execute(&self, _t: &str) -> Result<PreviewResolution, DraftPreviewError> {
            self.0.lock().unwrap().take().unwrap()
        }
    }

    fn a_draft() -> PreviewResolution {
        PreviewResolution::Draft(Box::new(BlogPostView {
            post: BlogPost {
                id: Uuid::new_v4(),
                user_id: Uuid::new_v4(),
                title: "Work in progress".into(),
                slug: "wip".into(),
                excerpt: None,
                content: "body".into(),
                published_at: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            topics: vec![],
            media: vec![],
        }))
    }

    async fn call(
        resolution: Result<PreviewResolution, DraftPreviewError>,
    ) -> actix_web::dev::ServiceResponse {
        let state = TestAppStateBuilder::default()
            .with_draft_preview_read(StubRead(std::sync::Mutex::new(Some(resolution))))
            .build();

        let app = test::init_service(
            App::new()
                .app_data(state)
                .service(read_draft_preview_handler),
        )
        .await;

        test::call_service(
            &app,
            test::TestRequest::get()
                .uri("/api/public/blog/preview/sometoken")
                .to_request(),
        )
        .await
    }

    /// The token is a bearer credential on a public route. An indexed preview
    /// link is a published draft, so the header is not optional.
    #[actix_web::test]
    async fn a_served_draft_is_marked_noindex() {
        let resp = call(Ok(a_draft())).await;

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get("x-robots-tag").unwrap(),
            "noindex, nofollow"
        );
    }

    /// Without this flag a reviewer cannot tell a draft from the live post,
    /// and may share it as though it were public.
    #[actix_web::test]
    async fn a_served_draft_says_it_is_a_preview() {
        let resp = call(Ok(a_draft())).await;
        let body: serde_json::Value = test::read_body_json(resp).await;

        assert_eq!(body["data"]["preview"], true);

        // The rest of the payload is exactly what the public post endpoint
        // returns, flattened the same way, so a client reuses its parser and
        // only has to notice the extra flag.
        assert_eq!(body["data"]["title"], "Work in progress");
        assert_eq!(body["data"]["slug"], "wip");
        assert!(body["data"]["topics"].is_array());
    }

    #[actix_web::test]
    async fn a_published_post_redirects_rather_than_erroring() {
        let resp = call(Ok(PreviewResolution::Published {
            username: "janedoe".into(),
            slug: "shipped".into(),
        }))
        .await;

        assert_eq!(resp.status(), StatusCode::FOUND);
        assert_eq!(resp.headers().get("location").unwrap(), "/janedoe/shipped");
        assert_eq!(
            resp.headers().get("cache-control").unwrap(),
            "no-store",
            "a cached redirect would strand the reader if the post is unpublished again"
        );
    }

    #[actix_web::test]
    async fn a_dead_token_is_a_plain_404() {
        let resp = call(Err(DraftPreviewError::PostNotFound)).await;

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
