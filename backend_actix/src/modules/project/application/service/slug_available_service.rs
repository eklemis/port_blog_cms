//! Reports whether a project slug is free.

use async_trait::async_trait;

use crate::auth::application::domain::entities::UserId;
use crate::project::application::ports::incoming::use_cases::{
    GetProjectsError, ProjectSlugAvailableUseCase,
};
use crate::project::application::ports::outgoing::project_query::ProjectQuery;

/// Implements the corresponding use-case contract.
#[derive(Debug, Clone)]
pub struct ProjectSlugAvailableService<Q> {
    query: Q,
}

impl<Q> ProjectSlugAvailableService<Q> {
    /// Builds it from the ports it depends on.
    pub fn new(query: Q) -> Self {
        Self { query }
    }
}

#[async_trait]
impl<Q: ProjectQuery + Send + Sync> ProjectSlugAvailableUseCase for ProjectSlugAvailableService<Q> {
    async fn execute(&self, owner: UserId, slug: String) -> Result<bool, GetProjectsError> {
        self.query
            .slug_exists(owner, &slug)
            .await
            .map_err(|e| GetProjectsError::QueryFailed(e.to_string()))
    }
}
