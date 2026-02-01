use async_trait::async_trait;

use crate::auth::application::domain::entities::UserId;
use crate::modules::project::application::ports::outgoing::project_query::{
    PageRequest, PageResult, ProjectCardView, ProjectListFilter, ProjectQueryError, ProjectSort,
};

//
// ──────────────────────────────────────────────────────────
// Errors
// ──────────────────────────────────────────────────────────
//

#[derive(Debug, Clone, thiserror::Error)]
pub enum GetProjectsError {
    #[error("Query failed: {0}")]
    QueryFailed(String),
}

impl From<ProjectQueryError> for GetProjectsError {
    fn from(err: ProjectQueryError) -> Self {
        match err {
            ProjectQueryError::DatabaseError(msg) => GetProjectsError::QueryFailed(msg),

            // For list(), NotFound is typically not used (empty list is valid),
            // but we still map defensively.
            ProjectQueryError::NotFound => GetProjectsError::QueryFailed("Not found".to_string()),

            ProjectQueryError::SerializationError(msg) => GetProjectsError::QueryFailed(msg),
        }
    }
}

//
// ──────────────────────────────────────────────────────────
// Incoming Port (Use Case)
// ──────────────────────────────────────────────────────────
//

#[async_trait]
pub trait GetProjectsUseCase: Send + Sync {
    async fn execute(
        &self,
        owner: UserId,
        filter: ProjectListFilter,
        sort: ProjectSort,
        page: PageRequest,
    ) -> Result<PageResult<ProjectCardView>, GetProjectsError>;
}
