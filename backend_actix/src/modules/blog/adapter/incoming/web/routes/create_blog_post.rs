use crate::shared::api::ErrorCode;
use actix_web::{post, web, Responder};
use tracing::error;

use crate::{
    api::schemas::{ErrorResponse, SuccessResponse},
    auth::{
        adapter::incoming::web::extractors::auth::VerifiedUser,
        application::domain::entities::UserId,
    },
    blog::adapter::incoming::web::dto::{BlogPostResponse, CreateBlogPostRequest},
    blog::application::ports::incoming::use_cases::{
        CreateBlogPostCommand, CreateBlogPostError,
    },
    shared::api::ApiResponse,
    AppState,
};

/// Create a blog post
///
/// Omit `published_at` to create a draft. A future timestamp schedules the
/// post: it stays out of public listings until that moment passes.
///
/// Slugs are lowercased and must contain only letters, numbers and hyphens.
/// They are unique per author, so two authors may both use `hello-world`.
#[utoipa::path(
    post,
    path = "/api/blog",
    tag = "blog",
    request_body = CreateBlogPostRequest,
    responses(
        (
            status = 201,
            description = "Post created",
            body = inline(SuccessResponse<BlogPostResponse>)
        ),
        (
            status = 400,
            description = "Validation failed. Codes: INVALID_TITLE, INVALID_SLUG, INVALID_CONTENT",
            body = ErrorResponse,
            example = json!({
                "success": false,
                "error": {
                    "code": "INVALID_SLUG",
                    "message": "Slug may contain only letters, numbers, and hyphens"
                }
            })
        ),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
        (
            status = 409,
            description = "This author already has a post with that slug",
            body = ErrorResponse,
            example = json!({
                "success": false,
                "error": { "code": "SLUG_ALREADY_EXISTS", "message": "Slug already exists" }
            })
        ),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[post("/api/blog")]
pub async fn create_blog_post_handler(
    user: VerifiedUser,
    req: web::Json<CreateBlogPostRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let req = req.into_inner();

    let command = CreateBlogPostCommand {
        owner: UserId::from(user.user_id),
        title: req.title,
        slug: req.slug,
        excerpt: req.excerpt,
        content: req.content,
        published_at: req.published_at,
    };

    match data.blog.create.execute(command).await {
        Ok(post) => ApiResponse::created(BlogPostResponse::from(post)),

        Err(CreateBlogPostError::InvalidTitle(m)) => ApiResponse::bad_request(ErrorCode::InvalidTitle, &m),
        Err(CreateBlogPostError::InvalidSlug(m)) => ApiResponse::bad_request(ErrorCode::InvalidSlug, &m),
        Err(CreateBlogPostError::InvalidContent(m)) => {
            ApiResponse::bad_request(ErrorCode::InvalidContent, &m)
        }
        Err(CreateBlogPostError::SlugAlreadyExists) => {
            ApiResponse::conflict(ErrorCode::SlugAlreadyExists, "Slug already exists")
        }
        Err(CreateBlogPostError::RepositoryError(e)) => {
            error!("Repository error creating blog post: {}", e);
            ApiResponse::internal_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::adapter::outgoing::jwt::{JwtConfig, JwtTokenService};
    use crate::auth::application::ports::outgoing::token_provider::TokenProvider;
    use crate::blog::application::ports::incoming::use_cases::CreateBlogPostUseCase;
    use crate::blog::domain::entities::BlogPost;
    use crate::tests::support::app_state_builder::TestAppStateBuilder;
    use actix_web::{http::StatusCode, test, App};
    use async_trait::async_trait;
    use chrono::Utc;
    use serde_json::{json, Value};
    use std::sync::Arc;
    use uuid::Uuid;

    struct MockCreate {
        result: Result<BlogPost, CreateBlogPostError>,
    }

    #[async_trait]
    impl CreateBlogPostUseCase for MockCreate {
        async fn execute(
            &self,
            _c: CreateBlogPostCommand,
        ) -> Result<BlogPost, CreateBlogPostError> {
            self.result.clone()
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

    fn a_post(user_id: Uuid) -> BlogPost {
        let now = Utc::now();
        BlogPost {
            id: Uuid::new_v4(),
            user_id,
            title: "Building a CMS in Rust".into(),
            slug: "building-a-cms-in-rust".into(),
            excerpt: None,
            content: "body".into(),
            published_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    async fn call(
        result: Result<BlogPost, CreateBlogPostError>,
        verified: bool,
    ) -> actix_web::dev::ServiceResponse {
        let j = jwt();
        let token = j.generate_access_token(Uuid::new_v4(), verified).unwrap();
        let provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(j);

        let app = test::init_service(
            App::new()
                .app_data(
                    TestAppStateBuilder::default()
                        .with_blog_create(MockCreate { result })
                        .build(),
                )
                .app_data(web::Data::new(provider))
                .service(create_blog_post_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/blog")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({
                "title": "Building a CMS in Rust",
                "slug": "building-a-cms-in-rust",
                "content": "body"
            }))
            .to_request();

        test::call_service(&app, req).await
    }

    #[actix_web::test]
    async fn creates_a_post_and_returns_201() {
        let resp = call(Ok(a_post(Uuid::new_v4())), true).await;
        assert_eq!(resp.status(), StatusCode::CREATED);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert_eq!(body["data"]["slug"], "building-a-cms-in-rust");
        // Absent published_at is a draft, and must serialise as null rather
        // than being omitted, so clients need not treat the two differently.
        assert!(body["data"]["published_at"].is_null());
    }

    #[actix_web::test]
    async fn an_invalid_slug_is_a_bad_request() {
        let resp = call(
            Err(CreateBlogPostError::InvalidSlug("bad slug".into())),
            true,
        )
        .await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "INVALID_SLUG");
    }

    #[actix_web::test]
    async fn a_duplicate_slug_is_a_conflict() {
        let resp = call(Err(CreateBlogPostError::SlugAlreadyExists), true).await;
        assert_eq!(resp.status(), StatusCode::CONFLICT);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "SLUG_ALREADY_EXISTS");
    }

    #[actix_web::test]
    async fn repository_failure_is_an_internal_error() {
        let resp = call(
            Err(CreateBlogPostError::RepositoryError("db down".into())),
            true,
        )
        .await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[actix_web::test]
    async fn unverified_user_is_forbidden() {
        let resp = call(Ok(a_post(Uuid::new_v4())), false).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "EMAIL_NOT_VERIFIED");
    }
}
