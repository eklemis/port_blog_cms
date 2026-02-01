// src/modules/project/application/ports/outgoing/project_repository.rs

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;

//
// ──────────────────────────────────────────────────────────
// PatchField (explicit PATCH semantics)
// ──────────────────────────────────────────────────────────
// Meaning:
// - Unset: field not provided => keep DB value
// - Null: explicitly null => set DB column NULL (only for nullable fields)
// - Value(v): replace with v
//
// Serde behavior (recommended usage):
// - omitted field => Unset (because of #[serde(default)])
// - null => Null
// - value => Value(value)
//

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PatchField<T> {
    #[serde(skip)]
    Unset,
    Null,
    Value(T),
}

impl<T> Default for PatchField<T> {
    fn default() -> Self {
        PatchField::Unset
    }
}

impl<T> PatchField<T> {
    pub fn is_unset(&self) -> bool {
        matches!(self, PatchField::Unset)
    }

    pub fn is_null(&self) -> bool {
        matches!(self, PatchField::Null)
    }

    pub fn is_value(&self) -> bool {
        matches!(self, PatchField::Value(_))
    }

    pub fn as_value(&self) -> Option<&T> {
        if let PatchField::Value(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

//
// ──────────────────────────────────────────────────────────
// DTOs
// ──────────────────────────────────────────────────────────
//

#[derive(Debug, Clone)]
pub struct CreateProjectData {
    pub owner: UserId,

    pub title: String,

    /// Slug is immutable: only set at creation time
    pub slug: String,

    pub description: String,

    /// Stored as JSONB in DB (array of strings)
    pub tech_stack: Vec<String>,

    /// Stored as JSONB in DB (array of strings)
    pub screenshots: Vec<String>,

    pub repo_url: Option<String>,
    pub live_demo_url: Option<String>,
}

/// Patch semantics:
/// - title/description: Unset => keep, Value => replace
/// - tech_stack/screenshots: Value(vec) => replace whole array (no merge)
/// - repo_url/live_demo_url: Unset => keep, Null => clear, Value => set
#[derive(Debug, Clone, Default)]
pub struct PatchProjectData {
    pub title: PatchField<String>,
    pub description: PatchField<String>,
    pub tech_stack: PatchField<Vec<String>>,
    pub screenshots: PatchField<Vec<String>>,
    pub repo_url: PatchField<String>,
    pub live_demo_url: PatchField<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectResult {
    pub id: Uuid,
    pub owner: UserId,
    pub title: String,
    pub slug: String,
    pub description: String,
    pub tech_stack: Vec<String>,
    pub screenshots: Vec<String>,
    pub repo_url: Option<String>,
    pub live_demo_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

//
// ──────────────────────────────────────────────────────────
// Errors
// ──────────────────────────────────────────────────────────
//

#[derive(Debug, Clone, thiserror::Error)]
pub enum ProjectRepositoryError {
    /// Project doesn't exist OR doesn't belong to owner.
    #[error("Project not found")]
    NotFound,

    /// Global unique slug violated at INSERT time.
    #[error("Slug already exists")]
    SlugAlreadyExists,

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

//
// ──────────────────────────────────────────────────────────
// Port (Command-side, non-destructive, projects table only)
// ──────────────────────────────────────────────────────────
//

#[async_trait]
pub trait ProjectRepository: Send + Sync {
    async fn create_project(
        &self,
        data: CreateProjectData,
    ) -> Result<ProjectResult, ProjectRepositoryError>;

    /// Patch without pre-read by the use case.
    /// Slug is immutable and MUST NOT be patchable.
    async fn patch_project(
        &self,
        owner: UserId,
        project_id: Uuid,
        data: PatchProjectData,
    ) -> Result<ProjectResult, ProjectRepositoryError>;
}
