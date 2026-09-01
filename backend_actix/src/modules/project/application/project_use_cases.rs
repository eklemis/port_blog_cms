use std::sync::Arc;

use crate::{
    modules::project::application::ports::incoming::use_cases::{
        CreateProjectUseCase, GetProjectsUseCase,
    },
    project::application::ports::incoming::use_cases::{
        AddProjectTopicUseCase, ClearProjectTopicsUseCase, GetProjectTopicsUseCase,
        GetPublicSingleProjectUseCase, GetSingleProjectUseCase, HardDeleteProjectUseCase,
        PatchProjectUseCase, RemoveProjectTopicUseCase, SoftDeleteProjectUseCase,
    },
};

/// This module's use cases, grouped for `AppState`.
#[derive(Clone)]
pub struct ProjectUseCases {
    /// The [`CreateProjectUseCase`] implementation.
    pub create: Arc<dyn CreateProjectUseCase + Send + Sync>,
    /// The [`GetProjectsUseCase`] implementation.
    pub get_list: Arc<dyn GetProjectsUseCase + Send + Sync>,
    /// The [`GetSingleProjectUseCase`] implementation.
    pub get_single: Arc<dyn GetSingleProjectUseCase + Send + Sync>,
    /// The [`GetPublicSingleProjectUseCase`] implementation.
    pub get_public_single: Arc<dyn GetPublicSingleProjectUseCase + Send + Sync>,
    /// The [`PatchProjectUseCase`] implementation.
    pub patch: Arc<dyn PatchProjectUseCase + Send + Sync>,
    /// The [`GetProjectTopicsUseCase`] implementation.
    pub get_topics: Arc<dyn GetProjectTopicsUseCase + Send + Sync>,
    /// The [`AddProjectTopicUseCase`] implementation.
    pub add_topic: Arc<dyn AddProjectTopicUseCase + Send + Sync>,
    /// The [`RemoveProjectTopicUseCase`] implementation.
    pub remove_topic: Arc<dyn RemoveProjectTopicUseCase + Send + Sync>,
    /// The [`ClearProjectTopicsUseCase`] implementation.
    pub clear_topics: Arc<dyn ClearProjectTopicsUseCase + Send + Sync>,
    /// The [`HardDeleteProjectUseCase`] implementation.
    pub hard_delete: Arc<dyn HardDeleteProjectUseCase + Send + Sync>,
    /// The [`SoftDeleteProjectUseCase`] implementation.
    pub soft_delete: Arc<dyn SoftDeleteProjectUseCase + Send + Sync>,
}
