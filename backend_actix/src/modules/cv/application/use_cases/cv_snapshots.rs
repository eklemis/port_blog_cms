//! Taking and reading immutable CV snapshots.

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::cv::application::ports::outgoing::{CvSnapshot, CvSnapshotStoreError};

/// Why a snapshot operation failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum CvSnapshotError {
    /// No CV matched, or it belongs to another user.
    #[error("CV not found")]
    CvNotFound,

    /// No snapshot matched, or it belongs to another user.
    #[error("Snapshot not found")]
    SnapshotNotFound,

    /// The store could not be reached, or gave back something unreadable.
    #[error("Repository error: {0}")]
    RepositoryError(String),
}

impl From<CvSnapshotStoreError> for CvSnapshotError {
    fn from(e: CvSnapshotStoreError) -> Self {
        match e {
            CvSnapshotStoreError::CvNotFound => CvSnapshotError::CvNotFound,
            CvSnapshotStoreError::SnapshotNotFound => CvSnapshotError::SnapshotNotFound,
            CvSnapshotStoreError::Corrupt(m) | CvSnapshotStoreError::DatabaseError(m) => {
                CvSnapshotError::RepositoryError(m)
            }
        }
    }
}

/// Freezes a CV as it stands.
#[async_trait]
pub trait CreateCvSnapshotUseCase: Send + Sync {
    /// Takes the snapshot and returns it.
    async fn execute(&self, owner: UserId, cv_id: Uuid) -> Result<CvSnapshot, CvSnapshotError>;
}

/// Reads a snapshot back, read-only and unchanged.
#[async_trait]
pub trait GetCvSnapshotUseCase: Send + Sync {
    /// Returns the frozen document.
    async fn execute(
        &self,
        owner: UserId,
        snapshot_id: Uuid,
    ) -> Result<CvSnapshot, CvSnapshotError>;
}
