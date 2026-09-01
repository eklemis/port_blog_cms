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
    /// No media matched the id, or it belongs to another user.
    #[error("Media not found")]
    MediaNotFound,

    /// The file arrived and variants are being generated. Retryable — ask again
    /// shortly.
    #[error("Media is still being processed")]
    MediaProcessing,

    /// The row exists but the file has not been uploaded yet. Retryable.
    #[error("Media is pending upload")]
    MediaPending,

    /// Variant generation failed. Terminal: retrying will not help.
    #[error("Media processing failed")]
    MediaFailed,

    /// The item is ready but not in the size asked for. The payload names the
    /// size.
    #[error("Variant '{0}' not found for this media")]
    VariantNotFound(MediaSize),

    /// The object store could not be reached or refused to sign.
    #[error("Storage error: {0}")]
    StorageError(String),
    /// The database could not be reached.
    #[error("Query error: {0}")]
    QueryError(String),
}

/// Which variant of which media item is wanted.
pub struct GetUrlCommand {
    /// The user asking. Reads are scoped to their own media.
    pub owner: UserId,
    /// Which media item.
    pub media_id: Uuid,
    /// Which generated size.
    pub size: MediaSize,
}

/// A signed URL and the moment it stops working.
///
/// `expires_at` is short by design — the URL is a bearer credential for the
/// object, so clients should fetch promptly rather than cache it.
#[derive(Clone)]
pub struct GetUrlResult {
    /// Which media item.
    pub media_id: Uuid,
    /// Which generated size.
    pub size: MediaSize,
    /// The signed URL. Treat it as a bearer credential for the object.
    pub url: String,
    /// When the URL stops working. Short by design — fetch promptly rather than
    /// caching it.
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

/// Produces a time-limited read URL for one variant.
#[async_trait]
pub trait GetVariantReadUrlUseCase: Send + Sync {
    /// Returns a signed URL, or says why the variant is not available yet.
    async fn execute(&self, command: GetUrlCommand) -> Result<GetUrlResult, GetReadUrlError>;
}
