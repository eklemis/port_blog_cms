//! Storage for the one preview link a draft may have.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// A live preview link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftPreview {
    /// The post it grants access to.
    pub post_id: Uuid,

    /// The shareable secret. Whoever holds it can read the draft, so treat it
    /// as a credential: never log it and never put it in an outbound link.
    pub token: String,

    /// When it stops working.
    pub expires_at: DateTime<Utc>,

    /// When it was first minted. Renewing does not change this, so the author
    /// can see how long a draft has been shared.
    pub created_at: DateTime<Utc>,
}

/// A live link, with the author whose post it opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LivePreview {
    /// The link itself.
    pub preview: DraftPreview,

    /// The post's author.
    pub owner_id: Uuid,
}

/// Why a preview write or read failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum DraftPreviewStoreError {
    /// No post matched the id, or it belongs to another user. The two are
    /// indistinguishable because the write is owner-scoped in SQL.
    #[error("Blog post not found")]
    PostNotFound,

    /// The store could not be reached.
    #[error("Database error: {0}")]
    DatabaseError(String),
}

/// Reads and writes preview links.
#[async_trait]
pub trait DraftPreviewStore: Send + Sync {
    /// Creates the post's link, or extends the one it has.
    ///
    /// Extending deliberately keeps the existing token: renewing is what an
    /// author does when a review is taking longer than expected, and minting a
    /// a fresh secret there would break the reviewer's bookmark at exactly the
    /// moment the author was trying to keep it alive.
    ///
    /// Owner-scoped: a post belonging to someone else is `PostNotFound`.
    async fn upsert(
        &self,
        owner: Uuid,
        post_id: Uuid,
        expires_at: DateTime<Utc>,
        new_token: &str,
    ) -> Result<DraftPreview, DraftPreviewStoreError>;

    /// The post's link, or `None` when it has never had one or it was revoked.
    ///
    /// An expired link is still returned — the author's sharing panel needs to
    /// say "expired" rather than "never shared", and renewing it is a different
    /// action from minting a first one.
    async fn find_for_post(
        &self,
        owner: Uuid,
        post_id: Uuid,
    ) -> Result<Option<DraftPreview>, DraftPreviewStoreError>;

    /// Removes the post's link. Revoking a post that has none succeeds.
    async fn revoke(&self, owner: Uuid, post_id: Uuid) -> Result<(), DraftPreviewStoreError>;

    /// Looks a link up by its token, for the public read.
    ///
    /// Not owner-scoped — the whole point is that the holder has no account.
    /// Expired links are **not** returned here: to a reader, expired and never
    /// existed must look the same.
    ///
    /// Returns the post's author alongside the link, because the reader has no
    /// identity of their own and the post has to be read as somebody. Joined
    /// here rather than fetched separately to keep this public path to one
    /// query.
    async fn find_live_by_token(
        &self,
        token: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<LivePreview>, DraftPreviewStoreError>;
}
