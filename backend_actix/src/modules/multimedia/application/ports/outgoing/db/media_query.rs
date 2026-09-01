//! Read-side port for media rows, their variants and their attachments.

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    auth::application::domain::entities::UserId,
    multimedia::application::domain::entities::{
        AttachmentTarget, MediaRole, MediaSize, MediaState, MediaStateInfo,
    },
};

/// Information about a media variant from storage
#[derive(Debug, Clone)]
pub struct StoredVariant {
    /// Which generated size this row is.
    pub size: MediaSize,
    /// Bucket the variant was written to.
    pub bucket_name: String,
    /// Object key of the variant.
    pub object_name: String,
    /// Width in pixels, as generated.
    pub width: u32,
    /// Height in pixels, as generated.
    pub height: u32,
    /// Size of the generated file.
    pub file_size_bytes: u64,
    /// MIME type of the generated file, which may differ from the original's.
    pub mime_type: String,
}

/// Complete media attachment information from database
#[derive(Debug, Clone)]
pub struct MediaAttachment {
    /// The media item.
    pub media_id: Uuid,
    /// The user who uploaded it.
    pub owner: UserId,
    /// What kind of thing it is attached to — a CV, a project, a post.
    pub attachment_target: AttachmentTarget,
    /// The id of that thing.
    pub attachment_target_id: Uuid,
    /// Where the item is in processing. Rows exist before their bytes do, so
    /// this is what says whether the file is usable.
    pub status: MediaState,
    /// What the media is for on its target — a cover image, a gallery entry.
    pub role: MediaRole,
    /// Display order within its role, starting at 0.
    pub position: u8,
    /// Alternative text. Empty rather than absent when unset.
    pub alt_text: String,
    /// Caption. Empty rather than absent when unset.
    pub caption: String,
    /// The name the file was uploaded under.
    pub original_filename: String,
    /// Generated sizes. Empty while the item is still processing.
    pub variants: Vec<StoredVariant>,
}

/// Why a media read failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum MediaQueryError {
    /// No media matched the id, or it belongs to another user.
    #[error("Media not found")]
    MediaNotFound,

    /// The store could not be reached.
    #[error("Database error: {0}")]
    DatabaseError(String),
}

/// Reads media rows.
#[async_trait]
pub trait MediaQuery: Send + Sync {
    /// The processing state of one upload.
    ///
    /// Callers poll this to learn whether variants exist yet: a row is created
    /// before the bytes arrive, so existence does not imply availability.
    async fn get_state(&self, media_id: Uuid) -> Result<MediaStateInfo, MediaQueryError>;

    /// Every media item attached to one target — a CV, a project, a post.
    async fn list_by_target(
        &self,
        owner: UserId,
        target: AttachmentTarget,
    ) -> Result<Vec<MediaAttachment>, MediaQueryError>;

    /// What a media item is attached to, and in what role.
    async fn get_attachment_info(&self, media_id: Uuid)
        -> Result<MediaAttachment, MediaQueryError>;
}
