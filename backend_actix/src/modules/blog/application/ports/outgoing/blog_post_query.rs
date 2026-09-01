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
    /// No post matched. Only meaningful for single fetches.
    #[error("Blog post not found")]
    NotFound,

    /// The store could not be reached.
    #[error("Database error: {0}")]
    DatabaseError(String),
}

/// Narrows a listing. Every field defaults to "no filter".
#[derive(Debug, Clone, Default)]
pub struct BlogPostListFilter {
    /// Free-text filter. `None` matches everything.
    pub search: Option<String>,
    /// Restricts to posts carrying this topic. `None` matches everything.
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
    /// 1-based page number.
    pub page: u32,
    /// Rows per page.
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
    /// The rows on this page.
    pub items: Vec<T>,

    /// 1-based page number.
    #[schema(example = 1)]
    pub page: u32,

    /// Rows per page.
    #[schema(example = 10)]
    pub per_page: u32,

    /// Rows matching the filter across *all* pages, not just this one.
    #[schema(example = 42)]
    pub total: u64,
}

/// Listing row. Deliberately omits `content`, which is the largest column and
/// is never needed to render an index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlogPostCard {
    /// Primary key.
    pub id: Uuid,
    /// Display title.
    pub title: String,
    /// URL segment. Unique per author.
    pub slug: String,
    /// Short summary for listings. `None` when the author wrote none.
    pub excerpt: Option<String>,
    /// Publication state, carried as a timestamp rather than a status column:
    /// `None` is a draft, a past value is published, a future one is scheduled.
    pub published_at: Option<chrono::DateTime<chrono::Utc>>,
    /// When the post was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When it was last edited.
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// A media item attached to a post, projected for a **public** response.
///
/// The URLs are unsigned and durable, unlike the console's signed ones — see
/// `docs/adr/0006-public-media-urls.md` for why a public page cannot use a
/// signed URL.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
pub struct PublicMedia {
    /// The media item's identifier.
    #[schema(example = "8f1b2c3d-4e5f-6a7b-8c9d-0e1f2a3b4c5d")]
    pub media_id: Uuid,

    /// Alternative text. Empty rather than absent when the author set none —
    /// which is itself worth surfacing in the console.
    #[schema(example = "Hexagonal layout of the API")]
    pub alt_text: String,

    /// Caption. Empty rather than absent when unset.
    pub caption: String,

    /// What the media is for on this post — `cover`, `gallery`, `inline`.
    #[schema(example = "cover")]
    pub role: String,

    /// Display order within the role, ascending from 0.
    pub position: i32,

    /// One entry per generated size, keyed by size name: `thumbnail`, `small`,
    /// `medium`, `large`.
    ///
    /// **May be empty**, and a caller must handle that: a media row exists
    /// before its variants do, so a post published while its cover is still
    /// processing carries the attachment with no URLs yet.
    #[schema(example = json!({"thumbnail": "https://storage.googleapis.com/…"}))]
    pub variants: std::collections::BTreeMap<String, String>,
}

/// A post plus its topics, for detail views.
#[derive(Debug, Clone, Serialize)]
pub struct BlogPostView {
    /// The post itself.
    pub post: BlogPost,
    /// Topics attached to it.
    pub topics: Vec<BlogPostTopic>,

    /// Media attached to it, ordered by role then position.
    ///
    /// Populated only on the **public** read path; the console's reads leave it
    /// empty and go through the media endpoints, which return signed URLs.
    pub media: Vec<PublicMedia>,
}

/// Reads blog posts.
///
/// Writes belong to [`BlogPostRepository`](super::blog_post_repository::BlogPostRepository).
/// Public callers must force `published = Some(true)` in the filter — this port
/// does not do it for them.
#[async_trait]
pub trait BlogPostQuery: Send + Sync {
    /// Lists an author's posts, drafts included, honouring `filter.published`.
    async fn list_by_owner(
        &self,
        owner: UserId,
        filter: BlogPostListFilter,
        sort: BlogPostSort,
        page: BlogPageRequest,
    ) -> Result<BlogPageResult<BlogPostCard>, BlogPostQueryError>;

    /// Fetches one post by id regardless of publication state.
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

    /// Lists only published posts. Public listings must use this rather than
    /// [`list_by_owner`](Self::list_by_owner) with a filter, so a caller cannot
    /// forget to exclude drafts.
    async fn list_published(
        &self,
        owner: UserId,
        filter: BlogPostListFilter,
        sort: BlogPostSort,
        page: BlogPageRequest,
    ) -> Result<BlogPageResult<BlogPostCard>, BlogPostQueryError>;

    /// Lists the topics attached to a post.
    async fn get_topics(&self, post_id: Uuid) -> Result<Vec<BlogPostTopic>, BlogPostQueryError>;
}
