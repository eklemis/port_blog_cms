use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;

// Unimplemented
/// Why permanent deletion failed.
#[derive(Debug, Clone)]
pub enum HardDeleteCVError {
    /// The CV exists but belongs to another user. Distinguishable here because
    /// the use case reads the CV before acting — `CVArchiver` does not scope on
    /// owner, unlike the blog and project archivers.
    Unauthorized,
    /// No CV matched the id.
    CVNotFound,
    /// The store could not be reached, or the write failed.
    RepositoryError(String),
}

/// Removes a CV permanently. Irreversible.
#[async_trait::async_trait]
pub trait HardDeleteCvUseCase: Send + Sync {
    /// Deletes the CV after checking ownership.
    async fn execute(&self, user_id: UserId, cv_id: Uuid) -> Result<(), HardDeleteCVError>;
}
