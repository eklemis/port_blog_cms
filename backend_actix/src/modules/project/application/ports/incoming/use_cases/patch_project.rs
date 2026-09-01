//! Applies a partial update.

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

/// Why patching a project failed.
///
/// No `Unauthorized` variant: the repository scopes on `owner`, so a project
/// belonging to someone else is reported as [`NotFound`](Self::NotFound)
/// rather than confirming it exists.
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

/// Applies a partial update.
///
/// Uses [`PatchField`](crate::project::application::ports::outgoing::project_repository::PatchField)
/// semantics: an omitted field is left alone, an explicit `null` clears it.
#[async_trait]
pub trait PatchProjectUseCase: Send + Sync {
    /// Applies the patch and returns the project as stored.
    async fn execute(
        &self,
        owner: UserId,
        project_id: Uuid,
        data: PatchProjectData,
    ) -> Result<ProjectResult, PatchProjectError>;
}
