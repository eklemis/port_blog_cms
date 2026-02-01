use std::sync::Arc;

use crate::{
    modules::project::application::ports::incoming::use_cases::{
        CreateProjectUseCase, GetProjectsUseCase,
    },
    project::application::ports::incoming::use_cases::GetSingleProjectUseCase,
};

#[derive(Clone)]
pub struct ProjectUseCases {
    pub create: Arc<dyn CreateProjectUseCase + Send + Sync>,
    pub get_list: Arc<dyn GetProjectsUseCase + Send + Sync>,
    pub get_single: Arc<dyn GetSingleProjectUseCase + Send + Sync>,
}
