//! Fetches one of the owner's projects by id, including unpublished ones.

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::modules::project::application::ports::outgoing::project_query::ProjectView;

/// Why fetching one project failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum GetSingleProjectError {
    /// No project matched the id, or it belongs to another user.
    /// The repository scopes on owner in SQL, so the two are indistinguishable here.
    #[error("Project not found")]
    NotFound,

    /// The store could not be reached, or failed for a reason this operation
    /// does not model. A 500 for the caller.
    #[error("Repository error: {0}")]
    RepositoryError(String),
}

/// Fetches one of the owner's projects by id, including unpublished ones.
#[async_trait]
pub trait GetSingleProjectUseCase: Send + Sync {
    /// Returns the project in full.
    async fn execute(
        &self,
        owner: UserId,
        project_id: Uuid,
    ) -> Result<ProjectView, GetSingleProjectError>;
}
