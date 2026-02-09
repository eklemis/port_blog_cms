use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::application::domain::entities::UserId,
    multimedia::application::domain::entities::{
        AttachmentTarget, MediaRole, MediaSize, MediaState, MediaStateInfo, MediaVariant,
    },
};

/// Represents a new media row to be recorded.
/// Refactor: use unsigned types for dimensions and sizes.
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

/// Represents a new attachment row to be recorded with a media.
/// Refactor: media_id is not needed here because it is created in the transaction.
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

#[derive(Debug, Clone, thiserror::Error)]
pub enum MediaRepositoryError {
    /// Media doesn't exist OR doesn't belong to owner.
    #[error("Media not found")]
    NotFound,

    #[error("Database error: {0}")]
    DatabaseError(String),
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaVariantRecord {
    pub owner: UserId,
    pub size: MediaSize,
    pub bucket_name: String,
    pub object_key: String,
    pub mime_type: String,
    pub file_size_bytes: u64,
    pub width_px: Option<u32>,
    pub height_px: Option<u32>,
}

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
}
