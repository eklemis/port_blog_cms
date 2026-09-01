//! Links a topic to a project. Idempotent — re-adding an existing link

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::modules::project::application::ports::outgoing::project_topic_repository::ProjectTopicRepositoryError;

//
// ──────────────────────────────────────────────────────────
// Errors
// ──────────────────────────────────────────────────────────
//

/// Why linking a topic to a project failed.
///
/// Distinguishes a missing project from a missing topic, so the handler can
/// say which id was wrong.
#[derive(Debug, Clone, thiserror::Error)]
pub enum AddProjectTopicError {
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

impl From<ProjectTopicRepositoryError> for AddProjectTopicError {
    fn from(err: ProjectTopicRepositoryError) -> Self {
        match err {
            ProjectTopicRepositoryError::ProjectNotFound => AddProjectTopicError::ProjectNotFound,
            ProjectTopicRepositoryError::TopicNotFound => AddProjectTopicError::TopicNotFound,
            ProjectTopicRepositoryError::DatabaseError(msg) => {
                AddProjectTopicError::RepositoryError(msg)
            }
        }
    }
}

//
// ──────────────────────────────────────────────────────────
// Incoming Port (Use Case)
// ──────────────────────────────────────────────────────────
//

/// Links a topic to a project. Idempotent — re-adding an existing link
/// succeeds.
#[async_trait]
pub trait AddProjectTopicUseCase: Send + Sync {
    /// Adds the link.
    async fn execute(
        &self,
        owner: UserId,
        project_id: Uuid,
        topic_id: Uuid,
    ) -> Result<(), AddProjectTopicError>;
}
