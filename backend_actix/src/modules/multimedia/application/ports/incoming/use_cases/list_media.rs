//! Listing the media attached to one target.
use async_trait::async_trait;
use serde::Serialize;
use uuid::Uuid;

use crate::{
    auth::application::domain::entities::UserId,
    multimedia::application::{
        domain::entities::{AttachmentTarget, MediaRole, MediaState},
        ports::outgoing::db::{MediaAttachment, MediaQueryError},
    },
};

/// Why listing media failed.
///
/// A target with no media is an empty `Vec`, not an error.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ListMediaError {
    /// The store could not be reached.
    #[error("Repository error: {0}")]
    RepositoryError(String),
}
impl From<MediaQueryError> for ListMediaError {
    fn from(err: MediaQueryError) -> Self {
        Self::RepositoryError(err.to_string())
    }
}

/// Which target's media to list.
pub struct ListMediaCommand {
    /// The user asking. Listings are scoped to their own media.
    pub owner: UserId,
    /// What kind of thing it is attached to.
    pub attachment_target: AttachmentTarget,
    /// Which one of them, when the caller wants a single thing's media.
    ///
    /// `None` lists the target kind across everything the caller owns, which is
    /// what this endpoint did unconditionally before.
    pub target_id: Option<Uuid>,
    /// Which role to keep — `cover`, `screenshot`, `avatar`, `inline`.
    ///
    /// `None` keeps every role.
    pub role: Option<String>,
    /// Whether archived items are included alongside live ones.
    ///
    /// Archived media can already be restored and purged, but until this there
    /// was no way to *find* an archived item: the listing excluded it and the
    /// row carried no sign it existed. So a Restore button had nothing to act
    /// on.
    ///
    /// This includes them rather than showing them alone — the screen that
    /// wants them draws one grid with archived tiles marked, and `deleted_at`
    /// on the row is what marks them.
    pub include_deleted: bool,
}

/// One media item as it appears in a listing.
///
/// Carries no `available_sizes`, unlike
/// [`MediaDetail`](super::get_media::MediaDetail): a listing would otherwise
/// need a variant query per row.
#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct MediaItem {
    /// The media item.
    pub media_id: Uuid,
    /// The name the file was uploaded under.
    pub original_filename: String,
    /// Where the item is in processing. A row exists before its bytes do, so
    /// this is what says whether the file is usable.
    pub status: MediaState,
    /// What kind of thing it is attached to.
    pub attachment_target: AttachmentTarget,
    /// The id of that thing.
    pub attachment_target_id: Uuid,
    /// What the media is for on its target.
    pub role: MediaRole,
    /// Display order within the role, starting at 0.
    pub position: u8,
    /// Alternative text. Empty rather than absent when unset.
    pub alt_text: String,
    /// Caption. Empty rather than absent when unset.
    pub caption: String,

    /// When this item was archived, or `null` while it is live.
    ///
    /// Only present at all when the listing was asked for archived items; a
    /// live listing returns `null` on every row. It is what lets one grid draw
    /// live and archived tiles differently, and what a Restore control acts on.
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
}
impl MediaItem {
    /// Projects a full attachment row down to the listing shape.
    pub fn from_media_attachment(media: MediaAttachment) -> Self {
        Self {
            media_id: media.media_id,
            original_filename: media.original_filename,
            status: media.status,
            attachment_target: media.attachment_target,
            attachment_target_id: media.attachment_target_id,
            role: media.role,
            position: media.position,
            alt_text: media.alt_text,
            caption: media.caption,
            deleted_at: media.deleted_at,
        }
    }
}

/// Lists every media item attached to one target.
#[async_trait]
pub trait ListMediaUseCase: Send + Sync {
    /// Returns the attached items, scoped to the command's owner.
    async fn execute(&self, command: ListMediaCommand) -> Result<Vec<MediaItem>, ListMediaError>;
}
