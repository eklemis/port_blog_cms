//! Editing, restoring and permanently removing media.
//!
//! Media was the one resource with no way back: `DELETE` already soft-deleted,
//! but nothing could undo it and nothing could correct an attachment's
//! metadata. Alt text set wrongly at upload was wrong forever.

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::multimedia::application::domain::entities::{AttachmentTarget, MediaState};
use crate::multimedia::application::ports::outgoing::db::{
    MediaRepositoryError, PatchAttachmentData,
};

/// Why a media lifecycle operation failed.
///
/// One enum for patch, restore and hard delete: they fail the same two ways,
/// and a caller that handles one handles all three.
#[derive(Debug, Clone, thiserror::Error)]
pub enum MediaLifecycleError {
    /// No such media, or it belongs to another user. The two are
    /// indistinguishable because every query is owner-scoped in SQL, which is
    /// what stops this confirming that someone else's media exists.
    #[error("Media not found")]
    NotFound,

    /// The store could not be reached.
    #[error("Repository error: {0}")]
    RepositoryError(String),
}

impl From<MediaRepositoryError> for MediaLifecycleError {
    fn from(e: MediaRepositoryError) -> Self {
        match e {
            MediaRepositoryError::NotFound => MediaLifecycleError::NotFound,
            MediaRepositoryError::DatabaseError(m) => MediaLifecycleError::RepositoryError(m),
        }
    }
}

/// Corrects an attachment's metadata.
#[async_trait]
pub trait PatchMediaUseCase: Send + Sync {
    /// Applies the patch. An empty patch succeeds without writing.
    async fn execute(
        &self,
        owner: UserId,
        media_id: Uuid,
        data: PatchAttachmentData,
    ) -> Result<(), MediaLifecycleError>;
}

/// Returns a soft-deleted item to service.
#[async_trait]
pub trait RestoreMediaUseCase: Send + Sync {
    /// Clears the deleted flag. Idempotent.
    async fn execute(&self, owner: UserId, media_id: Uuid) -> Result<(), MediaLifecycleError>;
}

/// Removes an item permanently.
#[async_trait]
pub trait HardDeleteMediaUseCase: Send + Sync {
    /// Deletes the rows. The stored objects are left to the bucket's lifecycle
    /// policy, so this is not a way to make bytes unreachable quickly.
    async fn execute(&self, owner: UserId, media_id: Uuid) -> Result<(), MediaLifecycleError>;
}

/// One place a media item is used.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct MediaUsage {
    /// What kind of thing it is attached to.
    pub target: AttachmentTarget,
    /// The id of that thing.
    pub target_id: Uuid,
    /// What the media is for on that target.
    pub role: String,
    /// Whether that target is visible to readers right now.
    ///
    /// This is the field that earns the endpoint. "Used on 3 posts" is mildly
    /// useful; "used on a post that is live right now" is what stops someone
    /// breaking their own published page.
    ///
    /// Projects have no draft state, so a non-deleted project is always `true`.
    pub is_published: bool,
}

/// Reports where a media item is used, before it is deleted.
#[async_trait]
pub trait GetMediaUsageUseCase: Send + Sync {
    /// Every attachment of this media, with the visibility of each target.
    ///
    /// An unused item is an empty list, not an error.
    async fn execute(
        &self,
        owner: UserId,
        media_id: Uuid,
    ) -> Result<Vec<MediaUsage>, MediaLifecycleError>;
}

/// One item's processing state, as returned by a batched poll.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct MediaStatus {
    /// Which item.
    pub media_id: Uuid,
    /// Where it is in processing.
    pub state: MediaState,
    /// When the state last changed.
    pub updated_at: String,
}

/// Reports the processing state of several items in one call.
#[async_trait]
pub trait GetMediaStatusesUseCase: Send + Sync {
    /// States for the requested ids.
    ///
    /// **Ids that do not resolve are absent from the result, not errors.** A
    /// client polling a set should not lose the whole batch because one item
    /// was deleted between polls, and it can treat an absent id as "gone".
    ///
    /// An empty request is an empty result without touching the database.
    async fn execute(
        &self,
        owner: UserId,
        media_ids: Vec<Uuid>,
    ) -> Result<Vec<MediaStatus>, MediaLifecycleError>;
}
