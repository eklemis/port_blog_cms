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
    pub owner: UserId,
    pub attachment_target: AttachmentTarget,
}

/// One media item as it appears in a listing.
///
/// Carries no `available_sizes`, unlike
/// [`MediaDetail`](super::get_media::MediaDetail): a listing would otherwise
/// need a variant query per row.
#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct MediaItem {
    pub media_id: Uuid,
    pub original_filename: String,
    pub status: MediaState,
    pub attachment_target: AttachmentTarget,
    pub attachment_target_id: Uuid,
    pub role: MediaRole,
    pub position: u8,
    pub alt_text: String,
    pub caption: String,
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
        }
    }
}

/// Lists every media item attached to one target.
#[async_trait]
pub trait ListMediaUseCase: Send + Sync {
    /// Returns the attached items, scoped to the command's owner.
    async fn execute(&self, command: ListMediaCommand) -> Result<Vec<MediaItem>, ListMediaError>;
}
