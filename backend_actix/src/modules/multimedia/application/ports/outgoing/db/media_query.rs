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
    pub size: MediaSize,
    pub bucket_name: String,
    pub object_name: String,
    pub width: u32,
    pub height: u32,
    pub file_size_bytes: u64,
    pub mime_type: String,
}

/// Complete media attachment information from database
#[derive(Debug, Clone)]
pub struct MediaAttachment {
    pub media_id: Uuid,
    pub owner: UserId,
    pub attachment_target: AttachmentTarget,
    pub attachment_target_id: Uuid,
    pub status: MediaState,
    pub role: MediaRole,
    pub position: i16,
    pub alt_text: String,
    pub caption: String,
    pub original_filename: String,
    pub variants: Vec<StoredVariant>,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum MediaQueryError {
    #[error("Media not found")]
    MediaNotFound,

    #[error("Database error: {0}")]
    DatabaseError(String),
}

#[async_trait]
pub trait MediaQuery: Send + Sync {
    async fn get_state(&self, media_id: Uuid) -> Result<MediaStateInfo, MediaQueryError>;

    async fn list_by_target(
        &self,
        owner: UserId,
        target: AttachmentTarget,
    ) -> Result<Vec<MediaAttachment>, MediaQueryError>;

    async fn get_attachment_info(&self, media_id: Uuid)
        -> Result<MediaAttachment, MediaQueryError>;
}
