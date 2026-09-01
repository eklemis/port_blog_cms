//! Read-side port for blog posts: listings, single fetches and topic links.
//!
//! Public and owner-facing listings share this port. The difference is the
//! filter: public callers always force `published = Some(true)`, owners may
//! ask for drafts.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::blog::domain::entities::{BlogPost, BlogPostTopic};

/// Why a blog-post read failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum BlogPostQueryError {
    #[error("Blog post not found")]
    NotFound,

    #[error("Database error: {0}")]
    DatabaseError(String),
}

/// Narrows a listing. Every field defaults to "no filter".
#[derive(Debug, Clone, Default)]
pub struct BlogPostListFilter {
    pub search: Option<String>,
    pub topic_id: Option<Uuid>,
    /// Owner-facing listings can ask for drafts only, published only, or both.
    /// Public listings ignore this and always force published.
    pub published: Option<bool>,
}

/// Listing order. Defaults to [`RecentlyPublished`](Self::RecentlyPublished),
/// which is what a blog index wants.
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema, Default)]
pub enum BlogPostSort {
    /// Newest by creation date.
    Newest,
    /// Oldest by creation date.
    Oldest,
    /// Most recently published first. Drafts, having no publication date,
    /// sort last.
    #[default]
    RecentlyPublished,
    /// Most recently edited first.
    RecentlyUpdated,
}

/// Which page to return. Pages are 1-based; defaults to 10 per page.
#[derive(Debug, Clone)]
pub struct BlogPageRequest {
    pub page: u32,
    pub per_page: u32,
}

impl Default for BlogPageRequest {
    fn default() -> Self {
        Self {
            page: 1,
            per_page: 10,
        }
    }
}

/// One page of results, plus the totals a client needs to paginate.
///
/// `total` counts every row matching the filter, not just this page.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BlogPageResult<T> {
    pub items: Vec<T>,

    #[schema(example = 1)]
    pub page: u32,

    #[schema(example = 10)]
    pub per_page: u32,

    #[schema(example = 42)]
    pub total: u64,
}

/// Listing row. Deliberately omits `content`, which is the largest column and
/// is never needed to render an index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlogPostCard {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub excerpt: Option<String>,
    pub published_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// A post plus its topics, for detail views.
#[derive(Debug, Clone, Serialize)]
pub struct BlogPostView {
    pub post: BlogPost,
    pub topics: Vec<BlogPostTopic>,
}

/// Reads blog posts.
///
/// Writes belong to [`BlogPostRepository`](super::blog_post_repository::BlogPostRepository).
/// Public callers must force `published = Some(true)` in the filter — this port
/// does not do it for them.
#[async_trait]
pub trait BlogPostQuery: Send + Sync {
    async fn list_by_owner(
        &self,
        owner: UserId,
        filter: BlogPostListFilter,
        sort: BlogPostSort,
        page: BlogPageRequest,
    ) -> Result<BlogPageResult<BlogPostCard>, BlogPostQueryError>;

    async fn get_by_id(
        &self,
        owner: UserId,
        post_id: Uuid,
    ) -> Result<BlogPostView, BlogPostQueryError>;

    /// Published-only lookup by author and slug, for the public endpoint.
    /// Drafts and scheduled posts are reported as not found.
    async fn get_published_by_slug(
        &self,
        owner: UserId,
        slug: &str,
    ) -> Result<BlogPostView, BlogPostQueryError>;

    async fn list_published(
        &self,
        owner: UserId,
        filter: BlogPostListFilter,
        sort: BlogPostSort,
        page: BlogPageRequest,
    ) -> Result<BlogPageResult<BlogPostCard>, BlogPostQueryError>;

    async fn get_topics(&self, post_id: Uuid) -> Result<Vec<BlogPostTopic>, BlogPostQueryError>;
}
