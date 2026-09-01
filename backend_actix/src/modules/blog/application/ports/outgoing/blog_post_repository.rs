//! Write-side port for blog posts.
//!
//! Publication state is carried by `published_at` rather than a status
//! column: `None` is a draft, a past timestamp is published, a future one is
//! scheduled. That is why patching needs [`BlogPatchField`] — `Option` alone
//! cannot distinguish "leave the date alone" from "unpublish this".

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::blog::domain::entities::BlogPost;

/// Why a blog-post write failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum BlogPostRepositoryError {
    /// No post matched the id.
    #[error("Blog post not found")]
    NotFound,

    /// The author already has a post with that slug.
    ///
    /// Slugs are unique per author, not globally, so two users may both have
    /// `/hello-world`.
    #[error("Slug already exists for this author")]
    SlugAlreadyExists,

    /// The store failed for a reason this port does not model.
    #[error("Database error: {0}")]
    DatabaseError(String),
}

/// Everything needed to insert a post.
#[derive(Debug, Clone)]
pub struct CreateBlogPostData {
    pub owner: UserId,
    pub title: String,
    pub slug: String,
    pub excerpt: Option<String>,
    pub content: String,
    /// `None` creates a draft. Callers publish by passing a timestamp, which
    /// may be in the future to schedule.
    pub published_at: Option<DateTime<Utc>>,
}

/// Distinguishes "leave alone" from "set to null", which `Option` alone
/// cannot. Mirrors `PatchField` in the project module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BlogPatchField<T> {
    #[serde(skip)]
    Unset,
    Null,
    Value(T),
}

impl<T> Default for BlogPatchField<T> {
    fn default() -> Self {
        BlogPatchField::Unset
    }
}

impl<T> BlogPatchField<T> {
    /// True when the client did not mention this field at all.
    pub fn is_unset(&self) -> bool {
        matches!(self, BlogPatchField::Unset)
    }

    /// The new value, if one was supplied. `Null` and `Unset` both yield
    /// `None` — use [`is_unset`](Self::is_unset) to tell them apart.
    pub fn as_value(&self) -> Option<&T> {
        if let BlogPatchField::Value(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

/// A partial update. Every field defaults to
/// [`Unset`](BlogPatchField::Unset), so omitted fields are left alone.
#[derive(Debug, Clone, Default)]
pub struct PatchBlogPostData {
    pub title: BlogPatchField<String>,
    pub slug: BlogPatchField<String>,
    pub excerpt: BlogPatchField<String>,
    pub content: BlogPatchField<String>,
    /// Setting `Null` unpublishes a post back to draft; setting a value
    /// publishes or reschedules it.
    pub published_at: BlogPatchField<DateTime<Utc>>,
}

/// Creates and edits blog posts.
///
/// Reads for listing and public display belong to
/// [`BlogPostQuery`](super::blog_post_query::BlogPostQuery); lifecycle
/// transitions to [`BlogPostArchiver`](super::blog_post_archiver::BlogPostArchiver).
#[async_trait]
pub trait BlogPostRepository: Send + Sync {
    /// Inserts a post.
    ///
    /// # Errors
    /// [`SlugAlreadyExists`](BlogPostRepositoryError::SlugAlreadyExists) if the
    /// author already uses that slug.
    async fn create(&self, data: CreateBlogPostData) -> Result<BlogPost, BlogPostRepositoryError>;

    /// Fetches a post regardless of publication state, including soft-deleted
    /// ones, so callers can perform ownership checks before acting.
    async fn fetch_by_id(&self, post_id: Uuid)
        -> Result<Option<BlogPost>, BlogPostRepositoryError>;

    /// Applies a partial update and returns the post as stored.
    ///
    /// Does not check ownership — callers must fetch and verify first.
    ///
    /// # Errors
    /// [`NotFound`](BlogPostRepositoryError::NotFound) if the post is gone,
    /// [`SlugAlreadyExists`](BlogPostRepositoryError::SlugAlreadyExists) if the
    /// patch moves it onto a slug the author already uses.
    async fn patch(
        &self,
        post_id: Uuid,
        data: PatchBlogPostData,
    ) -> Result<BlogPost, BlogPostRepositoryError>;
}
