//! Lists an owner's projects, filtered, sorted and paginated.

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

/// Why a project listing failed.
///
/// A listing that matches nothing is an empty page, not an error.
#[derive(Debug, Clone, thiserror::Error)]
pub enum GetProjectsError {
    /// The read could not be executed. A 500 for the caller.
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

/// Lists an owner's projects, filtered, sorted and paginated.
#[async_trait]
pub trait GetProjectsUseCase: Send + Sync {
    /// Returns one page of results, with the total for the whole filter.
    async fn execute(
        &self,
        owner: UserId,
        filter: ProjectListFilter,
        sort: ProjectSort,
        page: PageRequest,
    ) -> Result<PageResult<ProjectCardView>, GetProjectsError>;
}
