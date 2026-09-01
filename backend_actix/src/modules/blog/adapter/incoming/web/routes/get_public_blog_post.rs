use crate::shared::api::ErrorCode;
use actix_web::{get, web, Responder};
use serde::Deserialize;
use tracing::error;

use crate::{
    api::schemas::{ErrorResponse, SuccessResponse},
    auth::adapter::incoming::web::extractors::auth::resolve_owner_id_or_response,
    auth::application::domain::entities::UserId,
    blog::adapter::incoming::web::dto::BlogPostDetailResponse,
    blog::application::ports::incoming::use_cases::GetBlogPostError,
    shared::api::ApiResponse,
    AppState,
};

/// See the module documentation.
#[derive(Debug, Deserialize)]
pub struct PublicPostPath {
    /// Public handle.
    pub username: String,
    /// URL segment. Unique per owner.
    pub slug: String,
}

/// Get a published post by author and slug
///
/// Public: no authentication required. Addressed by slug, which is unique per
/// author, so this is the shareable permalink. Drafts and posts scheduled for
/// the future report 404 — indistinguishable from a slug that does not exist,
/// so an unpublished post cannot be detected.
#[utoipa::path(
    get,
    path = "/api/public/blog/{username}/{slug}",
    tag = "blog",
    params(
        ("username" = String, Path, description = "Author of the post"),
        ("slug" = String, Path, description = "URL slug of the post")
    ),
    responses(
        (
            status = 200,
            description = "Post retrieved",
            body = inline(SuccessResponse<BlogPostDetailResponse>)
        ),
        (
            status = 404,
            description = "No such username, or no published post with that slug",
            body = ErrorResponse,
            example = json!({
                "success": false,
                "error": { "code": "POST_NOT_FOUND", "message": "Blog post not found" }
            })
        ),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    )
)]
#[get("/api/public/blog/{username}/{slug}")]
pub async fn get_public_blog_post_handler(
    path: web::Path<PublicPostPath>,
    data: web::Data<AppState>,
) -> impl Responder {
    let path = path.into_inner();

    let owner_id = match resolve_owner_id_or_response(&data, &path.username).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    match data
        .blog
        .get_public
        .execute(UserId::from(owner_id), &path.slug)
        .await
    {
        Ok(view) => ApiResponse::success(BlogPostDetailResponse::public(
            view.post.into(),
            view.topics.into_iter().map(Into::into).collect(),
            view.media,
        )),
        Err(GetBlogPostError::NotFound) => {
            ApiResponse::not_found(ErrorCode::PostNotFound, "Blog post not found")
        }
        Err(GetBlogPostError::QueryFailed(e)) => {
            error!("Failed to fetch public blog post: {}", e);
            ApiResponse::internal_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::application::helpers::UserIdentityResolver;
    use crate::auth::application::ports::outgoing::user_query::{
        UserQuery, UserQueryError, UserQueryResult,
    };
    use crate::blog::application::ports::incoming::use_cases::GetPublicBlogPostUseCase;
    use crate::blog::application::ports::outgoing::BlogPostView;
    use crate::blog::domain::entities::BlogPost;
    use crate::tests::support::app_state_builder::TestAppStateBuilder;
    use actix_web::{http::StatusCode, test, App};
    use async_trait::async_trait;
    use chrono::Utc;
    use serde_json::Value;
    use std::sync::Arc;
    use uuid::Uuid;

    #[derive(Clone)]
    struct MockUserQuery {
        result: Result<Option<UserQueryResult>, UserQueryError>,
    }

    #[async_trait]
    impl UserQuery for MockUserQuery {
        async fn find_by_email(&self, _e: &str) -> Result<Option<UserQueryResult>, UserQueryError> {
            unimplemented!()
        }
        async fn find_by_username(
            &self,
            _u: &str,
        ) -> Result<Option<UserQueryResult>, UserQueryError> {
            self.result.clone()
        }
        async fn find_by_id(&self, _i: Uuid) -> Result<Option<UserQueryResult>, UserQueryError> {
            unimplemented!()
        }
    }

    struct MockGetPublic {
        result: Result<BlogPostView, GetBlogPostError>,
    }

    #[async_trait]
    impl GetPublicBlogPostUseCase for MockGetPublic {
        async fn execute(&self, _o: UserId, _s: &str) -> Result<BlogPostView, GetBlogPostError> {
            self.result.clone()
        }
    }

    fn a_user(id: Uuid) -> UserQueryResult {
        UserQueryResult {
            id,
            username: "author".into(),
            email: "author@example.com".into(),
            full_name: "The Author".into(),
            password_hash: "hash".into(),
            is_verified: true,
            is_deleted: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn a_view(user_id: Uuid) -> BlogPostView {
        let now = Utc::now();
        BlogPostView {
            post: BlogPost {
                id: Uuid::new_v4(),
                user_id,
                title: "Published".into(),
                slug: "published".into(),
                excerpt: None,
                content: "body".into(),
                published_at: Some(now),
                created_at: now,
                updated_at: now,
            },
            topics: vec![],
            media: Vec::new(),
        }
    }

    async fn call(
        user: Result<Option<UserQueryResult>, UserQueryError>,
        post: Result<BlogPostView, GetBlogPostError>,
    ) -> actix_web::dev::ServiceResponse {
        let resolver = UserIdentityResolver::new(Arc::new(MockUserQuery { result: user }));

        let app = test::init_service(
            App::new()
                .app_data(
                    TestAppStateBuilder::default()
                        .with_user_identity_resolver(resolver)
                        .with_blog_get_public(MockGetPublic { result: post })
                        .build(),
                )
                .service(get_public_blog_post_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/public/blog/author/published")
            .to_request();

        test::call_service(&app, req).await
    }

    /// No Authorization header: the public permalink must work for anonymous
    /// readers, which is the entire point of the endpoint.
    #[actix_web::test]
    async fn serves_a_published_post_without_authentication() {
        let user_id = Uuid::new_v4();
        let resp = call(Ok(Some(a_user(user_id))), Ok(a_view(user_id))).await;

        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["data"]["slug"], "published");
        assert_eq!(body["data"]["topics"], serde_json::json!([]));
    }

    /// A draft is reported exactly like a slug that does not exist, so the
    /// endpoint cannot be used to discover unpublished posts.
    #[actix_web::test]
    async fn a_draft_is_indistinguishable_from_a_missing_post() {
        let user_id = Uuid::new_v4();
        let resp = call(Ok(Some(a_user(user_id))), Err(GetBlogPostError::NotFound)).await;

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "POST_NOT_FOUND");
    }

    #[actix_web::test]
    async fn an_unknown_author_is_not_found() {
        let resp = call(Ok(None), Ok(a_view(Uuid::new_v4()))).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn a_query_failure_is_an_internal_error() {
        let user_id = Uuid::new_v4();
        let resp = call(
            Ok(Some(a_user(user_id))),
            Err(GetBlogPostError::QueryFailed("db down".into())),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
