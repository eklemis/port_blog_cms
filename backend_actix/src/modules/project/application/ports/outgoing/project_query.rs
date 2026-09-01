//! Read-side port for projects: listings, single fetches and topic links.
//!
//! Two view shapes are returned: [`ProjectCardView`] for listings and
//! [`ProjectView`] for a single project. Listings deliberately carry less,
//! so a page of twenty does not haul twenty full bodies out of the database.

// src/modules/project/application/ports/outgoing/project_query.rs

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;

//
// ──────────────────────────────────────────────────────────
// Query DTOs
// ──────────────────────────────────────────────────────────
//

/// A topic as it appears attached to a project: just enough to render a tag.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProjectTopicItem {
    pub id: Uuid,
    pub title: String,
    pub description: String,
}

/// A single project in full, including its body and topic links.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ProjectView {
    pub id: Uuid,
    /// Owning user. Serialises as a bare UUID string.
    #[schema(value_type = String, example = "123e4567-e89b-12d3-a456-426614174000")]
    pub owner: UserId,
    pub title: String,
    pub slug: String,
    pub description: String,
    pub tech_stack: Vec<String>,
    pub screenshots: Vec<String>,
    pub repo_url: Option<String>,
    pub live_demo_url: Option<String>,
    pub topics: Vec<ProjectTopicItem>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A project as it appears in a listing — the summary fields only.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProjectCardView {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub tech_stack: Vec<String>,
    pub repo_url: Option<String>,
    pub live_demo_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Narrows a project listing. Every field defaults to "no filter".
#[derive(Debug, Clone, Default)]
pub struct ProjectListFilter {
    pub search: Option<String>,
    pub topic_id: Option<Uuid>,
}

/// Listing order.
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema, Default)]
pub enum ProjectSort {
    /// Newest by creation date first
    Newest,
    /// Oldest by creation date first
    Oldest,
    /// Most recently updated first (default)
    #[default]
    UpdatedNewest,
    /// Least recently updated first
    UpdatedOldest,
}

/// Which page to return. Pages are 1-based.
#[derive(Debug, Clone)]
pub struct PageRequest {
    pub page: u32,
    pub per_page: u32,
}

impl Default for PageRequest {
    fn default() -> Self {
        Self {
            page: 1,
            per_page: 20,
        }
    }
}

/// One page of results, plus the totals a client needs to paginate.
///
/// `total` counts every row matching the filter, not just this page.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PageResult<T> {
    /// Items on the current page
    pub items: Vec<T>,

    /// Current page number, 1-based
    #[schema(example = 1)]
    pub page: u32,

    /// Items per page
    #[schema(example = 10)]
    pub per_page: u32,

    /// Total items across all pages
    #[schema(example = 42)]
    pub total: u64,
}

//
// ──────────────────────────────────────────────────────────
// Errors
// ──────────────────────────────────────────────────────────
//

/// Why a project read failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ProjectQueryError {
    #[error("Project not found")]
    NotFound,

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

//
// ──────────────────────────────────────────────────────────
// Port (Read-side, may join project_topics)
// ──────────────────────────────────────────────────────────
//

/// Reads projects.
///
/// Writes belong to [`ProjectRepository`](super::project_repository::ProjectRepository).
#[async_trait]
pub trait ProjectQuery: Send + Sync {
    /// Owner-scoped read
    async fn get_by_id(
        &self,
        owner: UserId,
        project_id: Uuid,
    ) -> Result<ProjectView, ProjectQueryError>;

    /// Public read (global slug); implement when needed.
    async fn get_by_slug(&self, slug: &str) -> Result<ProjectView, ProjectQueryError>;

    /// Owner-scoped listing with filter/sort/pagination
    async fn list(
        &self,
        owner: UserId,
        filter: ProjectListFilter,
        sort: ProjectSort,
        page: PageRequest,
    ) -> Result<PageResult<ProjectCardView>, ProjectQueryError>;

    /// Sometimes caller needs only topic IDs for a project.
    async fn get_project_topics(
        &self,
        project_id: Uuid,
    ) -> Result<Vec<ProjectTopicItem>, ProjectQueryError>;

    /// Helper to support slug generator later
    async fn slug_exists(&self, slug: &str) -> Result<bool, ProjectQueryError>;
}
