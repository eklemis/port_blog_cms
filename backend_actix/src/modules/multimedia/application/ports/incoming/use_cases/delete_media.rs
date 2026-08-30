use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::multimedia::application::ports::outgoing::db::MediaRepositoryError;

#[derive(Debug, Clone, thiserror::Error)]
pub enum DeleteMediaError {
    /// Media does not exist, or belongs to another user. The two are reported
    /// identically so the endpoint cannot be used to probe for media ids.
    #[error("Media not found")]
    MediaNotFound,

    #[error("Repository error: {0}")]
    RepositoryError(String),
}

impl From<MediaRepositoryError> for DeleteMediaError {
    fn from(e: MediaRepositoryError) -> Self {
        match e {
            MediaRepositoryError::NotFound => DeleteMediaError::MediaNotFound,
            MediaRepositoryError::DatabaseError(msg) => DeleteMediaError::RepositoryError(msg),
        }
    }
}

#[async_trait]
pub trait DeleteMediaUseCase: Send + Sync {
    /// Soft-deletes a media item.
    ///
    /// Idempotent: deleting an already-deleted item succeeds.
    async fn execute(&self, owner: UserId, media_id: Uuid) -> Result<(), DeleteMediaError>;
}
