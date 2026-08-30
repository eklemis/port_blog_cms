use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::blog::domain::entities::BlogPost;

#[derive(Debug, Clone, thiserror::Error)]
pub enum BlogPostRepositoryError {
    #[error("Blog post not found")]
    NotFound,

    #[error("Slug already exists for this author")]
    SlugAlreadyExists,

    #[error("Database error: {0}")]
    DatabaseError(String),
}

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
    pub fn is_unset(&self) -> bool {
        matches!(self, BlogPatchField::Unset)
    }

    pub fn as_value(&self) -> Option<&T> {
        if let BlogPatchField::Value(v) = self {
            Some(v)
        } else {
            None
        }
    }
}


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

#[async_trait]
pub trait BlogPostRepository: Send + Sync {
    async fn create(&self, data: CreateBlogPostData) -> Result<BlogPost, BlogPostRepositoryError>;

    /// Fetches a post regardless of publication state, including soft-deleted
    /// ones, so callers can perform ownership checks before acting.
    async fn fetch_by_id(
        &self,
        post_id: Uuid,
    ) -> Result<Option<BlogPost>, BlogPostRepositoryError>;

    async fn patch(
        &self,
        post_id: Uuid,
        data: PatchBlogPostData,
    ) -> Result<BlogPost, BlogPostRepositoryError>;
}
