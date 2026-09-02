//! Checking whether a project slug is free.

use async_trait::async_trait;

use crate::auth::application::domain::entities::UserId;
use crate::project::application::ports::incoming::use_cases::GetProjectsError;

/// Reports whether an owner already uses a project slug.
///
/// Returns the *taken* state rather than the available one, so the name and
/// the boolean cannot drift apart in a caller's head.
#[async_trait]
pub trait ProjectSlugAvailableUseCase: Send + Sync {
    /// True when the owner already has a project with this slug.
    async fn execute(&self, owner: UserId, slug: String) -> Result<bool, GetProjectsError>;
}
