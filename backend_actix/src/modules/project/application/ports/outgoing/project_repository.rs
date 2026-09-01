//! Write-side port for projects.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;

/// Three-way PATCH semantics: leave alone, set to null, or replace.
///
/// `Option<T>` cannot express this. A PATCH body that omits a field and one
/// that sends `null` both deserialise to `None`, so an endpoint using `Option`
/// cannot tell "don't touch this" from "clear this".
///
/// | Variant | JSON | Effect |
/// | --- | --- | --- |
/// | [`Unset`](Self::Unset) | field omitted | keep the stored value |
/// | [`Null`](Self::Null) | `null` | set the column to NULL — nullable columns only |
/// | [`Value`](Self::Value) | any value | replace |
///
/// Deserialising omitted fields as `Unset` relies on `#[serde(default)]` at
/// the field's use site; without it, serde errors on the missing field instead.
///
/// `blog` carries its own copy as `BlogPatchField`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
#[derive(Default)]
pub enum PatchField<T> {
    /// The client did not mention the field. Keep the stored value.
    #[serde(skip)]
    #[default]
    Unset,
    /// The client sent `null`. Set the column to NULL — nullable columns only.
    Null,
    /// The client sent a replacement.
    Value(T),
}

impl<T> PatchField<T> {
    /// True when the client did not mention this field.
    pub fn is_unset(&self) -> bool {
        matches!(self, PatchField::Unset)
    }

    /// True when the client explicitly sent `null`, asking to clear it.
    pub fn is_null(&self) -> bool {
        matches!(self, PatchField::Null)
    }

    /// True when the client supplied a replacement value.
    pub fn is_value(&self) -> bool {
        matches!(self, PatchField::Value(_))
    }

    /// The replacement value, if any. Both `Unset` and `Null` yield `None` —
    /// use [`is_null`](Self::is_null) to tell them apart.
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

/// Everything needed to insert a project.
#[derive(Debug, Clone)]
pub struct CreateProjectData {
    /// The user the project belongs to.
    pub owner: UserId,

    /// Display title.
    pub title: String,

    /// Slug is immutable: only set at creation time
    pub slug: String,

    /// Long-form body.
    pub description: String,

    /// Stored as JSONB in DB (array of strings)
    pub tech_stack: Vec<String>,

    /// Stored as JSONB in DB (array of strings)
    pub screenshots: Vec<String>,

    /// Source repository, if there is one.
    pub repo_url: Option<String>,
    /// Running instance, if there is one.
    pub live_demo_url: Option<String>,
}

/// Patch semantics:
/// - title/description: Unset => keep, Value => replace
/// - tech_stack/screenshots: Value(vec) => replace whole array (no merge)
/// - repo_url/live_demo_url: Unset => keep, Null => clear, Value => set
#[derive(Debug, Clone, Default)]
pub struct PatchProjectData {
    /// New title, if the client sent one.
    pub title: PatchField<String>,
    /// New body, if the client sent one.
    pub description: PatchField<String>,
    /// Replaces the whole list. `Null` clears it.
    pub tech_stack: PatchField<Vec<String>>,
    /// Replaces the whole list. `Null` clears it.
    pub screenshots: PatchField<Vec<String>>,
    /// New repository URL. `Null` clears it.
    pub repo_url: PatchField<String>,
    /// New demo URL. `Null` clears it.
    pub live_demo_url: PatchField<String>,
}

/// A project as returned after a write.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ProjectResult {
    /// Primary key.
    pub id: Uuid,
    /// Owning user. Serialises as a bare UUID string.
    #[schema(value_type = String, example = "123e4567-e89b-12d3-a456-426614174000")]
    pub owner: UserId,
    /// Display title.
    pub title: String,
    /// URL segment. Unique per owner.
    pub slug: String,
    /// Long-form body.
    pub description: String,
    /// Technology labels, in the order the owner set them.
    pub tech_stack: Vec<String>,
    /// Image URLs, in display order.
    pub screenshots: Vec<String>,
    /// Source repository, if there is one.
    pub repo_url: Option<String>,
    /// Running instance, if there is one.
    pub live_demo_url: Option<String>,
    /// When the project was created.
    pub created_at: DateTime<Utc>,
    /// When it was last edited.
    pub updated_at: DateTime<Utc>,
}

//
// ──────────────────────────────────────────────────────────
// Errors
// ──────────────────────────────────────────────────────────
//

/// Why a project write failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ProjectRepositoryError {
    /// Project doesn't exist OR doesn't belong to owner.
    #[error("Project not found")]
    NotFound,

    /// Global unique slug violated at INSERT time.
    #[error("Slug already exists")]
    SlugAlreadyExists,

    /// The store could not be reached.
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// A stored column could not be decoded into its Rust type.
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

//
// ──────────────────────────────────────────────────────────
// Port (Command-side, non-destructive, projects table only)
// ──────────────────────────────────────────────────────────
//

/// Creates and edits projects.
///
/// Reads belong to [`ProjectQuery`](super::project_query::ProjectQuery),
/// lifecycle transitions to [`ProjectArchiver`](super::project_archiver::ProjectArchiver),
/// and topic links to [`ProjectTopicRepository`](super::project_topic_repository::ProjectTopicRepository).
#[async_trait]
pub trait ProjectRepository: Send + Sync {
    /// Inserts a project.
    ///
    /// # Errors
    /// [`SlugAlreadyExists`](ProjectRepositoryError::SlugAlreadyExists) if the
    /// owner already uses that slug.
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
