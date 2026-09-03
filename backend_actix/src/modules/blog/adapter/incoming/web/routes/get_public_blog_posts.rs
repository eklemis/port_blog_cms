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
            items: result
                .items
                .into_iter()
                .map(Into::into)
                .collect::<Vec<BlogPostCardResponse>>(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::application::helpers::UserIdentityResolver;
    use crate::auth::application::ports::outgoing::user_query::{
        UserQuery, UserQueryError, UserQueryResult,
    };
    use crate::blog::application::ports::incoming::use_cases::GetPublicBlogPostsUseCase;
    use crate::blog::application::ports::outgoing::{
        BlogPageRequest, BlogPostCard, BlogPostListFilter, BlogPostSort,
    };
    use crate::tests::support::app_state_builder::TestAppStateBuilder;
    use actix_web::{http::StatusCode, test, App};
    use async_trait::async_trait;
    use chrono::Utc;
    use serde_json::Value;
    use std::sync::{Arc, Mutex};
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

    struct SpyPublicList {
        seen: Mutex<Option<BlogPostListFilter>>,
    }

    #[async_trait]
    impl GetPublicBlogPostsUseCase for SpyPublicList {
        async fn execute(
            &self,
            _o: UserId,
            filter: BlogPostListFilter,
            _s: BlogPostSort,
            page: BlogPageRequest,
        ) -> Result<BlogPageResult<BlogPostCard>, GetBlogPostsError> {
            *self.seen.lock().unwrap() = Some(filter);
            let now = Utc::now();
            Ok(BlogPageResult {
                items: vec![BlogPostCard {
                    cover: None,
                    id: Uuid::new_v4(),
                    title: "Published".into(),
                    slug: "published".into(),
                    excerpt: None,
                    published_at: Some(now),
                    created_at: now,
                    updated_at: now,
                }],
                page: page.page,
                per_page: page.per_page,
                total: 1,
            })
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
            bio: None,
            locale: "en".to_string(),
        }
    }

    async fn call(
        user: Result<Option<UserQueryResult>, UserQueryError>,
        query: &str,
    ) -> (actix_web::dev::ServiceResponse, Arc<SpyPublicList>) {
        let spy = Arc::new(SpyPublicList {
            seen: Mutex::new(None),
        });

        struct Delegate(Arc<SpyPublicList>);

        #[async_trait]
        impl GetPublicBlogPostsUseCase for Delegate {
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

        let resolver = UserIdentityResolver::new(Arc::new(MockUserQuery { result: user }));

        let app = test::init_service(
            App::new()
                .app_data(
                    TestAppStateBuilder::default()
                        .with_user_identity_resolver(resolver)
                        .with_blog_list_public(Delegate(Arc::clone(&spy)))
                        .build(),
                )
                .service(get_public_blog_posts_handler),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/public/blog/author{query}"))
            .to_request();

        (test::call_service(&app, req).await, spy)
    }

    /// No Authorization header: a public index has to serve anonymous readers.
    #[actix_web::test]
    async fn lists_published_posts_without_authentication() {
        let (resp, _) = call(Ok(Some(a_user(Uuid::new_v4()))), "").await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["data"]["total"], 1);
        assert_eq!(body["data"]["items"][0]["slug"], "published");
    }

    #[actix_web::test]
    async fn an_unknown_author_is_not_found() {
        let (resp, _) = call(Ok(None), "").await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    /// `published=false` is accepted by the query DTO and reaches the use case,
    /// but the public path calls `list_published`, which ignores it. This pins
    /// the routing half of that guarantee — the adapter half is pinned in
    /// blog_post_query_postgres.
    #[actix_web::test]
    async fn a_drafts_only_filter_cannot_reach_the_published_only_query() {
        let (resp, spy) = call(Ok(Some(a_user(Uuid::new_v4()))), "?published=false").await;
        assert_eq!(resp.status(), StatusCode::OK);

        let filter = spy.seen.lock().unwrap().clone().unwrap();
        assert_eq!(filter.published, Some(false));

        // The handler routed to list_public regardless, which is the use case
        // backed by the published-only query.
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["data"]["total"], 1);
    }
}
