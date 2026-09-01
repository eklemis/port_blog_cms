//! Fetches one project by slug for a public reader.

use async_trait::async_trait;

use crate::auth::application::domain::entities::UserId;
use crate::modules::project::application::ports::outgoing::project_query::ProjectView;

/// Why fetching a project for a public reader failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum GetPublicSingleProjectError {
    #[error("Project not found")]
    NotFound,

    #[error("Repository error: {0}")]
    RepositoryError(String),
}

/// Fetches one project by slug for a public reader.
///
/// Addressed by slug rather than id, because that is what appears in a public
/// URL. A project not visible publicly is reported as
/// [`NotFound`](GetPublicSingleProjectError::NotFound) rather than forbidden,
/// so its slug cannot be probed for.
#[async_trait]
pub trait GetPublicSingleProjectUseCase: Send + Sync {
    /// Returns the project in full.
    async fn execute(
        &self,
        owner: UserId,
        slug: &str,
    ) -> Result<ProjectView, GetPublicSingleProjectError>;
}
