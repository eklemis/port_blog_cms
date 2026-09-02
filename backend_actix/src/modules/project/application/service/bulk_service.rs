//! Applies one lifecycle or topic operation across many projects.
//!
//! Composes the single-item use cases, so their ownership rules apply unchanged
//! and there is no second implementation to keep in step. See the blog
//! equivalent for the reasoning in full.

use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::project::application::ports::incoming::use_cases::{
    AddProjectTopicError, AddProjectTopicUseCase, BulkProjectsUseCase, HardDeleteProjectError,
    HardDeleteProjectUseCase, ProjectBulkOp, RemoveProjectTopicError, RemoveProjectTopicUseCase,
    RestoreProjectError, RestoreProjectUseCase, SoftDeleteProjectError, SoftDeleteProjectUseCase,
};
use crate::shared::api::{prepare_ids, BulkOutcome, BulkRequestError, ErrorCode};

/// Implements the corresponding use-case contract.
pub struct BulkProjectsService {
    archive: Arc<dyn SoftDeleteProjectUseCase + Send + Sync>,
    restore: Arc<dyn RestoreProjectUseCase + Send + Sync>,
    hard_delete: Arc<dyn HardDeleteProjectUseCase + Send + Sync>,
    attach_topic: Arc<dyn AddProjectTopicUseCase + Send + Sync>,
    detach_topic: Arc<dyn RemoveProjectTopicUseCase + Send + Sync>,
}

impl BulkProjectsService {
    /// Builds it from the single-item use cases it fans out to.
    pub fn new(
        archive: Arc<dyn SoftDeleteProjectUseCase + Send + Sync>,
        restore: Arc<dyn RestoreProjectUseCase + Send + Sync>,
        hard_delete: Arc<dyn HardDeleteProjectUseCase + Send + Sync>,
        attach_topic: Arc<dyn AddProjectTopicUseCase + Send + Sync>,
        detach_topic: Arc<dyn RemoveProjectTopicUseCase + Send + Sync>,
    ) -> Self {
        Self {
            archive,
            restore,
            hard_delete,
            attach_topic,
            detach_topic,
        }
    }
}

#[async_trait]
impl BulkProjectsUseCase for BulkProjectsService {
    async fn execute(
        &self,
        owner: UserId,
        op: ProjectBulkOp,
        ids: Vec<Uuid>,
    ) -> Result<BulkOutcome, BulkRequestError> {
        let ids = prepare_ids(ids)?;
        let mut outcome = BulkOutcome::default();

        // Sequential: each item is a database write, and a hundred at once
        // would exhaust the pool for every other request in flight.
        for id in ids {
            let result: Result<(), (ErrorCode, String)> = match &op {
                ProjectBulkOp::Archive => {
                    self.archive.execute(owner, id).await.map_err(|e| match e {
                        SoftDeleteProjectError::ProjectNotFound => {
                            (ErrorCode::ProjectNotFound, e.to_string())
                        }
                        SoftDeleteProjectError::RepositoryError(_) => {
                            (ErrorCode::InternalError, e.to_string())
                        }
                    })
                }
                ProjectBulkOp::Restore => {
                    self.restore.execute(owner, id).await.map_err(|e| match e {
                        RestoreProjectError::ProjectNotFound => {
                            (ErrorCode::ProjectNotFound, e.to_string())
                        }
                        RestoreProjectError::RepositoryError(_) => {
                            (ErrorCode::InternalError, e.to_string())
                        }
                    })
                }
                ProjectBulkOp::HardDelete => {
                    self.hard_delete
                        .execute(owner, id)
                        .await
                        .map_err(|e| match e {
                            HardDeleteProjectError::ProjectNotFound => {
                                (ErrorCode::ProjectNotFound, e.to_string())
                            }
                            HardDeleteProjectError::RepositoryError(_) => {
                                (ErrorCode::InternalError, e.to_string())
                            }
                        })
                }
                ProjectBulkOp::AttachTopic { topic_id } => self
                    .attach_topic
                    .execute(owner, id, *topic_id)
                    .await
                    .map_err(|e| match e {
                        AddProjectTopicError::ProjectNotFound => {
                            (ErrorCode::ProjectNotFound, e.to_string())
                        }
                        AddProjectTopicError::TopicNotFound => {
                            (ErrorCode::TopicNotFound, e.to_string())
                        }
                        AddProjectTopicError::RepositoryError(_) => {
                            (ErrorCode::InternalError, e.to_string())
                        }
                    }),
                ProjectBulkOp::DetachTopic { topic_id } => self
                    .detach_topic
                    .execute(owner, id, *topic_id)
                    .await
                    .map_err(|e| match e {
                        RemoveProjectTopicError::ProjectNotFound => {
                            (ErrorCode::ProjectNotFound, e.to_string())
                        }
                        RemoveProjectTopicError::TopicNotFound => {
                            (ErrorCode::TopicNotFound, e.to_string())
                        }
                        RemoveProjectTopicError::RepositoryError(_) => {
                            (ErrorCode::InternalError, e.to_string())
                        }
                    }),
            };

            match result {
                Ok(()) => outcome.succeed(id),
                Err((code, message)) => outcome.fail(id, code, message),
            }
        }

        Ok(outcome)
    }
}
