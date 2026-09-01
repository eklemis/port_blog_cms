//! Lifecycle port for projects: soft delete, restore, hard delete.

// src/modules/project/application/ports/outgoing/project_archiver.rs

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;

//
// ──────────────────────────────────────────────────────────
// Errors
// ──────────────────────────────────────────────────────────
//

/// Why a project lifecycle operation failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ProjectArchiverError {
    /// Project doesn't exist OR doesn't belong to owner.
    #[error("Project not found")]
    NotFound,

    /// The store could not be reached.
    #[error("Database error: {0}")]
    DatabaseError(String),
}

//
// ──────────────────────────────────────────────────────────
// Port (Command-side, lifecycle operations)
// ──────────────────────────────────────────────────────────
//

/// Archives, restores and permanently removes projects.
///
/// Every method scopes on `owner` in SQL, so a missing project and one owned
/// by somebody else are indistinguishable here — both are
/// [`NotFound`](ProjectArchiverError::NotFound). That is
/// deliberate: it avoids confirming that another user's project exists.
#[async_trait]
pub trait ProjectArchiver: Send + Sync {
    /// Flags the project as deleted, hiding it while keeping the row.
    async fn soft_delete(
        &self,
        owner: UserId,
        project_id: Uuid,
    ) -> Result<(), ProjectArchiverError>;

    /// Removes the row and its topic links permanently. Irreversible.
    async fn hard_delete(
        &self,
        owner: UserId,
        project_id: Uuid,
    ) -> Result<(), ProjectArchiverError>;

    /// Clears the deleted flag.
    async fn restore(&self, owner: UserId, project_id: Uuid) -> Result<(), ProjectArchiverError>;
}
