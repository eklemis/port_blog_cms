use crate::shared::api::ErrorCode;
use actix_web::{patch, web, Responder};
use tracing::error;
use uuid::Uuid;

use crate::{
    api::schemas::{ErrorResponse, SuccessResponse},
    auth::{
        adapter::incoming::web::extractors::auth::VerifiedUser,
        application::domain::entities::UserId,
    },
    blog::adapter::incoming::web::dto::{BlogPostResponse, PatchBlogPostRequest},
    blog::application::ports::incoming::use_cases::PatchBlogPostError,
    blog::application::ports::outgoing::PatchBlogPostData,
    shared::api::ApiResponse,
    AppState,
};

/// Partially update a post
///
/// Only the keys present in the body change. Sending `null` for `excerpt` or
/// `published_at` clears them — clearing `published_at` is how a post is
/// unpublished back to draft. The slug cannot be cleared, since the post's
/// public URL depends on it.
#[utoipa::path(
    patch,
    path = "/api/blog/{post_id}",
    tag = "blog",
    params(("post_id" = Uuid, Path, description = "Identifier of the post")),
    request_body = PatchBlogPostRequest,
    responses(
        (
            status = 200,
            description = "Post updated",
            body = inline(SuccessResponse<BlogPostResponse>)
        ),
        (
            status = 400,
            description = "Invalid slug, including an attempt to clear it",
            body = ErrorResponse,
            example = json!({
                "success": false,
                "error": { "code": "INVALID_SLUG", "message": "Slug cannot be cleared" }
            })
        ),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (
            status = 403,
            description = "Email not verified, or the post belongs to another author",
            body = ErrorResponse
        ),
        (status = 404, description = "Post not found", body = ErrorResponse),
        (
            status = 409,
            description = "This author already has a post with that slug",
            body = ErrorResponse
        ),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[patch("/api/blog/{post_id}")]
pub async fn patch_blog_post_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    req: web::Json<PatchBlogPostRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let req = req.into_inner();

    let patch = PatchBlogPostData {
        title: req.title,
        slug: req.slug,
        excerpt: req.excerpt,
        content: req.content,
        published_at: req.published_at,
    };

    match data
        .blog
        .patch
        .execute(UserId::from(user.user_id), path.into_inner(), patch)
        .await
    {
        Ok(post) => ApiResponse::success(BlogPostResponse::from(post)),

        Err(PatchBlogPostError::InvalidSlug(m)) => {
            ApiResponse::bad_request(ErrorCode::InvalidSlug, &m)
        }
        Err(PatchBlogPostError::NotFound) => {
            ApiResponse::not_found(ErrorCode::PostNotFound, "Blog post not found")
        }
        Err(PatchBlogPostError::Unauthorized) => ApiResponse::forbidden(
            ErrorCode::PostUnauthorized,
            "You are not authorized to edit this post",
        ),
        Err(PatchBlogPostError::SlugAlreadyExists) => {
            ApiResponse::conflict(ErrorCode::SlugAlreadyExists, "Slug already exists")
        }
        Err(PatchBlogPostError::RepositoryError(e)) => {
            error!("Repository error patching blog post: {}", e);
            ApiResponse::internal_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::adapter::outgoing::jwt::{JwtConfig, JwtTokenService};
    use crate::auth::application::ports::outgoing::token_provider::TokenProvider;
    use crate::blog::application::ports::incoming::use_cases::PatchBlogPostUseCase;
    use crate::blog::domain::entities::BlogPost;
    use crate::tests::support::app_state_builder::TestAppStateBuilder;
    use actix_web::{http::StatusCode, test, App};
    use async_trait::async_trait;
    use chrono::Utc;
    use serde_json::{json, Value};
    use std::sync::{Arc, Mutex};

    struct MockPatch {
        result: Result<BlogPost, PatchBlogPostError>,
        seen: Mutex<Option<PatchBlogPostData>>,
    }

    #[async_trait]
    impl PatchBlogPostUseCase for MockPatch {
        async fn execute(
            &self,
            _o: UserId,
            _p: Uuid,
            d: PatchBlogPostData,
        ) -> Result<BlogPost, PatchBlogPostError> {
            *self.seen.lock().unwrap() = Some(d);
            self.result.clone()
        }
    }

    fn a_post() -> BlogPost {
        let now = Utc::now();
        BlogPost {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            title: "Title".into(),
            slug: "title".into(),
            excerpt: None,
            content: "body".into(),
            published_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn jwt() -> JwtTokenService {
        JwtTokenService::new(JwtConfig {
            issuer: "Lotion".to_string(),
            secret_key: "test_secret_key_for_testing_purposes_only".to_string(),
            access_token_expiry: 3600,
            refresh_token_expiry: 86400,
            verification_token_expiry: 86400,
            password_reset_expiry: 3600,
        })
    }

    async fn call_with_body(body: Value) -> (StatusCode, Value) {
        let j = jwt();
        let token = j.generate_access_token(Uuid::new_v4(), true).unwrap();
        let provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(j);

        let app = test::init_service(
            App::new()
                .app_data(
                    TestAppStateBuilder::default()
                        .with_blog_patch(MockPatch {
                            result: Ok(a_post()),
                            seen: Mutex::new(None),
                        })
                        .build(),
                )
                .app_data(web::Data::new(provider))
                .service(patch_blog_post_handler),
        )
        .await;

        let req = test::TestRequest::patch()
            .uri(&format!("/api/blog/{}", Uuid::new_v4()))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(body)
            .to_request();

        let resp = test::call_service(&app, req).await;
        let status = resp.status();
        let json: Value = test::read_body_json(resp).await;
        (status, json)
    }

    async fn call_err(err: PatchBlogPostError) -> actix_web::dev::ServiceResponse {
        let j = jwt();
        let token = j.generate_access_token(Uuid::new_v4(), true).unwrap();
        let provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(j);

        let app = test::init_service(
            App::new()
                .app_data(
                    TestAppStateBuilder::default()
                        .with_blog_patch(MockPatch {
                            result: Err(err),
                            seen: Mutex::new(None),
                        })
                        .build(),
                )
                .app_data(web::Data::new(provider))
                .service(patch_blog_post_handler),
        )
        .await;

        let req = test::TestRequest::patch()
            .uri(&format!("/api/blog/{}", Uuid::new_v4()))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({ "title": "New" }))
            .to_request();

        test::call_service(&app, req).await
    }

    #[actix_web::test]
    async fn patches_a_post() {
        let (status, body) = call_with_body(json!({ "title": "New title" })).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["success"], true);
    }

    /// An omitted key must deserialise to Unset, not to a null write. This is
    /// the distinction the endpoint's whole patch semantics rest on, and it is
    /// carried by `#[serde(default)]` on each field.
    #[actix_web::test]
    async fn an_omitted_field_is_not_treated_as_null() {
        let (status, _) = call_with_body(json!({ "title": "Only the title" })).await;
        assert_eq!(status, StatusCode::OK);
    }

    /// An explicit null is a write, and it is how a post is unpublished.
    #[actix_web::test]
    async fn an_explicit_null_published_at_is_accepted() {
        let (status, _) = call_with_body(json!({ "published_at": null })).await;
        assert_eq!(status, StatusCode::OK);
    }

    #[actix_web::test]
    async fn another_authors_post_is_forbidden() {
        let resp = call_err(PatchBlogPostError::Unauthorized).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "POST_UNAUTHORIZED");
    }

    #[actix_web::test]
    async fn a_missing_post_is_not_found() {
        let resp = call_err(PatchBlogPostError::NotFound).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "POST_NOT_FOUND");
    }

    #[actix_web::test]
    async fn clearing_the_slug_is_a_bad_request() {
        let resp = call_err(PatchBlogPostError::InvalidSlug(
            "Slug cannot be cleared".into(),
        ))
        .await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "INVALID_SLUG");
    }
}
