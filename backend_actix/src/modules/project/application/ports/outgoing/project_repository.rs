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

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ProjectResult {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// PatchField exists to tell "leave alone" from "set to null" — the
    /// distinction Option cannot express. These predicates are how callers read
    /// that, and getting one wrong would silently turn an omitted field into a
    /// clear, wiping data the caller never mentioned.
    #[test]
    fn the_three_states_are_distinguishable() {
        let unset: PatchField<String> = PatchField::Unset;
        assert!(unset.is_unset() && !unset.is_null() && !unset.is_value());

        let null: PatchField<String> = PatchField::Null;
        assert!(null.is_null() && !null.is_unset() && !null.is_value());

        let value = PatchField::Value("x".to_string());
        assert!(value.is_value() && !value.is_unset() && !value.is_null());
    }

    #[test]
    fn as_value_yields_the_inner_value_only_when_present() {
        assert_eq!(
            PatchField::Value("hello".to_string()).as_value(),
            Some(&"hello".to_string())
        );
        assert_eq!(PatchField::<String>::Null.as_value(), None);
        assert_eq!(PatchField::<String>::Unset.as_value(), None);
    }

    #[test]
    fn the_default_is_unset_so_an_omitted_field_changes_nothing() {
        assert!(PatchField::<String>::default().is_unset());
    }

    /// The enum is `#[serde(untagged)]` with `Unset` skipped, so an absent key
    /// deserialises to Unset while an explicit null becomes Null. That mapping
    /// is the whole contract of a PATCH body.
    #[test]
    fn absent_and_null_deserialise_differently() {
        #[derive(serde::Deserialize)]
        struct Body {
            #[serde(default)]
            title: PatchField<String>,
        }

        let absent: Body = serde_json::from_str("{}").unwrap();
        assert!(absent.title.is_unset());

        let null: Body = serde_json::from_str(r#"{"title":null}"#).unwrap();
        assert!(null.title.is_null());

        let set: Body = serde_json::from_str(r#"{"title":"hi"}"#).unwrap();
        assert_eq!(set.title.as_value(), Some(&"hi".to_string()));
    }
}
