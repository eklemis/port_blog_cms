use std::sync::Arc;

use crate::{
    modules::project::application::ports::incoming::use_cases::{
        CreateProjectUseCase, GetProjectsUseCase,
    },
    project::application::ports::incoming::use_cases::{
        AddProjectTopicUseCase, ClearProjectTopicsUseCase, GetProjectTopicsUseCase,
        GetPublicSingleProjectUseCase, GetSingleProjectUseCase, HardDeleteProjectUseCase,
        PatchProjectUseCase, RemoveProjectTopicUseCase,
    },
};

#[derive(Clone)]
pub struct ProjectUseCases {
    pub create: Arc<dyn CreateProjectUseCase + Send + Sync>,
    pub get_list: Arc<dyn GetProjectsUseCase + Send + Sync>,
    pub get_single: Arc<dyn GetSingleProjectUseCase + Send + Sync>,
    pub get_public_single: Arc<dyn GetPublicSingleProjectUseCase + Send + Sync>,
    pub patch: Arc<dyn PatchProjectUseCase + Send + Sync>,
    pub get_topics: Arc<dyn GetProjectTopicsUseCase + Send + Sync>,
    pub add_topic: Arc<dyn AddProjectTopicUseCase + Send + Sync>,
    pub remove_topic: Arc<dyn RemoveProjectTopicUseCase + Send + Sync>,
    pub clear_topics: Arc<dyn ClearProjectTopicsUseCase + Send + Sync>,
    pub hard_delete: Arc<dyn HardDeleteProjectUseCase + Send + Sync>,
}
