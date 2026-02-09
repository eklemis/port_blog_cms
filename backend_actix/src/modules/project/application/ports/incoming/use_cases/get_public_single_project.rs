use async_trait::async_trait;

use crate::auth::application::domain::entities::UserId;
use crate::modules::project::application::ports::outgoing::project_query::ProjectView;

#[derive(Debug, Clone, thiserror::Error)]
pub enum GetPublicSingleProjectError {
    #[error("Project not found")]
    NotFound,

    #[error("Repository error: {0}")]
    RepositoryError(String),
}

#[async_trait]
pub trait GetPublicSingleProjectUseCase: Send + Sync {
    async fn execute(
        &self,
        owner: UserId,
        slug: &str,
    ) -> Result<ProjectView, GetPublicSingleProjectError>;
}
