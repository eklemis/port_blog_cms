//! Un-archiving a soft-deleted project.
//!
//! Posts and CVs have had restore since they gained soft delete; projects had
//! the archiver method but no way to reach it, so the console had to present
//! project deletion as permanent — contradicting the archive pattern used
//! everywhere else.

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::modules::project::application::ports::outgoing::project_archiver::ProjectArchiverError;

/// Why restoring a project failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum RestoreProjectError {
    /// No project matched the id, or it belongs to another user. The archiver
    /// scopes on owner in SQL, so the two are indistinguishable here — which
    /// is what stops this confirming that someone else's project exists.
    #[error("Project not found")]
    ProjectNotFound,

    /// The store could not be reached.
    #[error("Repository error: {0}")]
    RepositoryError(String),
}

impl From<ProjectArchiverError> for RestoreProjectError {
    fn from(err: ProjectArchiverError) -> Self {
        match err {
            ProjectArchiverError::NotFound => RestoreProjectError::ProjectNotFound,
            ProjectArchiverError::DatabaseError(msg) => RestoreProjectError::RepositoryError(msg),
        }
    }
}

/// Returns an archived project to service.
#[async_trait]
pub trait RestoreProjectUseCase: Send + Sync {
    /// Clears the deleted flag.
    ///
    /// The project keeps its slug and id, so any link to it resolves again.
    /// Restoring a project that was never archived succeeds — the archiver
    /// treats it as idempotent.
    async fn execute(&self, owner: UserId, project_id: Uuid) -> Result<(), RestoreProjectError>;
}
