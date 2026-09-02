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

use crate::multimedia::application::domain::entities::PublicMedia;

use crate::auth::application::domain::entities::UserId;

//
// ──────────────────────────────────────────────────────────
// Query DTOs
// ──────────────────────────────────────────────────────────
//

/// A topic as it appears attached to a project: just enough to render a tag.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProjectTopicItem {
    /// Primary key.
    pub id: Uuid,
    /// Display title, as the owner wrote it.
    pub title: String,
    /// Long-form body.
    pub description: String,
}

/// A single project in full, including its body and topic links.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ProjectView {
    /// Primary key.
    pub id: Uuid,
    /// Owning user. Serialises as a bare UUID string.
    #[schema(value_type = String, example = "123e4567-e89b-12d3-a456-426614174000")]
    pub owner: UserId,
    /// Display title, as the owner wrote it.
    pub title: String,
    /// URL segment. Unique per owner, so two users may hold the same one.
    pub slug: String,
    /// Long-form body.
    pub description: String,
    /// Free-form technology labels, in the order the owner set them.
    pub tech_stack: Vec<String>,
    /// Image URLs, in display order.
    pub screenshots: Vec<String>,
    /// Source repository, if the owner published one.
    pub repo_url: Option<String>,
    /// Running instance, if there is one.
    pub live_demo_url: Option<String>,
    /// Topics attached to this project.
    pub topics: Vec<ProjectTopicItem>,
    /// When the project was created.
    pub created_at: DateTime<Utc>,
    /// When it was last edited.
    pub updated_at: DateTime<Utc>,

    /// Media attached to the project, on the **public** read path only.
    ///
    /// Each item carries its `role`, so a client picks screenshots with
    /// `role == "screenshot"` and a cover with `role == "cover"`.
    ///
    /// Distinct from [`screenshots`](Self::screenshots), which is a plain list
    /// of author-supplied URLs stored on the project row. The two coexist; this
    /// one is backed by uploaded media and carries generated sizes.
    pub media: Vec<PublicMedia>,
}

/// A project as it appears in a listing — the summary fields only.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProjectCardView {
    /// Primary key.
    pub id: Uuid,
    /// Display title, as the owner wrote it.
    pub title: String,
    /// URL segment. Unique per owner, so two users may hold the same one.
    pub slug: String,
    /// Free-form technology labels, in the order the owner set them.
    pub tech_stack: Vec<String>,
    /// Source repository, if the owner published one.
    pub repo_url: Option<String>,
    /// Running instance, if there is one.
    pub live_demo_url: Option<String>,
    /// When the project was created.
    pub created_at: DateTime<Utc>,
    /// When it was last edited.
    pub updated_at: DateTime<Utc>,

    /// The project's cover, on public listings only.
    pub cover: Option<PublicMedia>,
}

/// Narrows a project listing. Every field defaults to "no filter".
#[derive(Debug, Clone, Default)]
pub struct ProjectListFilter {
    /// Free-text filter. `None` matches everything.
    pub search: Option<String>,
    /// Restricts to projects carrying this topic. `None` matches everything.
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
    /// 1-based page number.
    pub page: u32,
    /// Rows per page.
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
    /// No project matched. Only meaningful for single fetches; a listing that
    /// matches nothing is an empty page.
    #[error("Project not found")]
    NotFound,

    /// The store could not be reached.
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// A stored column could not be decoded into its Rust type — most likely a
    /// JSON column written by an older schema.
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
    /// Whether this owner already uses a slug.
    ///
    /// **Scoped by owner**, because the unique index is `(user_id,
    /// lower(slug))` — see `m20260830_000003_fix_projects_slug_uniqueness`.
    /// Checking globally would report a slug as taken when another author
    /// happens to hold it, which is exactly the behaviour that migration
    /// removed at the database level.
    ///
    /// Soft-deleted projects do not count: their slug is free to reuse.
    async fn slug_exists(&self, owner: UserId, slug: &str) -> Result<bool, ProjectQueryError>;
}
