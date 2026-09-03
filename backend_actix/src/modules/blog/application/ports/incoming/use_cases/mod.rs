//! The blog use-case contracts, one per operation.
//!
//! Each operation gets its own error enum rather than sharing one, so a
//! handler matches only the outcomes its endpoint can actually produce and the
//! compiler catches a missing arm. The `From` impls in this file are where an
//! outgoing-port error is narrowed to that set.

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
use crate::shared::api::{BulkOutcome, BulkRequestError};

/// Why creating a post failed.
///
/// The three `Invalid*` variants come from domain validation, before the
/// repository is touched.
#[derive(Debug, Clone, thiserror::Error)]
pub enum CreateBlogPostError {
    /// The title failed domain validation. Caught before the repository is
    /// touched; the payload says which rule.
    #[error("Invalid title: {0}")]
    InvalidTitle(String),

    /// The slug is empty, too long, or has characters outside `[a-z0-9-]`.
    #[error("Invalid slug: {0}")]
    InvalidSlug(String),

    /// The body is empty or otherwise unacceptable.
    #[error("Invalid content: {0}")]
    InvalidContent(String),

    /// The author already has a post with that slug.
    #[error("Slug already exists")]
    SlugAlreadyExists,

    /// The store could not be reached, or failed for a reason this operation
    /// does not model.
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

/// Why a post listing failed.
///
/// A listing that matches nothing is an empty page, not an error, so the only
/// failure is the store itself.
#[derive(Debug, Clone, thiserror::Error)]
pub enum GetBlogPostsError {
    /// The read could not be executed.
    #[error("Query failed: {0}")]
    QueryFailed(String),
}

/// Why fetching a single post failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum GetBlogPostError {
    /// No post matched.
    #[error("Blog post not found")]
    NotFound,

    /// The read could not be executed.
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

/// Why patching a post failed.
///
/// Unlike the archive operations, this distinguishes
/// [`Unauthorized`](Self::Unauthorized) from [`NotFound`](Self::NotFound): the
/// patch path fetches the post first to check ownership, so it knows which of
/// the two happened. The archivers scope on owner in SQL and cannot tell.
#[derive(Debug, Clone, thiserror::Error)]
pub enum PatchBlogPostError {
    /// No post matched.
    #[error("Blog post not found")]
    NotFound,

    /// The post exists but belongs to another user. Distinguishable here because
    /// the patch path fetches the post before writing; the archive operations
    /// cannot tell, and report [`NotFound`](Self::NotFound) instead.
    #[error("Not the owner of this post")]
    Unauthorized,

    /// The slug is empty, too long, or has characters outside `[a-z0-9-]`.
    #[error("Invalid slug: {0}")]
    InvalidSlug(String),

    /// The author already has a post with that slug.
    #[error("Slug already exists")]
    SlugAlreadyExists,

    /// The store could not be reached, or failed for a reason this operation
    /// does not model.
    #[error("Repository error: {0}")]
    RepositoryError(String),
}

/// Why archiving, restoring or hard-deleting a post failed.
///
/// Shared by all three lifecycle use cases. There is no `Unauthorized`
/// variant: [`BlogPostArchiver`](crate::blog::application::ports::outgoing::BlogPostArchiver)
/// scopes on owner in SQL, so another user's post is indistinguishable from a
/// missing one — which is what stops this confirming that someone else's post
/// exists.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ArchiveBlogPostError {
    /// No post matched.
    #[error("Blog post not found")]
    NotFound,

    /// The store could not be reached, or failed for a reason this operation
    /// does not model.
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

/// Why a post-topic link operation failed.
///
/// Shared by the four topic-link use cases. Distinguishes a missing post from
/// a missing topic so the handler can say which id was wrong.
#[derive(Debug, Clone, thiserror::Error)]
pub enum BlogPostTopicError {
    /// No post matched the id, or it belongs to another user.
    #[error("Blog post not found")]
    PostNotFound,

    /// No topic matched the id.
    #[error("Topic not found")]
    TopicNotFound,

