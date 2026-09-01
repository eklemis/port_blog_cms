//! Removes every topic link from a project.

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::modules::project::application::ports::outgoing::project_topic_repository::ProjectTopicRepositoryError;

//
// ──────────────────────────────────────────────────────────
// Errors
// ──────────────────────────────────────────────────────────
//

/// Why clearing a project's topics failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ClearProjectTopicsError {
    /// No project matched the id, or it belongs to another user.
    /// The repository scopes on owner in SQL, so the two are indistinguishable here.
    #[error("Project not found")]
    ProjectNotFound,

    /// The store could not be reached, or failed for a reason this operation
    /// does not model. A 500 for the caller.
    #[error("Repository error: {0}")]
    RepositoryError(String),
}

impl From<ProjectTopicRepositoryError> for ClearProjectTopicsError {
    fn from(err: ProjectTopicRepositoryError) -> Self {
        match err {
            ProjectTopicRepositoryError::ProjectNotFound => {
                ClearProjectTopicsError::ProjectNotFound
            }
            ProjectTopicRepositoryError::TopicNotFound => {
                // Not part of this use case contract; treat as repo-level failure.
                ClearProjectTopicsError::RepositoryError("Topic not found".to_string())
            }
            ProjectTopicRepositoryError::DatabaseError(msg) => {
                ClearProjectTopicsError::RepositoryError(msg)
            }
        }
    }
}

//
// ──────────────────────────────────────────────────────────
// Incoming Port (Use Case)
// ──────────────────────────────────────────────────────────
//

/// Removes every topic link from a project.
#[async_trait]
pub trait ClearProjectTopicsUseCase: Send + Sync {
    /// Clears the links.
    async fn execute(&self, owner: UserId, project_id: Uuid)
        -> Result<(), ClearProjectTopicsError>;
}
