//! Topic attachment for blog posts.
//!
//! Kept in one file because the four handlers share the same error mapping and
//! differ only in verb and payload.

use crate::shared::api::ErrorCode;
use actix_web::{delete, get, post, web, Responder};
use tracing::error;
use uuid::Uuid;

use crate::{
    api::schemas::{ErrorResponse, SuccessResponse},
    auth::{
        adapter::incoming::web::extractors::auth::VerifiedUser,
        application::domain::entities::UserId,
    },
    blog::adapter::incoming::web::dto::{BlogPostTopicRequest, BlogPostTopicResponse},
    blog::application::ports::incoming::use_cases::{BlogPostTopicError, GetBlogPostError},
    shared::api::ApiResponse,
    AppState,
};

fn map_topic_error(e: BlogPostTopicError) -> actix_web::HttpResponse {
    match e {
        BlogPostTopicError::PostNotFound => {
            ApiResponse::not_found(ErrorCode::PostNotFound, "Blog post not found")
        }
        BlogPostTopicError::TopicNotFound => {
            ApiResponse::not_found(ErrorCode::TopicNotFound, "Topic not found")
        }
        BlogPostTopicError::RepositoryError(e) => {
            error!("Repository error on blog post topics: {}", e);
            ApiResponse::internal_error()
        }
    }
}