    /// The store could not be reached, or failed for a reason this operation
    /// does not model.
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

/// Everything needed to create a post.
///
/// `published_at` carries the publication state: `None` is a draft, a past
/// timestamp is published, a future one is scheduled.
#[derive(Debug, Clone)]
pub struct CreateBlogPostCommand {
    /// The user the post will belong to.
    pub owner: UserId,
    /// Display title.
    pub title: String,
    /// URL segment. Unique per author.
    pub slug: String,
    /// Short summary for listings.
    pub excerpt: Option<String>,
    /// The post body.
    pub content: String,
    /// `None` creates a draft. A timestamp publishes, and one in the future
    /// schedules.
    pub published_at: Option<DateTime<Utc>>,
}

/// Creates a post.
#[async_trait]
pub trait CreateBlogPostUseCase: Send + Sync {
    /// Runs the operation.
    async fn execute(
        &self,
        command: CreateBlogPostCommand,
    ) -> Result<BlogPost, CreateBlogPostError>;
}

/// Lists an author's own posts.
///
/// Honours `filter.published`, so the author can ask for drafts, published
/// posts, or both.
#[async_trait]
pub trait GetBlogPostsUseCase: Send + Sync {
    /// Runs the operation.
    async fn execute(
        &self,
        owner: UserId,
        filter: BlogPostListFilter,
        sort: BlogPostSort,
        page: BlogPageRequest,
    ) -> Result<BlogPageResult<BlogPostCard>, GetBlogPostsError>;
}

/// Lists an author's posts for a public reader.
///
/// Identical in signature to [`GetBlogPostsUseCase`] and different in one
/// respect that matters: the implementation **forces published-only** and
/// ignores `filter.published`. Do not swap the two — wiring the owner-facing
/// use case into a public route would leak drafts.
#[async_trait]
pub trait GetPublicBlogPostsUseCase: Send + Sync {
    /// Runs the operation.
    async fn execute(
        &self,
        owner: UserId,
        filter: BlogPostListFilter,
        sort: BlogPostSort,
        page: BlogPageRequest,
    ) -> Result<BlogPageResult<BlogPostCard>, GetBlogPostsError>;
}

/// Fetches one of the author's own posts by id, draft or published.
#[async_trait]
pub trait GetSingleBlogPostUseCase: Send + Sync {
    /// Runs the operation.
    async fn execute(&self, owner: UserId, post_id: Uuid)
        -> Result<BlogPostView, GetBlogPostError>;
}

/// Fetches one published post by slug, for a public reader.
///
/// Addressed by slug rather than id because that is what appears in a public
/// URL. Unpublished posts are reported as
/// [`NotFound`](GetBlogPostError::NotFound) rather than forbidden, so a draft's
/// slug cannot be probed for.
#[async_trait]
pub trait GetPublicBlogPostUseCase: Send + Sync {
    /// Runs the operation.
    async fn execute(&self, owner: UserId, slug: &str) -> Result<BlogPostView, GetBlogPostError>;
}

/// Applies a partial update, after checking the caller owns the post.
#[async_trait]
pub trait PatchBlogPostUseCase: Send + Sync {
    /// Runs the operation.
    async fn execute(
        &self,
        owner: UserId,
        post_id: Uuid,
        data: PatchBlogPostData,
    ) -> Result<BlogPost, PatchBlogPostError>;
}

/// Hides a post without deleting it. Reversible with
/// [`RestoreBlogPostUseCase`].
#[async_trait]
pub trait ArchiveBlogPostUseCase: Send + Sync {
    /// Runs the operation.
    async fn execute(&self, owner: UserId, post_id: Uuid) -> Result<(), ArchiveBlogPostError>;
}

/// Un-archives a post. Publication state is untouched — a restored post
/// returns as the draft or published post it was.
#[async_trait]
pub trait RestoreBlogPostUseCase: Send + Sync {
    /// Runs the operation.
    async fn execute(&self, owner: UserId, post_id: Uuid) -> Result<(), ArchiveBlogPostError>;
}

/// Takes a post down, back to draft. Reversible by publishing again.
#[async_trait]
pub trait UnpublishBlogPostUseCase: Send + Sync {
    /// Idempotent: unpublishing a draft succeeds.
    async fn execute(&self, owner: UserId, post_id: Uuid) -> Result<(), ArchiveBlogPostError>;
}

/// Removes a post and its topic links permanently. Irreversible.
#[async_trait]
pub trait HardDeleteBlogPostUseCase: Send + Sync {
    /// Runs the operation.
    async fn execute(&self, owner: UserId, post_id: Uuid) -> Result<(), ArchiveBlogPostError>;
}

/// Links a topic to a post. Idempotent.
#[async_trait]
pub trait AttachBlogPostTopicUseCase: Send + Sync {
    /// Runs the operation.
    async fn execute(
        &self,
        owner: UserId,
        post_id: Uuid,
        topic_id: Uuid,
    ) -> Result<(), BlogPostTopicError>;
}

/// Removes one topic link. Removing a link that is not there succeeds.
#[async_trait]
pub trait DetachBlogPostTopicUseCase: Send + Sync {
    /// Runs the operation.
    async fn execute(
        &self,
        owner: UserId,
        post_id: Uuid,
        topic_id: Uuid,
    ) -> Result<(), BlogPostTopicError>;
}

/// Removes every topic link from a post.
#[async_trait]
pub trait ClearBlogPostTopicsUseCase: Send + Sync {
    /// Runs the operation.
    async fn execute(&self, owner: UserId, post_id: Uuid) -> Result<(), BlogPostTopicError>;
}

/// Reports whether an author already uses a slug.
///
/// Returns the *taken* state rather than the available one, so the name and
/// the boolean cannot drift apart in a caller's head.
#[async_trait]
pub trait SlugAvailableUseCase: Send + Sync {
    /// True when the author already has a post with this slug.
    async fn execute(&self, owner: UserId, slug: String) -> Result<bool, GetBlogPostsError>;
}

/// Lists the topics attached to a post.
#[async_trait]
pub trait GetBlogPostTopicsUseCase: Send + Sync {
    /// Runs the operation.
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

/// One operation applied across many posts.
///
/// Modelled as a tagged enum rather than an operation string beside an optional
/// `topic_id`, so "attach, but no topic given" cannot be expressed — the
/// request fails to deserialise instead of failing halfway through a batch.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, utoipa::ToSchema)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum BlogBulkOp {
    /// Hide each post. Reversible.
    Archive,
    /// Un-hide each post.
    Restore,
    /// Remove each post permanently.
    HardDelete,
    /// Take each post down, back to draft.
    ///
    /// There is no bulk `publish`, and that is deliberate rather than an
    /// omission. Publishing is per-post considered work — the slug, the cover,
    /// the topics — and it carries an ambiguity this operation does not: does
    /// "publish these" mean now, or at each post's scheduled time? Taking
    /// several down at once has no such question, is reversible, and is
    /// exactly the moment you do not want to click six times.
    Unpublish,
    /// Link one topic to each post.
    AttachTopic {
        /// The topic to link.
        topic_id: Uuid,
    },
    /// Remove one topic's link from each post.
    DetachTopic {
        /// The topic to unlink.
        topic_id: Uuid,
    },
}

/// Applies one operation to many posts, reporting per item.
///
/// Ownership is not re-checked here — each single-item use case this composes
/// already scopes its write to the owner and answers `NotFound` for a post
/// belonging to someone else. That is what keeps a bulk endpoint from becoming
/// a way to delete another author's posts by guessing ids.
#[async_trait]
pub trait BulkBlogPostsUseCase: Send + Sync {
    /// Runs the operation over `ids`, in order.
    async fn execute(
        &self,
        owner: UserId,
        op: BlogBulkOp,
        ids: Vec<Uuid>,
    ) -> Result<BulkOutcome, BulkRequestError>;
}

/// How long a freshly minted or renewed preview link lasts.
///
/// Generous on purpose. A draft under review sits for a week or two, and a
/// 24-hour link dies over a weekend — the author then learns about it from a
/// reviewer who got an error, which is the worst way to find out. Expiry is not
/// the problem; expiry the author cannot see is, which is why
/// [`DraftPreviewState`] reports it.
pub const DRAFT_PREVIEW_TTL_DAYS: i64 = 14;

/// The author-facing view of a post's sharing state.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct DraftPreviewState {
    /// The shareable secret. Shown so the sharing panel can display the link
    /// again rather than re-minting one and breaking the reviewer's bookmark.
    pub token: String,

