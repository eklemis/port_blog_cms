//! Handing a reader a signed URL for one size of a media item.
use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    auth::application::domain::entities::UserId,
    multimedia::application::domain::entities::MediaSize,
};

/// Why a read URL could not be produced.
///
/// The three processing-state variants are distinct on purpose: a media row is
/// created before the file arrives, so "not ready" is a normal, temporary
/// answer and not the same as "missing". Callers surface
/// [`MediaProcessing`](Self::MediaProcessing) and [`MediaPending`](Self::MediaPending)
/// as retryable, and [`MediaFailed`](Self::MediaFailed) as terminal.
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

/// Which variant of which media item is wanted.
pub struct GetUrlCommand {
    pub owner: UserId,
    pub media_id: Uuid,
    pub size: MediaSize,
}

/// A signed URL and the moment it stops working.
///
/// `expires_at` is short by design — the URL is a bearer credential for the
/// object, so clients should fetch promptly rather than cache it.
#[derive(Clone)]
pub struct GetUrlResult {
    pub media_id: Uuid,
    pub size: MediaSize,
    pub url: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

/// Produces a time-limited read URL for one variant.
#[async_trait]
pub trait GetVariantReadUrlUseCase: Send + Sync {
    /// Returns a signed URL, or says why the variant is not available yet.
    async fn execute(&self, command: GetUrlCommand) -> Result<GetUrlResult, GetReadUrlError>;
}
