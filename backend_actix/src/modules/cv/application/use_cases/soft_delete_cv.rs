use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;

#[derive(Debug, Clone)]
pub enum SoftDeleteCVError {
    Unauthorized,
    CVNotFound,
    RepositoryError(String),
}

#[async_trait::async_trait]
pub trait SoftDeleteCvUseCase: Send + Sync {
    /// Archives a CV.
    ///
    /// Idempotent: archiving an already-archived CV succeeds, so a repeated
    /// DELETE is not an error.
    async fn execute(&self, user_id: UserId, cv_id: Uuid) -> Result<(), SoftDeleteCVError>;
}
