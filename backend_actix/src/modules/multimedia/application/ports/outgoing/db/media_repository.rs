//! Write-side port for media rows and their attachments.
//!
//! An upload is recorded before the bytes arrive: the row is created in a
//! pending state, the client is handed a signed URL, and an out-of-band
//! function flips the state once variants exist. So a media row existing does
//! not mean the file does — see `MediaState`.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::application::domain::entities::UserId,
    multimedia::application::domain::entities::{
        AttachmentTarget, MediaRole, MediaSize, MediaState, MediaStateInfo, MediaVariant,
    },
};

/// A media row to be inserted.
///
/// Dimensions and durations are optional because they are only known for the
/// media types that have them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewMedia {
    pub owner: UserId,
    pub state: MediaState,
    pub bucket_name: String,
    pub original_name: String,
    pub mime_type: String,
    pub file_size_bytes: u64,
    pub width_px: Option<u32>,
    pub height_px: Option<u32>,
    pub duration_seconds: Option<u64>,
}

/// The attachment row recorded alongside a media row.
///
/// Carries no `media_id`: both rows are written in one transaction, so the id
/// is only known inside it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewMediaAttachment {
    pub owner: UserId,
    pub attachment_target: AttachmentTarget,
    pub attachment_target_id: Uuid,
    pub role: MediaRole,
    /// start from 0
    pub position: u8,
    pub alt_text: Option<String>,
    pub caption: Option<String>,
}

/// Transaction payload: record media + its attachment atomically.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordMediaTx {
    pub media: NewMedia,
    pub attachment: NewMediaAttachment,
}

/// Minimal info the use case needs after recording.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedMedia {
    pub owner: UserId,
    pub media_id: Uuid,
    pub bucket_name: String,
    pub original_name: String,
    pub attachment_target: AttachmentTarget,
    pub state: MediaState,
}

/// Why a media write failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum MediaRepositoryError {
    /// Media doesn't exist OR doesn't belong to owner.
    #[error("Media not found")]
    NotFound,

    #[error("Database error: {0}")]
    DatabaseError(String),
}

/// Why recording an upload failed.
///
/// Separate from [`MediaRepositoryError`] because recording writes two rows in
/// one transaction and can fail in ways a plain read or delete cannot.
#[derive(Debug, Clone, thiserror::Error)]
pub enum RecordMediaError {
    #[error("Database error: {0}")]
    DatabaseError(String),
}

/// Make fields public so other modules can construct this cleanly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMediaStateData {
    pub owner: UserId,
    pub media_id: Uuid,
    pub status: MediaState,
}

/// One generated size of a media item, written once processing finishes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaVariantRecord {
    pub owner: UserId,

    /// The media this variant belongs to. `media_variants.media_id` is NOT
    /// NULL, so without it a variant cannot be attached to anything.
    pub media_id: Uuid,

    pub size: MediaSize,
    pub bucket_name: String,
    pub object_key: String,
    pub mime_type: String,
    pub file_size_bytes: u64,
    pub width_px: Option<u32>,
    pub height_px: Option<u32>,
}

/// Writes media rows, their attachments and their variants.
///
/// Reads belong to [`MediaQuery`](super::media_query::MediaQuery).
#[async_trait]
pub trait MediaRepository: Send + Sync {
    /// Store a row into media and media attachment with transaction
    async fn record_media_tx(&self, tx: RecordMediaTx) -> Result<RecordedMedia, RecordMediaError>;

    async fn set_media_state(
        &self,
        data: UpdateMediaStateData,
    ) -> Result<MediaStateInfo, MediaRepositoryError>;

    async fn record_single_variant(
        &self,
        data: MediaVariantRecord,
    ) -> Result<MediaVariant, MediaRepositoryError>;

    async fn record_variants(
        &self,
        data: Vec<MediaVariantRecord>,
    ) -> Result<Vec<MediaVariant>, MediaRepositoryError>;

    /// Marks a media row deleted by stamping `deleted_at`.
    ///
    /// Storage objects are left in place: the upload bucket is reaped by a GCS
    /// lifecycle rule, and every read path already filters `deleted_at IS NULL`,
    /// so the media disappears from listings and signed-URL requests at once.
    ///
    /// Scoped by owner, so another user's media reports `NotFound` rather than
    /// revealing that it exists.
    async fn soft_delete(&self, owner: UserId, media_id: Uuid) -> Result<(), MediaRepositoryError>;
}
