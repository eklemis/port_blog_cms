//! Read-side services.
//!
//! These are thin: the query adapter owns filtering and pagination, so each
//! service exists to map an outgoing error onto the error type its endpoint
//! speaks, and — for the public variants — to pick the published-only query.

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::blog::application::ports::incoming::use_cases::{
    GetBlogPostError, GetBlogPostTopicsUseCase, GetBlogPostsError, GetBlogPostsUseCase,
    GetPublicBlogPostUseCase, GetPublicBlogPostsUseCase, GetSingleBlogPostUseCase,
};
use crate::blog::application::ports::outgoing::{
    BlogPageRequest, BlogPageResult, BlogPostCard, BlogPostListFilter, BlogPostQuery,
    BlogPostQueryError, BlogPostSort, BlogPostView,
};
use crate::blog::domain::entities::BlogPostTopic;

macro_rules! query_service {
    ($name:ident) => {
        pub struct $name<Q>
        where
            Q: BlogPostQuery,
        {
            query: Q,
        }

        impl<Q> $name<Q>
        where
            Q: BlogPostQuery,
        {
            pub fn new(query: Q) -> Self {
                Self { query }
            }
        }
    };
}

query_service!(GetBlogPostsService);
query_service!(GetPublicBlogPostsService);
query_service!(GetSingleBlogPostService);
query_service!(GetPublicBlogPostService);
query_service!(GetBlogPostTopicsService);

fn list_err(e: BlogPostQueryError) -> GetBlogPostsError {
    match e {
        BlogPostQueryError::NotFound => {
            GetBlogPostsError::QueryFailed("post not found".to_string())
        }
        BlogPostQueryError::DatabaseError(m) => GetBlogPostsError::QueryFailed(m),
    }
}

#[async_trait]
impl<Q> GetBlogPostsUseCase for GetBlogPostsService<Q>
where
    Q: BlogPostQuery + Send + Sync,
{
    async fn execute(
        &self,
        owner: UserId,
        filter: BlogPostListFilter,
        sort: BlogPostSort,
        page: BlogPageRequest,
    ) -> Result<BlogPageResult<BlogPostCard>, GetBlogPostsError> {
        self.query
            .list_by_owner(owner, filter, sort, page)
            .await
            .map_err(list_err)
    }
}

#[async_trait]
impl<Q> GetPublicBlogPostsUseCase for GetPublicBlogPostsService<Q>
where
    Q: BlogPostQuery + Send + Sync,
{
    async fn execute(
        &self,
        owner: UserId,
        filter: BlogPostListFilter,
        sort: BlogPostSort,
        page: BlogPageRequest,
    ) -> Result<BlogPageResult<BlogPostCard>, GetBlogPostsError> {
        // Deliberately the published-only query. The filter still arrives from
        // the query string, and the adapter ignores its `published` field here.
        self.query
            .list_published(owner, filter, sort, page)
            .await
            .map_err(list_err)
    }
}

#[async_trait]
impl<Q> GetSingleBlogPostUseCase for GetSingleBlogPostService<Q>
where
    Q: BlogPostQuery + Send + Sync,
{
    async fn execute(
        &self,
        owner: UserId,
        post_id: Uuid,
    ) -> Result<BlogPostView, GetBlogPostError> {
        self.query
            .get_by_id(owner, post_id)
            .await
            .map_err(GetBlogPostError::from)
    }
}

#[async_trait]
impl<Q> GetPublicBlogPostUseCase for GetPublicBlogPostService<Q>
where
    Q: BlogPostQuery + Send + Sync,
{
    async fn execute(&self, owner: UserId, slug: &str) -> Result<BlogPostView, GetBlogPostError> {
        self.query
            .get_published_by_slug(owner, slug)
            .await
            .map_err(GetBlogPostError::from)
    }
}

#[async_trait]
impl<Q> GetBlogPostTopicsUseCase for GetBlogPostTopicsService<Q>
where
    Q: BlogPostQuery + Send + Sync,
{
    async fn execute(
        &self,
        owner: UserId,
        post_id: Uuid,
    ) -> Result<Vec<BlogPostTopic>, GetBlogPostError> {
        // Resolve the post first so a caller cannot read the topics of a post
        // they do not own, or of one that is archived.
        self.query
            .get_by_id(owner, post_id)
            .await
            .map_err(GetBlogPostError::from)?;

        self.query
            .get_topics(post_id)
            .await
            .map_err(GetBlogPostError::from)
    }
}
