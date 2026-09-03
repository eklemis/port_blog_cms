//! Supplies `career` with CV snapshots, without `career` learning how CVs
//! work.
//!
//! `career` declares `CvSnapshotter`; this implements it over the `cv`
//! module's own snapshot use case, so there is one implementation of what
//! freezing a CV means and both callers get it.

use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::career::application::ports::outgoing::{CvSnapshotter, CvSnapshotterError};
use crate::cv::application::use_cases::cv_snapshots::{CreateCvSnapshotUseCase, CvSnapshotError};

/// Takes snapshots through the `cv` module.
pub struct CvSnapshotterCv {
    create: Arc<dyn CreateCvSnapshotUseCase + Send + Sync>,
}

impl CvSnapshotterCv {
    /// Builds it from the ports it depends on.
    pub fn new(create: Arc<dyn CreateCvSnapshotUseCase + Send + Sync>) -> Self {
        Self { create }
    }
}

#[async_trait]
impl CvSnapshotter for CvSnapshotterCv {
    async fn snapshot(&self, owner: Uuid, cv_id: Uuid) -> Result<Uuid, CvSnapshotterError> {
        self.create
            .execute(UserId::from(owner), cv_id)
            .await
            .map(|snapshot| snapshot.id)
            .map_err(|e| match e {
                CvSnapshotError::CvNotFound => CvSnapshotterError::CvNotFound,
                other => CvSnapshotterError::Failed(other.to_string()),
            })
    }
}
