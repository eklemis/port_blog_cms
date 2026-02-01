use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::modules::project::application::ports::outgoing::project_topic_repository::ProjectTopicRepositoryError;

//
// ──────────────────────────────────────────────────────────
// Errors
// ──────────────────────────────────────────────────────────
//

#[derive(Debug, Clone, thiserror::Error)]
pub enum RemoveProjectTopicError {
    #[error("Project not found")]
    ProjectNotFound,

    #[error("Topic not found")]
    TopicNotFound,

    #[error("Repository error: {0}")]
    RepositoryError(String),
}

impl From<ProjectTopicRepositoryError> for RemoveProjectTopicError {
    fn from(err: ProjectTopicRepositoryError) -> Self {
        match err {
            ProjectTopicRepositoryError::ProjectNotFound => {
                RemoveProjectTopicError::ProjectNotFound
            }
            ProjectTopicRepositoryError::TopicNotFound => RemoveProjectTopicError::TopicNotFound,
            ProjectTopicRepositoryError::DatabaseError(msg) => {
                RemoveProjectTopicError::RepositoryError(msg)
            }
        }
    }
}

//
// ──────────────────────────────────────────────────────────
// Incoming Port (Use Case)
// ──────────────────────────────────────────────────────────
//

#[async_trait]
pub trait RemoveProjectTopicUseCase: Send + Sync {
    async fn execute(
        &self,
        owner: UserId,
        project_id: Uuid,
        topic_id: Uuid,
    ) -> Result<(), RemoveProjectTopicError>;
}
