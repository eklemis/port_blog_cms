//! Applies one lifecycle operation across many media items.
//!
//! Composes the single-item use cases, so their owner scoping applies unchanged
//! and there is no second implementation to keep in step.

use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::multimedia::application::ports::incoming::use_cases::{
    BulkMediaUseCase, DeleteMediaError, DeleteMediaUseCase, HardDeleteMediaUseCase, MediaBulkOp,
    MediaLifecycleError, RestoreMediaUseCase,
};
use crate::shared::api::{prepare_ids, BulkOutcome, BulkRequestError, ErrorCode};

/// Implements the corresponding use-case contract.
pub struct BulkMediaService {
    archive: Arc<dyn DeleteMediaUseCase + Send + Sync>,
    restore: Arc<dyn RestoreMediaUseCase + Send + Sync>,
    hard_delete: Arc<dyn HardDeleteMediaUseCase + Send + Sync>,
}

impl BulkMediaService {
    /// Builds it from the single-item use cases it fans out to.
    pub fn new(
        archive: Arc<dyn DeleteMediaUseCase + Send + Sync>,
        restore: Arc<dyn RestoreMediaUseCase + Send + Sync>,
        hard_delete: Arc<dyn HardDeleteMediaUseCase + Send + Sync>,
    ) -> Self {
        Self {
            archive,
            restore,
            hard_delete,
        }
    }
}

/// Archive answers with its own error type; restore and hard delete share
/// `MediaLifecycleError`. Both collapse to the same two outcomes.
fn archive_failure(e: DeleteMediaError) -> (ErrorCode, String) {
    match e {
        DeleteMediaError::MediaNotFound => (ErrorCode::MediaNotFound, e.to_string()),
        DeleteMediaError::RepositoryError(_) => (ErrorCode::InternalError, e.to_string()),
    }
}

fn failure(e: MediaLifecycleError) -> (ErrorCode, String) {
    match e {
        MediaLifecycleError::NotFound => (ErrorCode::MediaNotFound, e.to_string()),
        MediaLifecycleError::RepositoryError(_) => (ErrorCode::InternalError, e.to_string()),
    }
}

#[async_trait]
impl BulkMediaUseCase for BulkMediaService {
    async fn execute(
        &self,
        owner: UserId,
        op: MediaBulkOp,
        ids: Vec<Uuid>,
    ) -> Result<BulkOutcome, BulkRequestError> {
        let ids = prepare_ids(ids)?;
        let mut outcome = BulkOutcome::default();

        for id in ids {
            let result = match op {
                MediaBulkOp::Archive => self
                    .archive
                    .execute(owner, id)
                    .await
                    .map_err(archive_failure),
                MediaBulkOp::Restore => self.restore.execute(owner, id).await.map_err(failure),
                MediaBulkOp::HardDelete => {
                    self.hard_delete.execute(owner, id).await.map_err(failure)
                }
            };

            match result {
                Ok(()) => outcome.succeed(id),
                Err((code, message)) => outcome.fail(id, code, message),
            }
        }

        Ok(outcome)
    }
}
