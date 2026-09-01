//! Removes one topic link. Removing a link that is not there succeeds.

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::modules::project::application::ports::outgoing::project_topic_repository::ProjectTopicRepositoryError;

//
// ──────────────────────────────────────────────────────────
// Errors
// ──────────────────────────────────────────────────────────
//

/// Why unlinking a topic from a project failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum RemoveProjectTopicError {
    /// No project matched the id, or it belongs to another user.
    /// The repository scopes on owner in SQL, so the two are indistinguishable here.
    #[error("Project not found")]
    ProjectNotFound,

    /// No topic matched the id.
    #[error("Topic not found")]
    TopicNotFound,

    /// The store could not be reached, or failed for a reason this operation
    /// does not model. A 500 for the caller.
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

/// Removes one topic link. Removing a link that is not there succeeds.
#[async_trait]
pub trait RemoveProjectTopicUseCase: Send + Sync {
    /// Removes the link.
    async fn execute(
        &self,
        owner: UserId,
        project_id: Uuid,
        topic_id: Uuid,
    ) -> Result<(), RemoveProjectTopicError>;
}
