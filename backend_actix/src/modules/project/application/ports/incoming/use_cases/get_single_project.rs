//! Fetches one of the owner's projects by id, including unpublished ones.

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::modules::project::application::ports::outgoing::project_query::ProjectView;

/// Why fetching one project failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum GetSingleProjectError {
    #[error("Project not found")]
    NotFound,

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
