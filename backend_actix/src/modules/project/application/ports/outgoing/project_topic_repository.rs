// src/modules/project/application/ports/outgoing/project_topic_repository.rs

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;

//
// ──────────────────────────────────────────────────────────
// Errors
// ──────────────────────────────────────────────────────────
//

#[derive(Debug, Clone, thiserror::Error)]
pub enum ProjectTopicRepositoryError {
    /// Project doesn't exist OR doesn't belong to owner.
    #[error("Project not found")]
    ProjectNotFound,

    /// Topic doesn't exist (if adapter enforces it via FK/explicit check).
    #[error("Topic not found")]
    TopicNotFound,

    #[error("Database error: {0}")]
    DatabaseError(String),
}

//
// ──────────────────────────────────────────────────────────
// Port (Command-side, project_topics table only)
// ──────────────────────────────────────────────────────────
//

#[async_trait]
pub trait ProjectTopicRepository: Send + Sync {
    /// Add one topic link to a project.
    /// Recommended to be idempotent:
    /// - unique(project_id, topic_id) + insert on conflict do nothing
    async fn add_project_topic(
        &self,
        owner: UserId,
        project_id: Uuid,
        topic_id: Uuid,
    ) -> Result<(), ProjectTopicRepositoryError>;

    /// Remove one topic link from a project.
    async fn remove_project_topic(
        &self,
        owner: UserId,
        project_id: Uuid,
        topic_id: Uuid,
    ) -> Result<(), ProjectTopicRepositoryError>;

    /// Remove all topic links for a project.
    async fn clear_project_topics(
        &self,
        owner: UserId,
        project_id: Uuid,
    ) -> Result<(), ProjectTopicRepositoryError>;

    /// Replace all topic links for a project:
    /// - delete existing links
    /// - insert (project_id, topic_id) in batch
    async fn set_project_topics(
        &self,
        owner: UserId,
        project_id: Uuid,
        topic_ids: Vec<Uuid>,
    ) -> Result<(), ProjectTopicRepositoryError>;
}
