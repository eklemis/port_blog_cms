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
    async fn execute(&self, owner: UserId, post_id: Uuid)
        -> Result<BlogPostView, GetBlogPostError>;
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Each From impl exists so a handler matches only outcomes its endpoint can
    /// produce. A wrong arm here changes an HTTP status — a slug clash becoming
    /// a 500 instead of a 409, say — which no route test would catch, since they
    /// construct the incoming error directly.
    #[test]
    fn repository_errors_map_onto_create_variants() {
        assert!(matches!(
            CreateBlogPostError::from(BlogPostRepositoryError::SlugAlreadyExists),
            CreateBlogPostError::SlugAlreadyExists
        ));
        assert!(matches!(
            CreateBlogPostError::from(BlogPostRepositoryError::DatabaseError("db".into())),
            CreateBlogPostError::RepositoryError(m) if m == "db"
        ));
        // NotFound has no meaning when creating, so it degrades to a repository
        // error rather than silently becoming a success or a slug clash.
        assert!(matches!(
            CreateBlogPostError::from(BlogPostRepositoryError::NotFound),
            CreateBlogPostError::RepositoryError(_)
        ));
    }

    #[test]
    fn query_errors_map_onto_get_variants() {
        assert!(matches!(
            GetBlogPostError::from(BlogPostQueryError::NotFound),
            GetBlogPostError::NotFound
        ));
        assert!(matches!(
            GetBlogPostError::from(BlogPostQueryError::DatabaseError("db".into())),
            GetBlogPostError::QueryFailed(m) if m == "db"
        ));
    }

    #[test]
    fn archiver_errors_map_onto_archive_variants() {
        assert!(matches!(
            ArchiveBlogPostError::from(BlogPostArchiverError::NotFound),
            ArchiveBlogPostError::NotFound
        ));
        assert!(matches!(
            ArchiveBlogPostError::from(BlogPostArchiverError::DatabaseError("db".into())),
            ArchiveBlogPostError::RepositoryError(m) if m == "db"
        ));
    }

    /// PostNotFound and TopicNotFound must stay separate: the route reports
    /// different error codes for each, which is the only way a caller can tell
    /// whether the post or the topic was the problem.
    #[test]
    fn topic_repository_errors_keep_post_and_topic_distinct() {
        assert!(matches!(
            BlogPostTopicError::from(BlogPostTopicRepositoryError::PostNotFound),
            BlogPostTopicError::PostNotFound
        ));
        assert!(matches!(
            BlogPostTopicError::from(BlogPostTopicRepositoryError::TopicNotFound),
            BlogPostTopicError::TopicNotFound
        ));
        assert!(matches!(
            BlogPostTopicError::from(BlogPostTopicRepositoryError::DatabaseError("db".into())),
            BlogPostTopicError::RepositoryError(m) if m == "db"
        ));
    }

    /// The Display strings reach clients in error bodies.
    #[test]
    fn error_messages_are_human_readable() {
        assert_eq!(
            CreateBlogPostError::InvalidSlug("bad".into()).to_string(),
            "Invalid slug: bad"
        );
        assert_eq!(
            GetBlogPostError::NotFound.to_string(),
            "Blog post not found"
        );
        assert_eq!(
            PatchBlogPostError::Unauthorized.to_string(),
            "Not the owner of this post"
        );
        assert_eq!(
            BlogPostTopicError::TopicNotFound.to_string(),
            "Topic not found"
        );
    }
}
