//! Freezing a CV, without `career` learning how CVs work.

use async_trait::async_trait;
use uuid::Uuid;

/// Why a snapshot could not be taken.
#[derive(Debug, Clone, thiserror::Error)]
pub enum CvSnapshotterError {
    /// No CV matched, or it belongs to another user.
    #[error("CV not found")]
    CvNotFound,

    /// The snapshot could not be written.
    #[error("Snapshot failed: {0}")]
    Failed(String),
}

/// Takes an immutable copy of a CV and reports its id.
///
/// A port rather than a direct call, so `career` depends on the idea of
/// freezing a CV rather than on the `cv` module — the same arrangement `auth`
/// uses for avatars and `blog` for preview images.
#[async_trait]
pub trait CvSnapshotter: Send + Sync {
    /// Freezes `cv_id` and returns the new snapshot's id.
    async fn snapshot(&self, owner: Uuid, cv_id: Uuid) -> Result<Uuid, CvSnapshotterError>;
}
