use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::modules::project::application::ports::outgoing::project_repository::{
    PatchProjectData, ProjectResult,
};

//
// ──────────────────────────────────────────────────────────
// Errors
// ──────────────────────────────────────────────────────────
//

#[derive(Debug, Clone, thiserror::Error)]
pub enum PatchProjectError {
    #[error("Project not found")]
    NotFound,

    #[error("Repository error: {0}")]
    RepositoryError(String),
}

//
// ──────────────────────────────────────────────────────────
// Use case trait
// ──────────────────────────────────────────────────────────
//

#[async_trait]
pub trait PatchProjectUseCase: Send + Sync {
    async fn execute(
        &self,
        owner: UserId,
        project_id: Uuid,
        data: PatchProjectData,
    ) -> Result<ProjectResult, PatchProjectError>;
}
