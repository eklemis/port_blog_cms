//! This module's use cases, grouped for `AppState`.

use std::sync::Arc;

use crate::career::application::ports::incoming::use_cases::{
    AnalyseApplicationUseCase, ArchiveApplicationUseCase, ArchiveJobUseCase,
    CreateApplicationUseCase, CreateJobUseCase, GetApplicationUseCase, GetApplicationsUseCase,
    GetJobUseCase, GetJobsUseCase, PatchApplicationUseCase, PatchJobUseCase,
};

/// Bundles the Career Studio's use cases so `AppState` gains one field.
#[derive(Clone)]
pub struct CareerUseCases {
    /// The [`CreateJobUseCase`] implementation.
    pub create_job: Arc<dyn CreateJobUseCase + Send + Sync>,
    /// The [`GetJobsUseCase`] implementation.
    pub list_jobs: Arc<dyn GetJobsUseCase + Send + Sync>,
    /// The [`GetJobUseCase`] implementation.
    pub get_job: Arc<dyn GetJobUseCase + Send + Sync>,
    /// The [`PatchJobUseCase`] implementation.
    pub patch_job: Arc<dyn PatchJobUseCase + Send + Sync>,
    /// The [`ArchiveJobUseCase`] implementation.
    pub archive_job: Arc<dyn ArchiveJobUseCase + Send + Sync>,
    /// The [`CreateApplicationUseCase`] implementation.
    pub create_application: Arc<dyn CreateApplicationUseCase + Send + Sync>,
    /// The [`GetApplicationsUseCase`] implementation.
    pub list_applications: Arc<dyn GetApplicationsUseCase + Send + Sync>,
    /// The [`GetApplicationUseCase`] implementation.
    pub get_application: Arc<dyn GetApplicationUseCase + Send + Sync>,
    /// The [`PatchApplicationUseCase`] implementation.
    pub patch_application: Arc<dyn PatchApplicationUseCase + Send + Sync>,
    /// The [`ArchiveApplicationUseCase`] implementation.
    pub archive_application: Arc<dyn ArchiveApplicationUseCase + Send + Sync>,
    /// The [`AnalyseApplicationUseCase`] implementation.
    pub analyse: Arc<dyn AnalyseApplicationUseCase + Send + Sync>,
}