/// Attach a topic to a post
///
/// Both the post and the topic must belong to the caller. Attaching a topic
/// that is already attached succeeds without creating a duplicate.
#[utoipa::path(
    post,
    path = "/api/blog/{post_id}/topics",
    tag = "blog",
    params(("post_id" = Uuid, Path, description = "Identifier of the post")),
    request_body = BlogPostTopicRequest,
    responses(
        (status = 204, description = "Topic attached, or already was"),
        (status = 400, description = "Malformed request body", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
        (
            status = 404,
            description = "Post or topic not found. Codes: POST_NOT_FOUND, TOPIC_NOT_FOUND",
            body = ErrorResponse
        ),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[post("/api/blog/{post_id}/topics")]
pub async fn attach_blog_post_topic_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    body: web::Json<BlogPostTopicRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    match data
        .blog
        .attach_topic
        .execute(UserId::from(user.user_id), path.into_inner(), body.topic_id)
        .await
    {
        Ok(()) => ApiResponse::no_content(),
        Err(e) => map_topic_error(e),
    }
}

/// Detach a topic from a post
///
/// Idempotent: detaching a topic that is not attached still returns 204.
#[utoipa::path(
    delete,
    path = "/api/blog/{post_id}/topics",
    tag = "blog",
    params(("post_id" = Uuid, Path, description = "Identifier of the post")),
    request_body = BlogPostTopicRequest,
    responses(
        (status = 204, description = "Topic detached, or was not attached"),
        (status = 400, description = "Malformed request body", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
        (status = 404, description = "Post not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[delete("/api/blog/{post_id}/topics")]
pub async fn detach_blog_post_topic_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    body: web::Json<BlogPostTopicRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    match data
        .blog
        .detach_topic
        .execute(UserId::from(user.user_id), path.into_inner(), body.topic_id)
        .await
    {
        Ok(()) => ApiResponse::no_content(),
        Err(e) => map_topic_error(e),
    }
}

/// Detach every topic from a post
#[utoipa::path(
    delete,
    path = "/api/blog/{post_id}/topics/all",
    tag = "blog",
    params(("post_id" = Uuid, Path, description = "Identifier of the post")),
    responses(
        (status = 204, description = "All topics detached"),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
        (status = 404, description = "Post not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[delete("/api/blog/{post_id}/topics/all")]
pub async fn clear_blog_post_topics_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> impl Responder {
    match data
        .blog
        .clear_topics
        .execute(UserId::from(user.user_id), path.into_inner())
        .await
    {
        Ok(()) => ApiResponse::no_content(),
        Err(e) => map_topic_error(e),
    }
}

/// List the topics attached to a post
#[utoipa::path(
    get,
    path = "/api/blog/{post_id}/topics",
    tag = "blog",
    params(("post_id" = Uuid, Path, description = "Identifier of the post")),
    responses(
        (
            status = 200,
            description = "Topics retrieved",
            body = inline(SuccessResponse<Vec<BlogPostTopicResponse>>)
        ),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
        (status = 404, description = "Post not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[get("/api/blog/{post_id}/topics")]
pub async fn get_blog_post_topics_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> impl Responder {
    match data
        .blog
        .get_topics
        .execute(UserId::from(user.user_id), path.into_inner())
        .await
    {
        Ok(topics) => ApiResponse::success(
            topics
                .into_iter()
                .map(BlogPostTopicResponse::from)
                .collect::<Vec<_>>(),
        ),
        Err(GetBlogPostError::NotFound) => {
            ApiResponse::not_found(ErrorCode::PostNotFound, "Blog post not found")
        }
        Err(GetBlogPostError::QueryFailed(e)) => {
            error!("Failed to list blog post topics: {}", e);
            ApiResponse::internal_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::adapter::outgoing::jwt::{JwtConfig, JwtTokenService};
    use crate::auth::application::ports::outgoing::token_provider::TokenProvider;
    use crate::blog::application::ports::incoming::use_cases::{
        AttachBlogPostTopicUseCase, GetBlogPostTopicsUseCase,
    };
    use crate::blog::domain::entities::BlogPostTopic;
    use crate::tests::support::app_state_builder::TestAppStateBuilder;
    use actix_web::{http::StatusCode, test, App};
    use async_trait::async_trait;
    use serde_json::{json, Value};
    use std::sync::Arc;

    struct MockAttach {
        result: Result<(), BlogPostTopicError>,
    }

    #[async_trait]
    impl AttachBlogPostTopicUseCase for MockAttach {
        async fn execute(&self, _o: UserId, _p: Uuid, _t: Uuid) -> Result<(), BlogPostTopicError> {
            self.result.clone()
        }
    }

    struct MockGetTopics {
        result: Result<Vec<BlogPostTopic>, GetBlogPostError>,
    }

    #[async_trait]
    impl GetBlogPostTopicsUseCase for MockGetTopics {
        async fn execute(
            &self,
            _o: UserId,
            _p: Uuid,
        ) -> Result<Vec<BlogPostTopic>, GetBlogPostError> {
            self.result.clone()
        }
    }

    fn token_and_provider() -> (String, Arc<dyn TokenProvider + Send + Sync>) {
        let j = JwtTokenService::new(JwtConfig {
            issuer: "Lotion".to_string(),
            secret_key: "test_secret_key_for_testing_purposes_only".to_string(),
            access_token_expiry: 3600,
            refresh_token_expiry: 86400,
            verification_token_expiry: 86400,
            password_reset_expiry: 3600,
        });
        let token = j.generate_access_token(Uuid::new_v4(), true).unwrap();
        (token, Arc::new(j))
    }

    async fn attach(result: Result<(), BlogPostTopicError>) -> actix_web::dev::ServiceResponse {
        let (token, provider) = token_and_provider();

        let app = test::init_service(
            App::new()
                .app_data(
                    TestAppStateBuilder::default()
                        .with_blog_attach_topic(MockAttach { result })
                        .build(),
                )
                .app_data(web::Data::new(provider))
                .service(attach_blog_post_topic_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri(&format!("/api/blog/{}/topics", Uuid::new_v4()))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({ "topic_id": Uuid::new_v4() }))
            .to_request();

        test::call_service(&app, req).await
    }

    #[actix_web::test]
    async fn attaching_returns_no_content() {
        assert_eq!(attach(Ok(())).await.status(), StatusCode::NO_CONTENT);
    }

    /// A missing post and a missing topic must be distinguishable, which is why
    /// the repository checks them separately rather than letting one INSERT
    /// violate either foreign key.
    #[actix_web::test]
    async fn a_missing_post_and_a_missing_topic_report_different_codes() {
        let resp = attach(Err(BlogPostTopicError::PostNotFound)).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "POST_NOT_FOUND");

        let resp = attach(Err(BlogPostTopicError::TopicNotFound)).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "TOPIC_NOT_FOUND");
    }

    #[actix_web::test]
    async fn a_repository_failure_is_an_internal_error() {
        let resp = attach(Err(BlogPostTopicError::RepositoryError("db down".into()))).await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[actix_web::test]
    async fn listing_topics_returns_them() {
        let (token, provider) = token_and_provider();

        let app = test::init_service(
            App::new()
                .app_data(
                    TestAppStateBuilder::default()
                        .with_blog_get_topics(MockGetTopics {
                            result: Ok(vec![BlogPostTopic {
                                id: Uuid::new_v4(),
                                title: "Rust".into(),
                                description: "Posts about Rust".into(),
                            }]),
                        })
                        .build(),
                )
                .app_data(web::Data::new(provider))
                .service(get_blog_post_topics_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/blog/{}/topics", Uuid::new_v4()))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["data"][0]["title"], "Rust");
    }

    #[actix_web::test]
    async fn listing_topics_for_a_missing_post_is_not_found() {
        let (token, provider) = token_and_provider();

        let app = test::init_service(
            App::new()
                .app_data(
                    TestAppStateBuilder::default()
                        .with_blog_get_topics(MockGetTopics {
                            result: Err(GetBlogPostError::NotFound),
                        })
                        .build(),
                )
                .app_data(web::Data::new(provider))
                .service(get_blog_post_topics_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/blog/{}/topics", Uuid::new_v4()))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    use crate::blog::application::ports::incoming::use_cases::{
        ClearBlogPostTopicsUseCase, DetachBlogPostTopicUseCase,
    };

    struct MockDetach {
        result: Result<(), BlogPostTopicError>,
    }

    #[async_trait]
    impl DetachBlogPostTopicUseCase for MockDetach {
        async fn execute(&self, _o: UserId, _p: Uuid, _t: Uuid) -> Result<(), BlogPostTopicError> {
            self.result.clone()
        }
    }

    struct MockClear {
        result: Result<(), BlogPostTopicError>,
    }

    #[async_trait]
    impl ClearBlogPostTopicsUseCase for MockClear {
        async fn execute(&self, _o: UserId, _p: Uuid) -> Result<(), BlogPostTopicError> {
            self.result.clone()
        }
    }

    async fn detach(result: Result<(), BlogPostTopicError>) -> actix_web::dev::ServiceResponse {
        let (token, provider) = token_and_provider();

        let app = test::init_service(
            App::new()
                .app_data(
                    TestAppStateBuilder::default()
                        .with_blog_detach_topic(MockDetach { result })
                        .build(),
                )
                .app_data(web::Data::new(provider))
                .service(detach_blog_post_topic_handler),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri(&format!("/api/blog/{}/topics", Uuid::new_v4()))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({ "topic_id": Uuid::new_v4() }))
            .to_request();

        test::call_service(&app, req).await
    }

    async fn clear(result: Result<(), BlogPostTopicError>) -> actix_web::dev::ServiceResponse {
        let (token, provider) = token_and_provider();

        let app = test::init_service(
            App::new()
                .app_data(
                    TestAppStateBuilder::default()
                        .with_blog_clear_topics(MockClear { result })
                        .build(),
                )
                .app_data(web::Data::new(provider))
                .service(clear_blog_post_topics_handler),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri(&format!("/api/blog/{}/topics/all", Uuid::new_v4()))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();

        test::call_service(&app, req).await
    }

    /// Detaching is idempotent at the repository, so the handler returns 204
    /// whether or not the topic was attached.
    #[actix_web::test]
    async fn detaching_returns_no_content() {
        assert_eq!(detach(Ok(())).await.status(), StatusCode::NO_CONTENT);
    }

    #[actix_web::test]
    async fn detaching_from_a_missing_post_is_not_found() {
        let resp = detach(Err(BlogPostTopicError::PostNotFound)).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "POST_NOT_FOUND");
    }

    #[actix_web::test]
    async fn detach_surfaces_repository_failures() {
        let resp = detach(Err(BlogPostTopicError::RepositoryError("db down".into()))).await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[actix_web::test]
    async fn clearing_returns_no_content() {
        assert_eq!(clear(Ok(())).await.status(), StatusCode::NO_CONTENT);
    }

    #[actix_web::test]
    async fn clearing_a_missing_post_is_not_found() {
        let resp = clear(Err(BlogPostTopicError::PostNotFound)).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "POST_NOT_FOUND");
    }

    #[actix_web::test]
    async fn clear_surfaces_repository_failures() {
        let resp = clear(Err(BlogPostTopicError::RepositoryError("db down".into()))).await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
