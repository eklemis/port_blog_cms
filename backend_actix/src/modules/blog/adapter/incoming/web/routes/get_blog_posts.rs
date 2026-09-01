use actix_web::{get, web, Responder};
use serde::Deserialize;
use tracing::error;
use utoipa::IntoParams;

use crate::{
    api::schemas::{ErrorResponse, SuccessResponse},
    auth::{
        adapter::incoming::web::extractors::auth::VerifiedUser,
        application::domain::entities::UserId,
    },
    blog::adapter::incoming::web::dto::BlogPostCardResponse,
    blog::application::ports::incoming::use_cases::GetBlogPostsError,
    blog::application::ports::outgoing::{
        BlogPageRequest, BlogPageResult, BlogPostListFilter, BlogPostSort,
    },
    shared::api::ApiResponse,
    AppState,
};

/// Adapter implementing the matching outgoing port.
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct GetBlogPostsQuery {
    /// Free-text search over title and excerpt
    #[param(example = "rust")]
    pub search: Option<String>,

    /// Restrict to posts carrying this topic
    pub topic_id: Option<uuid::Uuid>,

    /// `true` for published only, `false` for drafts only. Omit for both.
    /// Ignored by the public listing, which always forces published only.
    pub published: Option<bool>,

    /// Listing order.
    #[serde(default)]
    pub sort: BlogPostSort,

    /// 1-based page number.
    #[param(example = 1, minimum = 1)]
    #[serde(default)]
    pub page: u32,

    /// Rows per page.
    #[param(example = 10, minimum = 1, maximum = 100)]
    #[serde(default)]
    pub per_page: u32,
}

impl From<GetBlogPostsQuery> for (BlogPostListFilter, BlogPageRequest, BlogPostSort) {
    fn from(q: GetBlogPostsQuery) -> Self {
        (
            BlogPostListFilter {
                search: q.search,
                topic_id: q.topic_id,
                published: q.published,
            },
            BlogPageRequest {
                page: if q.page == 0 { 1 } else { q.page },
                per_page: if q.per_page == 0 { 10 } else { q.per_page },
            },
            q.sort,
        )
    }
}

fn to_response(
    result: BlogPageResult<crate::blog::application::ports::outgoing::BlogPostCard>,
) -> BlogPageResult<BlogPostCardResponse> {
    BlogPageResult {
        items: result.items.into_iter().map(Into::into).collect(),
        page: result.page,
        per_page: result.per_page,
        total: result.total,
    }
}

/// List the authenticated author's posts
///
/// Includes drafts. Use `published=true` or `published=false` to narrow.
#[utoipa::path(
    get,
    path = "/api/blog",
    tag = "blog",
    params(GetBlogPostsQuery),
    responses(
        (
            status = 200,
            description = "Posts retrieved",
            body = inline(SuccessResponse<BlogPageResult<BlogPostCardResponse>>)
        ),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[get("/api/blog")]
pub async fn get_blog_posts_handler(
    user: VerifiedUser,
    query: web::Query<GetBlogPostsQuery>,
    data: web::Data<AppState>,
) -> impl Responder {
    let (filter, page, sort) = query.into_inner().into();

    match data
        .blog
        .list
        .execute(UserId::from(user.user_id), filter, sort, page)
        .await
    {
        Ok(result) => ApiResponse::success(to_response(result)),
        Err(GetBlogPostsError::QueryFailed(e)) => {
            error!("Failed to list blog posts: {}", e);
            ApiResponse::internal_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::adapter::outgoing::jwt::{JwtConfig, JwtTokenService};
    use crate::auth::application::ports::outgoing::token_provider::TokenProvider;
    use crate::blog::application::ports::incoming::use_cases::GetBlogPostsUseCase;
    use crate::blog::application::ports::outgoing::BlogPostCard;
    use crate::tests::support::app_state_builder::TestAppStateBuilder;
    use actix_web::{http::StatusCode, test, App};
    use async_trait::async_trait;
    use chrono::Utc;
    use serde_json::Value;
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    struct MockList {
        seen: Mutex<Option<(BlogPostListFilter, BlogPageRequest)>>,
    }

    #[async_trait]
    impl GetBlogPostsUseCase for MockList {
        async fn execute(
            &self,
            _o: UserId,
            filter: BlogPostListFilter,
            _s: BlogPostSort,
            page: BlogPageRequest,
        ) -> Result<BlogPageResult<BlogPostCard>, GetBlogPostsError> {
            *self.seen.lock().unwrap() = Some((filter.clone(), page.clone()));
            let now = Utc::now();
            Ok(BlogPageResult {
                items: vec![BlogPostCard {
                    id: Uuid::new_v4(),
                    title: "Post".into(),
                    slug: "post".into(),
                    excerpt: None,
                    published_at: None,
                    created_at: now,
                    updated_at: now,
                }],
                page: page.page,
                per_page: page.per_page,
                total: 1,
            })
        }
    }

    async fn call(query: &str) -> (actix_web::dev::ServiceResponse, Arc<MockList>) {
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

        let mock = Arc::new(MockList {
            seen: Mutex::new(None),
        });

        struct Delegate(Arc<MockList>);

        #[async_trait]
        impl GetBlogPostsUseCase for Delegate {
            async fn execute(
                &self,
                o: UserId,
                f: BlogPostListFilter,
                s: BlogPostSort,
                p: BlogPageRequest,
            ) -> Result<BlogPageResult<BlogPostCard>, GetBlogPostsError> {
                self.0.execute(o, f, s, p).await
            }
        }

        let app = test::init_service(
            App::new()
                .app_data(
                    TestAppStateBuilder::default()
                        .with_blog_list(Delegate(Arc::clone(&mock)))
                        .build(),
                )
                .app_data(web::Data::new(provider))
                .service(get_blog_posts_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/blog{query}"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();

        (test::call_service(&app, req).await, mock)
    }

    #[actix_web::test]
    async fn lists_posts() {
        let (resp, _) = call("").await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["data"]["total"], 1);
        assert_eq!(body["data"]["items"][0]["slug"], "post");
        // A listing row carries no content: the query does not select it.
        assert!(body["data"]["items"][0].get("content").is_none());
    }

    /// Zero and absent both mean "use the default", so a caller omitting
    /// pagination gets page 1 of 10 rather than an empty page 0.
    #[actix_web::test]
    async fn absent_pagination_falls_back_to_the_defaults() {
        let (_, mock) = call("").await;
        let (_, page) = mock.seen.lock().unwrap().clone().unwrap();
        assert_eq!(page.page, 1);
        assert_eq!(page.per_page, 10);
    }

    #[actix_web::test]
    async fn the_published_filter_is_passed_through_for_owner_listings() {
        let (_, mock) = call("?published=false").await;
        let (filter, _) = mock.seen.lock().unwrap().clone().unwrap();
        assert_eq!(filter.published, Some(false));
    }
}
