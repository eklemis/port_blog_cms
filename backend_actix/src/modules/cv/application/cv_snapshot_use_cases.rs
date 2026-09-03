//! The snapshot use cases, grouped for `AppState`.

use std::sync::Arc;

use crate::cv::application::use_cases::cv_snapshots::{
    CreateCvSnapshotUseCase, GetCvSnapshotUseCase,
};

/// Bundles the two snapshot use cases so `AppState` gains one field.
#[derive(Clone)]
pub struct CvSnapshotUseCases {
    /// The [`CreateCvSnapshotUseCase`] implementation.
    pub create: Arc<dyn CreateCvSnapshotUseCase + Send + Sync>,
    /// The [`GetCvSnapshotUseCase`] implementation.
    pub get: Arc<dyn GetCvSnapshotUseCase + Send + Sync>,
}
