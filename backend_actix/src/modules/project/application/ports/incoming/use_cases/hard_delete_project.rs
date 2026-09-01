//! Removes a project and its topic links permanently. Irreversible.

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::modules::project::application::ports::outgoing::project_archiver::ProjectArchiverError;

//
// ──────────────────────────────────────────────────────────
// Errors
// ──────────────────────────────────────────────────────────
//

/// Why permanently deleting a project failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum HardDeleteProjectError {
    /// No project matched the id, or it belongs to another user.
    /// The repository scopes on owner in SQL, so the two are indistinguishable here.
    #[error("Project not found")]
    ProjectNotFound,

    /// The store could not be reached, or failed for a reason this operation
    /// does not model. A 500 for the caller.
    #[error("Repository error: {0}")]
    RepositoryError(String),
}

impl From<ProjectArchiverError> for HardDeleteProjectError {
    fn from(err: ProjectArchiverError) -> Self {
        match err {
            ProjectArchiverError::NotFound => HardDeleteProjectError::ProjectNotFound,
            ProjectArchiverError::DatabaseError(msg) => {
                HardDeleteProjectError::RepositoryError(msg)
            }
        }
    }
}

//
// ──────────────────────────────────────────────────────────
// Incoming Port (Use Case)
// ──────────────────────────────────────────────────────────
//

/// Removes a project and its topic links permanently. Irreversible.
#[async_trait]
pub trait HardDeleteProjectUseCase: Send + Sync {
    /// Deletes the project.
    async fn execute(&self, owner: UserId, project_id: Uuid) -> Result<(), HardDeleteProjectError>;
}
