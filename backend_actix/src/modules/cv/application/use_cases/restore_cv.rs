use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::cv::domain::entities::CVInfo;

/// Why restoring a CV failed.
#[derive(Debug, Clone)]
pub enum RestoreCVError {
    /// The CV exists but belongs to another user. Distinguishable here because
    /// the use case reads the CV before acting — `CVArchiver` does not scope on
    /// owner, unlike the blog and project archivers.
    Unauthorized,
    /// No CV matched the id.
    CVNotFound,
    /// The store could not be reached, or the write failed.
    RepositoryError(String),
}

/// Un-archives a soft-deleted CV.
#[async_trait::async_trait]
pub trait RestoreDeletedCvUseCase: Send + Sync {
    /// Restores a soft-deleted CV and returns it.
    ///
    /// Idempotent: restoring a CV that is not archived succeeds and returns the
    /// CV unchanged, so a repeated call is not an error.
    async fn execute(&self, user_id: UserId, cv_id: Uuid) -> Result<CVInfo, RestoreCVError>;
}
