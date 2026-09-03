//! Re-exports the module's use-case contracts.

mod add_project_topic;
mod clear_project_topics;
mod create_project;
mod get_project_topics;
mod get_projects;
mod get_public_single_project;
mod get_single_project;
mod hard_delete_project;
mod patch_project;
mod remove_project_topic;
mod soft_delete_project;

pub use add_project_topic::{AddProjectTopicError, AddProjectTopicUseCase};
pub use clear_project_topics::{ClearProjectTopicsError, ClearProjectTopicsUseCase};
pub use create_project::{CreateProjectError, CreateProjectUseCase};
pub use get_project_topics::{GetProjectTopicsError, GetProjectTopicsUseCase};
pub use get_projects::{GetProjectsError, GetProjectsUseCase};
pub use get_public_single_project::{GetPublicSingleProjectError, GetPublicSingleProjectUseCase};
pub use get_single_project::{GetSingleProjectError, GetSingleProjectUseCase};
pub use hard_delete_project::{HardDeleteProjectError, HardDeleteProjectUseCase};
pub use patch_project::{PatchProjectError, PatchProjectUseCase};
pub use remove_project_topic::{RemoveProjectTopicError, RemoveProjectTopicUseCase};
pub use soft_delete_project::{SoftDeleteProjectError, SoftDeleteProjectUseCase};
mod slug_available;
pub use slug_available::ProjectSlugAvailableUseCase;
mod restore_project;
pub use restore_project::{RestoreProjectError, RestoreProjectUseCase};

/// One operation applied across many projects.
///
/// Tagged so that "attach, but no topic given" cannot be expressed — the
/// request fails to deserialise rather than failing halfway through a batch.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, utoipa::ToSchema)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum ProjectBulkOp {
    /// Hide each project. Reversible.
    Archive,
    /// Un-hide each project.
    Restore,
    /// Remove each project permanently.
    HardDelete,
    /// Link one topic to each project.
    AttachTopic {
        /// The topic to link.
        topic_id: uuid::Uuid,
    },
    /// Remove one topic's link from each project.
    DetachTopic {
        /// The topic to unlink.
        topic_id: uuid::Uuid,
    },
}

/// Applies one operation to many projects, reporting per item.
///
/// Ownership is not re-checked here — each single-item use case this composes
/// is already owner-scoped and answers `ProjectNotFound` for a project
/// belonging to someone else.
#[async_trait::async_trait]
pub trait BulkProjectsUseCase: Send + Sync {
    /// Runs the operation over `ids`, in order.
    async fn execute(
        &self,
        owner: crate::auth::application::domain::entities::UserId,
        op: ProjectBulkOp,
        ids: Vec<uuid::Uuid>,
    ) -> Result<crate::shared::api::BulkOutcome, crate::shared::api::BulkRequestError>;
}
