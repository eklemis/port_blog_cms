//! Signing a short-lived read URL for a publicly visible media variant.
//!
//! The reader never sees a bucket URL. They fetch a stable path on this API,
//! which checks visibility, signs, and redirects. See
//! `docs/adr/0006-public-media-urls.md`.

use async_trait::async_trait;
use uuid::Uuid;

use crate::multimedia::application::domain::entities::MediaSize;

/// Why a public variant could not be served.
#[derive(Debug, Clone, thiserror::Error)]
pub enum GetPublicVariantUrlError {
    /// No such variant, or it is not attached to anything a reader can see.
    ///
    /// The two are deliberately one variant: distinguishing them would let a
    /// caller probe for media belonging to unpublished posts.
    #[error("Media not found")]
    NotFound,

    /// The object store could not be reached, or refused to sign.
    #[error("Storage error: {0}")]
    StorageError(String),

    /// The database could not be reached.
    #[error("Query error: {0}")]
    QueryError(String),
}

/// Produces a short-lived signed URL for a publicly visible variant.
#[async_trait]
pub trait GetPublicVariantUrlUseCase: Send + Sync {
    /// Returns a signed URL the caller should redirect to.
    ///
    /// The URL is short-lived on purpose: it exists only for the redirect hop,
    /// so it never needs to outlive a single fetch. Nothing caches it — the
    /// cached artefact is the stable API path that produced it.
    async fn execute(
        &self,
        media_id: Uuid,
        size: MediaSize,
    ) -> Result<String, GetPublicVariantUrlError>;
}
