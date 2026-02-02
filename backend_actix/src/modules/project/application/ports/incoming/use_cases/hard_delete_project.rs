use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::modules::project::application::ports::outgoing::project_archiver::ProjectArchiverError;

//
// ──────────────────────────────────────────────────────────
// Errors
// ──────────────────────────────────────────────────────────
//

#[derive(Debug, Clone, thiserror::Error)]
pub enum HardDeleteProjectError {
    #[error("Project not found")]
    ProjectNotFound,

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

#[async_trait]
pub trait HardDeleteProjectUseCase: Send + Sync {
    async fn execute(&self, owner: UserId, project_id: Uuid) -> Result<(), HardDeleteProjectError>;
}
