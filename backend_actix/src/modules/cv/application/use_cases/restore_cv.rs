use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::cv::domain::entities::CVInfo;

#[derive(Debug, Clone)]
pub enum RestoreCVError {
    Unauthorized,
    CVNotFound,
    RepositoryError(String),
}

#[async_trait::async_trait]
pub trait RestoreDeletedCvUseCase: Send + Sync {
    /// Restores a soft-deleted CV and returns it.
    ///
    /// Idempotent: restoring a CV that is not archived succeeds and returns the
    /// CV unchanged, so a repeated call is not an error.
    async fn execute(&self, user_id: UserId, cv_id: Uuid) -> Result<CVInfo, RestoreCVError>;
}
