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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectTopicItem {
    pub id: Uuid,
    pub title: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectView {
    pub id: Uuid,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Default)]
pub struct ProjectListFilter {
    pub search: Option<String>,
    pub topic_id: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
pub enum ProjectSort {
    Newest,
    Oldest,
    UpdatedNewest,
    UpdatedOldest,
}

impl Default for ProjectSort {
    fn default() -> Self {
        ProjectSort::UpdatedNewest
    }
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageResult<T> {
    pub items: Vec<T>,
    pub page: u32,
    pub per_page: u32,
    pub total: u64,
}

//
// ──────────────────────────────────────────────────────────
// Errors
// ──────────────────────────────────────────────────────────
//

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
