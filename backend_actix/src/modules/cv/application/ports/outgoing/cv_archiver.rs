//! Lifecycle port for CVs: soft delete, restore, hard delete.

use crate::cv::domain::entities::CVInfo;
use async_trait::async_trait;
use uuid::Uuid;

/// Why a CV lifecycle operation failed.
///
/// Unlike the blog and project archivers, this port distinguishes wrong-state
/// transitions from a missing row, so callers can tell "already gone" from
/// "never existed".
#[derive(Debug, Clone, thiserror::Error)]
pub enum CVArchiverError {
    /// No CV matched the id.
    #[error("CV not found")]
    NotFound,

    /// The CV is already soft-deleted, so archiving would be a no-op.
    #[error("CV already archived")]
    AlreadyArchived,

    /// The CV is not archived, so there is nothing to restore.
    #[error("CV not archived")]
    NotArchived,

    /// The store could not be reached.
    #[error("Database error: {0}")]
    DatabaseError(String),
}

/// Archives, restores and permanently removes CVs.
///
/// These take no owner and do not scope on one, unlike the blog and project
/// archivers — **the calling service must check ownership first**.
#[async_trait]
pub trait CVArchiver: Send + Sync {
    /// Soft deletes a CV by marking it as archived.
    /// Returns `NotFound` if CV doesn't exist.
    /// Returns `AlreadyArchived` if CV is already soft deleted.
    async fn soft_delete(&self, cv_id: Uuid) -> Result<(), CVArchiverError>;

    /// Permanently deletes a CV from the database.
    /// Returns `NotFound` if CV doesn't exist.
    async fn hard_delete(&self, cv_id: Uuid) -> Result<(), CVArchiverError>;

    /// Restores a soft-deleted CV.
    /// Returns `NotFound` if CV doesn't exist.
    /// Returns `NotArchived` if CV is not soft deleted.
    async fn restore(&self, cv_id: Uuid) -> Result<CVInfo, CVArchiverError>;
}
