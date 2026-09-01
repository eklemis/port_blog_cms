//! Lists the topics attached to a project.

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::modules::project::application::ports::outgoing::project_query::ProjectQueryError;
use crate::project::application::ports::outgoing::project_query::ProjectTopicItem;

//
// ──────────────────────────────────────────────────────────
// Output DTO
// ──────────────────────────────────────────────────────────
//

//
// ──────────────────────────────────────────────────────────
// Errors
// ──────────────────────────────────────────────────────────
//

/// Why listing a project's topics failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum GetProjectTopicsError {
    #[error("Project not found")]
    ProjectNotFound,

    #[error("Query failed: {0}")]
    QueryFailed(String),
}

impl From<ProjectQueryError> for GetProjectTopicsError {
    fn from(err: ProjectQueryError) -> Self {
        match err {
            ProjectQueryError::NotFound => GetProjectTopicsError::ProjectNotFound,
            ProjectQueryError::DatabaseError(msg) => GetProjectTopicsError::QueryFailed(msg),
            ProjectQueryError::SerializationError(msg) => GetProjectTopicsError::QueryFailed(msg),
        }
    }
}

//
// ──────────────────────────────────────────────────────────
// Incoming Port (Use Case)
// ──────────────────────────────────────────────────────────
//

/// Lists the topics attached to a project.
#[async_trait]
pub trait GetProjectTopicsUseCase: Send + Sync {
    /// Returns the attached topics.
    async fn execute(
        &self,
        owner: UserId,
        project_id: Uuid,
    ) -> Result<Vec<ProjectTopicItem>, GetProjectTopicsError>;
}
