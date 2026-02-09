use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::application::domain::entities::UserId,
    multimedia::application::domain::entities::{
        AttachmentTarget, MediaRole, MediaSize, MediaState, MediaStateInfo, MediaVariant,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentInfo {
    media_id: Uuid,
    attachment_target: AttachmentTarget,
    attachment_target_id: Uuid,
    role: MediaRole,
    // For ordered collections (galleries, screenshots)
    // 0-indexed, allows reordering
    position: i16,
    alt_text: String,
    caption: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueriedMediaInfo {
    owner: UserId,
    media_id: Uuid,
    attachment_target: AttachmentTarget,
    attachment_target_id: Uuid,
    variants: Vec<MediaVariant>,
    status: MediaState,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum MediaQueryError {
    #[error("Media not exist")]
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
    ) -> Result<Vec<AttachmentInfo>, MediaQueryError>;
    async fn get_attachment_info(&self, media_id: Uuid) -> Result<AttachmentInfo, MediaQueryError>;
}
