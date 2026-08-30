use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::blog::application::ports::outgoing::{
    BlogPageRequest, BlogPageResult, BlogPostArchiverError, BlogPostCard, BlogPostListFilter,
    BlogPostQueryError, BlogPostRepositoryError, BlogPostSort, BlogPostTopicRepositoryError,
    BlogPostView, PatchBlogPostData,
};
use crate::blog::domain::entities::{BlogPost, BlogPostTopic};

//
// ──────────────────────────────────────────────────────────
// Errors
//
// One enum per operation, so a handler matches only the outcomes its endpoint
// can actually produce, following the project module.
// ──────────────────────────────────────────────────────────
//

#[derive(Debug, Clone, thiserror::Error)]
pub enum CreateBlogPostError {
    #[error("Invalid title: {0}")]
    InvalidTitle(String),

    #[error("Invalid slug: {0}")]
    InvalidSlug(String),

    #[error("Invalid content: {0}")]
    InvalidContent(String),

    #[error("Slug already exists")]
    SlugAlreadyExists,

    #[error("Repository error: {0}")]
    RepositoryError(String),
}

impl From<BlogPostRepositoryError> for CreateBlogPostError {
    fn from(e: BlogPostRepositoryError) -> Self {
        match e {
            BlogPostRepositoryError::SlugAlreadyExists => CreateBlogPostError::SlugAlreadyExists,
            BlogPostRepositoryError::NotFound => {
                CreateBlogPostError::RepositoryError("post not found".to_string())
            }
            BlogPostRepositoryError::DatabaseError(m) => CreateBlogPostError::RepositoryError(m),
        }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum GetBlogPostsError {
    #[error("Query failed: {0}")]
    QueryFailed(String),
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum GetBlogPostError {
    #[error("Blog post not found")]
    NotFound,

    #[error("Query failed: {0}")]
    QueryFailed(String),
}

impl From<BlogPostQueryError> for GetBlogPostError {
    fn from(e: BlogPostQueryError) -> Self {
        match e {
            BlogPostQueryError::NotFound => GetBlogPostError::NotFound,
            BlogPostQueryError::DatabaseError(m) => GetBlogPostError::QueryFailed(m),
        }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum PatchBlogPostError {
    #[error("Blog post not found")]
    NotFound,

    #[error("Not the owner of this post")]
    Unauthorized,

    #[error("Invalid slug: {0}")]
    InvalidSlug(String),

    #[error("Slug already exists")]
    SlugAlreadyExists,

    #[error("Repository error: {0}")]
    RepositoryError(String),
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ArchiveBlogPostError {
    #[error("Blog post not found")]
    NotFound,

    #[error("Repository error: {0}")]
    RepositoryError(String),
}

impl From<BlogPostArchiverError> for ArchiveBlogPostError {
    fn from(e: BlogPostArchiverError) -> Self {
        match e {
            BlogPostArchiverError::NotFound => ArchiveBlogPostError::NotFound,
            BlogPostArchiverError::DatabaseError(m) => ArchiveBlogPostError::RepositoryError(m),
        }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum BlogPostTopicError {
    #[error("Blog post not found")]
    PostNotFound,

    #[error("Topic not found")]
    TopicNotFound,

    #[error("Repository error: {0}")]
    RepositoryError(String),
}

impl From<BlogPostTopicRepositoryError> for BlogPostTopicError {
    fn from(e: BlogPostTopicRepositoryError) -> Self {
        match e {
            BlogPostTopicRepositoryError::PostNotFound => BlogPostTopicError::PostNotFound,
            BlogPostTopicRepositoryError::TopicNotFound => BlogPostTopicError::TopicNotFound,
            BlogPostTopicRepositoryError::DatabaseError(m) => {
                BlogPostTopicError::RepositoryError(m)
            }
        }
    }
}

//
// ──────────────────────────────────────────────────────────
// Commands
// ──────────────────────────────────────────────────────────
//

#[derive(Debug, Clone)]
pub struct CreateBlogPostCommand {
    pub owner: UserId,
    pub title: String,
    pub slug: String,
    pub excerpt: Option<String>,
    pub content: String,
    pub published_at: Option<DateTime<Utc>>,
}

//
// ──────────────────────────────────────────────────────────
// Use cases
// ──────────────────────────────────────────────────────────
//

#[async_trait]
pub trait CreateBlogPostUseCase: Send + Sync {
    async fn execute(
        &self,
        command: CreateBlogPostCommand,
    ) -> Result<BlogPost, CreateBlogPostError>;
}

#[async_trait]
pub trait GetBlogPostsUseCase: Send + Sync {
    async fn execute(
        &self,
        owner: UserId,
        filter: BlogPostListFilter,
        sort: BlogPostSort,
        page: BlogPageRequest,
    ) -> Result<BlogPageResult<BlogPostCard>, GetBlogPostsError>;
}

#[async_trait]
pub trait GetPublicBlogPostsUseCase: Send + Sync {
    async fn execute(
        &self,
        owner: UserId,
        filter: BlogPostListFilter,
        sort: BlogPostSort,
        page: BlogPageRequest,
    ) -> Result<BlogPageResult<BlogPostCard>, GetBlogPostsError>;
}

#[async_trait]
pub trait GetSingleBlogPostUseCase: Send + Sync {
    async fn execute(
        &self,
        owner: UserId,
        post_id: Uuid,
    ) -> Result<BlogPostView, GetBlogPostError>;
}

#[async_trait]
pub trait GetPublicBlogPostUseCase: Send + Sync {
    async fn execute(&self, owner: UserId, slug: &str) -> Result<BlogPostView, GetBlogPostError>;
}

#[async_trait]
pub trait PatchBlogPostUseCase: Send + Sync {
    async fn execute(
        &self,
        owner: UserId,
        post_id: Uuid,
        data: PatchBlogPostData,
    ) -> Result<BlogPost, PatchBlogPostError>;
}

#[async_trait]
pub trait ArchiveBlogPostUseCase: Send + Sync {
    async fn execute(&self, owner: UserId, post_id: Uuid) -> Result<(), ArchiveBlogPostError>;
}

#[async_trait]
pub trait RestoreBlogPostUseCase: Send + Sync {
    async fn execute(&self, owner: UserId, post_id: Uuid) -> Result<(), ArchiveBlogPostError>;
}

#[async_trait]
pub trait HardDeleteBlogPostUseCase: Send + Sync {
    async fn execute(&self, owner: UserId, post_id: Uuid) -> Result<(), ArchiveBlogPostError>;
}

#[async_trait]
pub trait AttachBlogPostTopicUseCase: Send + Sync {
    async fn execute(
        &self,
        owner: UserId,
        post_id: Uuid,
        topic_id: Uuid,
    ) -> Result<(), BlogPostTopicError>;
}

#[async_trait]
pub trait DetachBlogPostTopicUseCase: Send + Sync {
    async fn execute(
        &self,
        owner: UserId,
        post_id: Uuid,
        topic_id: Uuid,
    ) -> Result<(), BlogPostTopicError>;
}

#[async_trait]
pub trait ClearBlogPostTopicsUseCase: Send + Sync {
    async fn execute(&self, owner: UserId, post_id: Uuid) -> Result<(), BlogPostTopicError>;
}

#[async_trait]
pub trait GetBlogPostTopicsUseCase: Send + Sync {
    async fn execute(
        &self,
        owner: UserId,
        post_id: Uuid,
    ) -> Result<Vec<BlogPostTopic>, GetBlogPostError>;
}
