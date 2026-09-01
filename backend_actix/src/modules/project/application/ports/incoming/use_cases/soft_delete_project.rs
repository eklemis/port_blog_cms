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
    #[error("Project not found")]
    ProjectNotFound,

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
