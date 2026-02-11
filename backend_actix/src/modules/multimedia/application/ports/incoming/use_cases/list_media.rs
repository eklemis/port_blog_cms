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

pub struct ListMediaCommand {
    pub owner: UserId,
    pub attachment_target: AttachmentTarget,
}

#[derive(Clone, Debug, Serialize)]
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

#[async_trait]
pub trait ListMediaUseCase: Send + Sync {
    async fn execute(&self, command: ListMediaCommand) -> Result<Vec<MediaItem>, ListMediaError>;
}
