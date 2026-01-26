use crate::cv::domain::entities::CVInfo;
use async_trait::async_trait;
use uuid::Uuid;

#[derive(Debug, Clone, thiserror::Error)]
pub enum CVArchiverError {
    #[error("CV not found")]
    NotFound,

    #[error("CV already archived")]
    AlreadyArchived,

    #[error("CV not archived")]
    NotArchived,

    #[error("Database error: {0}")]
    DatabaseError(String),
}

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
