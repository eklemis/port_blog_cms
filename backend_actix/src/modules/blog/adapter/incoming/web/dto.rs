//! Wire types for the blog endpoints.
//!
//! Kept in the adapter so `blog::domain` stays free of HTTP concerns, following
//! the split established for CV in 581c071.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::blog::application::ports::outgoing::{BlogPatchField, BlogPostCard};
use crate::blog::domain::entities::{BlogPost, BlogPostTopic};

/// Response body returned by this endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BlogPostResponse {
    /// Primary key.
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub id: Uuid,

    /// The owning user.
    #[schema(example = "987e6543-e21b-12d3-a456-426614174000")]
    pub user_id: Uuid,

    /// Display title.
    #[schema(example = "Building a CMS in Rust")]
    pub title: String,

    /// URL segment. Unique per owner.
    #[schema(example = "building-a-cms-in-rust")]
    pub slug: String,

    /// Short summary for listings. `None` when none was written.
    #[schema(example = "A walk through the hexagonal layout")]
    pub excerpt: Option<String>,

    /// The body.
    pub content: String,

    /// Null for a draft. A timestamp in the future means scheduled, not live.
    pub published_at: Option<DateTime<Utc>>,

    /// When it was created.
    pub created_at: DateTime<Utc>,
    /// When it was last edited.
    pub updated_at: DateTime<Utc>,
}

impl From<BlogPost> for BlogPostResponse {
    fn from(p: BlogPost) -> Self {
        Self {
            id: p.id,
            user_id: p.user_id,
            title: p.title,
            slug: p.slug,
            excerpt: p.excerpt,
            content: p.content,
            published_at: p.published_at,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

/// Response body returned by this endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BlogPostTopicResponse {
    /// Primary key.
    pub id: Uuid,
    /// Display title.
    #[schema(example = "Rust")]
    pub title: String,
    /// Long-form description.
    #[schema(example = "Posts about the Rust language")]
    pub description: String,
}

impl From<BlogPostTopic> for BlogPostTopicResponse {
    fn from(t: BlogPostTopic) -> Self {
        Self {
            id: t.id,
            title: t.title,
            description: t.description,
        }
    }
}

/// A post together with its topics, for detail views.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BlogPostDetailResponse {
    /// The post itself.
    #[serde(flatten)]
    pub post: BlogPostResponse,
    /// Topics attached to the post.
    pub topics: Vec<BlogPostTopicResponse>,
}

/// A listing row. Carries no `content` — that column is not selected for
/// listings, so exposing a field for it would be a lie.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BlogPostCardResponse {
    /// Primary key.
    pub id: Uuid,
    /// Display title.
    #[schema(example = "Building a CMS in Rust")]
    pub title: String,
    /// URL segment. Unique per owner.
    #[schema(example = "building-a-cms-in-rust")]
    pub slug: String,
    /// Short summary for listings. `None` when none was written.
    pub excerpt: Option<String>,
    /// `None` is a draft; a past value is published, a future one scheduled.
    pub published_at: Option<DateTime<Utc>>,
    /// When it was created.
    pub created_at: DateTime<Utc>,
    /// When it was last edited.
    pub updated_at: DateTime<Utc>,
}

impl From<BlogPostCard> for BlogPostCardResponse {
    fn from(c: BlogPostCard) -> Self {
        Self {
            id: c.id,
            title: c.title,
            slug: c.slug,
            excerpt: c.excerpt,
            published_at: c.published_at,
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }
}

/// Request body accepted by this endpoint.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateBlogPostRequest {
    /// Display title.
    #[schema(example = "Building a CMS in Rust", max_length = 200)]
    pub title: String,

    /// Lowercase letters, numbers and hyphens only. Unique per author.
    #[schema(example = "building-a-cms-in-rust", max_length = 200)]
    pub slug: String,

    /// Short summary for listings. `None` when none was written.
    #[schema(example = "A walk through the hexagonal layout")]
    pub excerpt: Option<String>,

    /// The body.
    pub content: String,

    /// Omit to create a draft. A future timestamp schedules the post.
    pub published_at: Option<DateTime<Utc>>,
}

/// Partial update.
///
/// Each field distinguishes three cases: omitting the key leaves the value
/// alone, `null` clears it, and a value sets it. Unpublishing a post back to
/// draft is a `null` on `published_at`. The slug cannot be cleared.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PatchBlogPostRequest {
    /// New title, if the client sent one.
    #[serde(default)]
    #[schema(value_type = Option<String>, example = "Building a CMS in Rust")]
    pub title: BlogPatchField<String>,

    /// New slug.
    #[serde(default)]
    #[schema(value_type = Option<String>, example = "building-a-cms-in-rust")]
    pub slug: BlogPatchField<String>,

    /// New excerpt. `null` clears it.
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub excerpt: BlogPatchField<String>,

    /// New body.
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub content: BlogPatchField<String>,

    /// `null` unpublishes back to draft; a value publishes or reschedules.
    #[serde(default)]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub published_at: BlogPatchField<DateTime<Utc>>,
}

/// Request body accepted by this endpoint.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BlogPostTopicRequest {
    /// The topic to attach or detach.
    #[schema(example = "9f1b2c3d-4e5f-6a7b-8c9d-0e1f2a3b4c5d")]
    pub topic_id: Uuid,
}
