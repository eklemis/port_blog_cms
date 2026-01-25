use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;

// Unimplemented
#[derive(Debug, Clone)]
pub enum DeleteCVError {
    Unauthorized,
    CVNotFound,
    RepositoryError(String),
}

#[async_trait::async_trait]
pub trait HardDeleteCvUseCase: Send + Sync {
    async fn execute(&self, user_id: UserId, cv_id: Uuid) -> Result<(), DeleteCVError>;
}
