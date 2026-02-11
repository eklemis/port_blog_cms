use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    auth::application::domain::entities::UserId,
    multimedia::application::domain::entities::MediaSize,
};

#[derive(Debug, Clone, thiserror::Error)]
pub enum GetReadUrlError {
    #[error("Media not found")]
    MediaNotFound,

    #[error("Media is still being processed")]
    MediaProcessing,

    #[error("Media is pending upload")]
    MediaPending,

    #[error("Media processing failed")]
    MediaFailed,

    #[error("Variant '{0}' not found for this media")]
    VariantNotFound(MediaSize),

    #[error("Storage error: {0}")]
    StorageError(String),
    #[error("Query error: {0}")]
    QueryError(String),
}

pub struct GetUrlCommand {
    pub owner: UserId,
    pub media_id: Uuid,
    pub size: MediaSize,
}

#[derive(Clone)]
pub struct GetUrlResult {
    pub media_id: Uuid,
    pub size: MediaSize,
    pub url: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[async_trait]
pub trait GetVariantReadUrlUseCase: Send + Sync {
    async fn execute(&self, command: GetUrlCommand) -> Result<GetUrlResult, GetReadUrlError>;
}
