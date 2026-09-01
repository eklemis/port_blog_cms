use crate::shared::api::ErrorCode;
use actix_web::{get, web, Responder};
use tracing::error;
use uuid::Uuid;

use crate::{
    api::schemas::{ErrorResponse, SuccessResponse},
    auth::{
        adapter::incoming::web::extractors::auth::VerifiedUser,
        application::domain::entities::UserId,
    },
    blog::adapter::incoming::web::dto::BlogPostDetailResponse,
    blog::application::ports::incoming::use_cases::GetBlogPostError,
    shared::api::ApiResponse,
    AppState,
};

/// Get one of the author's own posts
///
/// Returns drafts as well as published posts, with the post's topics. A post
/// belonging to another author is reported as not found rather than forbidden.
#[utoipa::path(
    get,
    path = "/api/blog/{post_id}",
    tag = "blog",
    params(("post_id" = Uuid, Path, description = "Identifier of the post")),
    responses(
        (
            status = 200,
            description = "Post retrieved",
            body = inline(SuccessResponse<BlogPostDetailResponse>)
        ),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
        (
            status = 404,
            description = "Post not found, archived, or owned by another author",
            body = ErrorResponse,
            example = json!({
                "success": false,
                "error": { "code": "POST_NOT_FOUND", "message": "Blog post not found" }
            })
        ),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[get("/api/blog/{post_id}")]
pub async fn get_single_blog_post_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> impl Responder {
    match data
        .blog
        .get_single
        .execute(UserId::from(user.user_id), path.into_inner())
        .await
    {
        Ok(view) => ApiResponse::success(BlogPostDetailResponse::owner(
            view.post.into(),
            view.topics.into_iter().map(Into::into).collect(),
        )),
        Err(GetBlogPostError::NotFound) => {
            ApiResponse::not_found(ErrorCode::PostNotFound, "Blog post not found")
        }
        Err(GetBlogPostError::QueryFailed(e)) => {
            error!("Failed to fetch blog post: {}", e);
            ApiResponse::internal_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::adapter::outgoing::jwt::{JwtConfig, JwtTokenService};
    use crate::auth::application::ports::outgoing::token_provider::TokenProvider;
    use crate::blog::application::ports::incoming::use_cases::GetSingleBlogPostUseCase;
    use crate::blog::application::ports::outgoing::BlogPostView;
    use crate::blog::domain::entities::{BlogPost, BlogPostTopic};
    use crate::tests::support::app_state_builder::TestAppStateBuilder;
    use actix_web::{http::StatusCode, test, App};
    use async_trait::async_trait;
    use chrono::Utc;
    use serde_json::Value;
    use std::sync::Arc;

    struct MockGet {
        result: Result<BlogPostView, GetBlogPostError>,
    }

    #[async_trait]
    impl GetSingleBlogPostUseCase for MockGet {
        async fn execute(&self, _o: UserId, _p: Uuid) -> Result<BlogPostView, GetBlogPostError> {
            self.result.clone()
        }
    }

    fn a_draft_view() -> BlogPostView {
        let now = Utc::now();
        BlogPostView {
            post: BlogPost {
                id: Uuid::new_v4(),
                user_id: Uuid::new_v4(),
                title: "Draft".into(),
                slug: "draft".into(),
                excerpt: None,
                content: "body".into(),
                published_at: None,
                created_at: now,
                updated_at: now,
            },
            topics: vec![BlogPostTopic {
                id: Uuid::new_v4(),
                title: "Rust".into(),
                description: "Posts about Rust".into(),
            }],
            media: Vec::new(),
        }
    }

    async fn call(
        result: Result<BlogPostView, GetBlogPostError>,
    ) -> actix_web::dev::ServiceResponse {
        let j = JwtTokenService::new(JwtConfig {
            issuer: "Lotion".to_string(),
            secret_key: "test_secret_key_for_testing_purposes_only".to_string(),
            access_token_expiry: 3600,
            refresh_token_expiry: 86400,
            verification_token_expiry: 86400,
            password_reset_expiry: 3600,
        });
        let token = j.generate_access_token(Uuid::new_v4(), true).unwrap();
        let provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(j);

        let app = test::init_service(
            App::new()
                .app_data(
                    TestAppStateBuilder::default()
                        .with_blog_get_single(MockGet { result })
                        .build(),
                )
                .app_data(web::Data::new(provider))
                .service(get_single_blog_post_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/blog/{}", Uuid::new_v4()))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();

        test::call_service(&app, req).await
    }

    /// Unlike the public endpoint, an author's own view includes drafts, with
    /// the post's topics flattened alongside its fields.
    #[actix_web::test]
    async fn returns_a_draft_with_its_topics() {
        let resp = call(Ok(a_draft_view())).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["data"]["slug"], "draft");
        assert!(body["data"]["published_at"].is_null());
        assert_eq!(body["data"]["topics"][0]["title"], "Rust");
    }

    #[actix_web::test]
    async fn a_missing_post_is_not_found() {
        let resp = call(Err(GetBlogPostError::NotFound)).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "POST_NOT_FOUND");
    }

    #[actix_web::test]
    async fn a_query_failure_is_an_internal_error() {
        let resp = call(Err(GetBlogPostError::QueryFailed("db down".into()))).await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
