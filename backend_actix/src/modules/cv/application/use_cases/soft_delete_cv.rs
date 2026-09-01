use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;

/// Why archiving a CV failed.
#[derive(Debug, Clone)]
pub enum SoftDeleteCVError {
    /// The CV exists but belongs to another user. Distinguishable here because
    /// the use case reads the CV before acting — `CVArchiver` does not scope on
    /// owner, unlike the blog and project archivers.
    Unauthorized,
    /// No CV matched the id.
    CVNotFound,
    /// The store could not be reached, or the write failed.
    RepositoryError(String),
}

/// Archives a CV without deleting it. Reversible.
#[async_trait::async_trait]
pub trait SoftDeleteCvUseCase: Send + Sync {
    /// Archives a CV.
    ///
    /// Idempotent: archiving an already-archived CV succeeds, so a repeated
    /// DELETE is not an error.
    async fn execute(&self, user_id: UserId, cv_id: Uuid) -> Result<(), SoftDeleteCVError>;
}