    /// When the link stops working.
    pub expires_at: DateTime<Utc>,

    /// When it was first shared. Unchanged by renewing.
    pub created_at: DateTime<Utc>,

    /// True once `expires_at` has passed. The link still exists and can be
    /// renewed; it just does not currently work.
    pub expired: bool,
}

/// Why a preview operation failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum DraftPreviewError {
    /// No post matched, or it belongs to another author.
    #[error("Blog post not found")]
    PostNotFound,

    /// The post has no preview link.
    #[error("This post is not shared")]
    NotShared,

    /// The store could not be reached.
    #[error("Repository error: {0}")]
    RepositoryError(String),
}

impl From<crate::blog::application::ports::outgoing::DraftPreviewStoreError> for DraftPreviewError {
    fn from(e: crate::blog::application::ports::outgoing::DraftPreviewStoreError) -> Self {
        use crate::blog::application::ports::outgoing::DraftPreviewStoreError as E;
        match e {
            E::PostNotFound => DraftPreviewError::PostNotFound,
            E::DatabaseError(m) => DraftPreviewError::RepositoryError(m),
        }
    }
}

/// Creates a post's preview link, or extends the one it has.
#[async_trait]
pub trait ShareDraftUseCase: Send + Sync {
    /// Mints or renews, and returns the resulting state.
    async fn execute(
        &self,
        owner: UserId,
        post_id: Uuid,
    ) -> Result<DraftPreviewState, DraftPreviewError>;
}

