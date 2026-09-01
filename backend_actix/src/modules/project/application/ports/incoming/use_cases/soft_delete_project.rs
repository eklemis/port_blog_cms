//! Hides a project without deleting it. Reversible.

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::modules::project::application::ports::outgoing::project_archiver::ProjectArchiverError;

//
// ──────────────────────────────────────────────────────────
// Errors
// ──────────────────────────────────────────────────────────
//

/// Why archiving a project failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum SoftDeleteProjectError {
    /// No project matched the id, or it belongs to another user.
    /// The repository scopes on owner in SQL, so the two are indistinguishable here.
    #[error("Project not found")]
    ProjectNotFound,

    /// The store could not be reached, or failed for a reason this operation
    /// does not model. A 500 for the caller.
    #[error("Repository error: {0}")]
    RepositoryError(String),
}

impl From<ProjectArchiverError> for SoftDeleteProjectError {
    fn from(err: ProjectArchiverError) -> Self {
        match err {
            ProjectArchiverError::NotFound => SoftDeleteProjectError::ProjectNotFound,
            ProjectArchiverError::DatabaseError(msg) => {
                SoftDeleteProjectError::RepositoryError(msg)
            }
        }
    }
}

//
// ──────────────────────────────────────────────────────────
// Incoming Port (Use Case)
// ──────────────────────────────────────────────────────────
//

/// Hides a project without deleting it. Reversible.
#[async_trait]
pub trait SoftDeleteProjectUseCase: Send + Sync {
    /// Archives the project.
    async fn execute(&self, owner: UserId, project_id: Uuid) -> Result<(), SoftDeleteProjectError>;
}
