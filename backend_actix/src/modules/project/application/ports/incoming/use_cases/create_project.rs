//! Creates a project.

use async_trait::async_trait;
use std::fmt;

use crate::modules::project::application::ports::outgoing::project_repository::{
    CreateProjectData, ProjectResult,
};

//
// ──────────────────────────────────────────────────────────
// Errors
// ──────────────────────────────────────────────────────────
//

/// Why creating a project failed.
#[derive(Debug, Clone)]
pub enum CreateProjectError {
    SlugAlreadyExists,
    RepositoryError(String),
}

impl fmt::Display for CreateProjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CreateProjectError::SlugAlreadyExists => write!(f, "slug already exists"),
            CreateProjectError::RepositoryError(msg) => {
                write!(f, "repository error: {}", msg)
            }
        }
    }
}

//
// ──────────────────────────────────────────────────────────
// Use case trait
// ──────────────────────────────────────────────────────────
//

/// Creates a project.
///
/// Slugs are unique per owner, so two users may both hold `my-app`.
#[async_trait]
pub trait CreateProjectUseCase: Send + Sync {
    /// Creates the project and returns it as stored.
    async fn execute(&self, data: CreateProjectData) -> Result<ProjectResult, CreateProjectError>;
}