/// Reads a post's sharing state for its author.
#[async_trait]
pub trait GetDraftPreviewUseCase: Send + Sync {
    /// Returns the state, or `NotShared`.
    async fn execute(
        &self,
        owner: UserId,
        post_id: Uuid,
    ) -> Result<DraftPreviewState, DraftPreviewError>;
}

/// Withdraws a post's preview link.
#[async_trait]
pub trait RevokeDraftPreviewUseCase: Send + Sync {
    /// Revoking a post that is not shared succeeds.
    async fn execute(&self, owner: UserId, post_id: Uuid) -> Result<(), DraftPreviewError>;
}

/// What the holder of a preview token should be shown.
#[derive(Debug, Clone)]
pub enum PreviewResolution {
    /// The post is still a draft or scheduled. Serve it, with the banner.
    Draft(Box<BlogPostView>),

    /// The post has since been published. The public page is the better
    /// destination, so the reviewer's bookmark quietly upgrades to it rather
    /// than reporting an expired link for something anyone can now read.
    Published {
        /// Author's handle, for the public path.
        username: String,
        /// The post's slug.
        slug: String,
    },
}

/// Resolves a preview token for a reader with no account.
#[async_trait]
pub trait ReadDraftPreviewUseCase: Send + Sync {
    /// Resolves the token.
    ///
    /// An unknown, revoked or expired token is `PostNotFound` — the three must
    /// be indistinguishable, or the endpoint confirms which drafts exist.
    async fn execute(&self, token: &str) -> Result<PreviewResolution, DraftPreviewError>;
}

/// Resolves one image on a previewed draft.
///
/// Separate from [`ReadDraftPreviewUseCase`] because it answers a different
/// question — not "what is this post" but "may the holder of this token see
/// this one image, and where is it". Both are authorised by the same token.
#[async_trait]
pub trait ReadPreviewMediaUseCase: Send + Sync {
    /// A signed URL to redirect the reader to.
    ///
    /// Every failure — dead token, media that belongs to a different post,
    /// a variant that has not been generated yet — is `PostNotFound`. A
    /// reader holding a link should not be able to probe what exists.
    async fn execute(
        &self,
        token: &str,
        media_id: Uuid,
        size: &str,
    ) -> Result<String, DraftPreviewError>;
}
