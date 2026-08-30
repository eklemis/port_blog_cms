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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BlogPostResponse {
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub id: Uuid,

    #[schema(example = "987e6543-e21b-12d3-a456-426614174000")]
    pub user_id: Uuid,

    #[schema(example = "Building a CMS in Rust")]
    pub title: String,

    #[schema(example = "building-a-cms-in-rust")]
    pub slug: String,

    #[schema(example = "A walk through the hexagonal layout")]
    pub excerpt: Option<String>,

    pub content: String,

    /// Null for a draft. A timestamp in the future means scheduled, not live.
    pub published_at: Option<DateTime<Utc>>,

    pub created_at: DateTime<Utc>,
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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BlogPostTopicResponse {
    pub id: Uuid,
    #[schema(example = "Rust")]
    pub title: String,
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
    #[serde(flatten)]
    pub post: BlogPostResponse,
    pub topics: Vec<BlogPostTopicResponse>,
}

/// A listing row. Carries no `content` — that column is not selected for
/// listings, so exposing a field for it would be a lie.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BlogPostCardResponse {
    pub id: Uuid,
    #[schema(example = "Building a CMS in Rust")]
    pub title: String,
    #[schema(example = "building-a-cms-in-rust")]
    pub slug: String,
    pub excerpt: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
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

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateBlogPostRequest {
    #[schema(example = "Building a CMS in Rust", max_length = 200)]
    pub title: String,

    /// Lowercase letters, numbers and hyphens only. Unique per author.
    #[schema(example = "building-a-cms-in-rust", max_length = 200)]
    pub slug: String,

    #[schema(example = "A walk through the hexagonal layout")]
    pub excerpt: Option<String>,

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
    #[serde(default)]
    #[schema(value_type = Option<String>, example = "Building a CMS in Rust")]
    pub title: BlogPatchField<String>,

    #[serde(default)]
    #[schema(value_type = Option<String>, example = "building-a-cms-in-rust")]
    pub slug: BlogPatchField<String>,

    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub excerpt: BlogPatchField<String>,

    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub content: BlogPatchField<String>,

    #[serde(default)]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub published_at: BlogPatchField<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BlogPostTopicRequest {
    #[schema(example = "9f1b2c3d-4e5f-6a7b-8c9d-0e1f2a3b4c5d")]
    pub topic_id: Uuid,
}
