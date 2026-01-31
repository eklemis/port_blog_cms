// src/modules/project/application/ports/outgoing/project_archiver.rs

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;

//
// ──────────────────────────────────────────────────────────
// Errors
// ──────────────────────────────────────────────────────────
//

#[derive(Debug, Clone, thiserror::Error)]
pub enum ProjectArchiverError {
    /// Project doesn't exist OR doesn't belong to owner.
    #[error("Project not found")]
    NotFound,

    #[error("Database error: {0}")]
    DatabaseError(String),
}

//
// ──────────────────────────────────────────────────────────
// Port (Command-side, lifecycle operations)
// ──────────────────────────────────────────────────────────
//

#[async_trait]
pub trait ProjectArchiver: Send + Sync {
    async fn soft_delete(
        &self,
        owner: UserId,
        project_id: Uuid,
    ) -> Result<(), ProjectArchiverError>;

    async fn hard_delete(
        &self,
        owner: UserId,
        project_id: Uuid,
    ) -> Result<(), ProjectArchiverError>;

    async fn restore(&self, owner: UserId, project_id: Uuid) -> Result<(), ProjectArchiverError>;
}
