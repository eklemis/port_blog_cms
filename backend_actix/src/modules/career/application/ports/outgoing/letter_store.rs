//! Storage for cover letters and reflections.
//!
//! One port rather than two because they are the same shape — at most one row
//! per application, owner-scoped, created on first write — and splitting them
//! would duplicate the scoping rather than clarify anything.

use async_trait::async_trait;
use uuid::Uuid;

use crate::career::domain::entities::{CoverLetter, CoverLetterStatus, Reflection};

/// Fields for writing a cover letter. `None` leaves a field alone.
#[derive(Debug, Clone, Default)]
pub struct PatchCoverLetterData {
    /// New body.
    pub content: Option<String>,
    /// New language. Explicit, never inferred from the text.
    pub language: Option<String>,
    /// New status.
    pub status: Option<CoverLetterStatus>,
}

impl PatchCoverLetterData {
    /// True when the caller asked for no change at all.
    pub fn is_empty(&self) -> bool {
        self.content.is_none() && self.language.is_none() && self.status.is_none()
    }
}

/// Fields for writing a reflection.
#[derive(Debug, Clone, Default)]
pub struct ReflectionData {
    /// How far it got.
    pub stage_reached: String,
    /// What happened.
    pub what_happened: String,
    /// What they would change.
    pub what_id_change: String,
}

/// Why a letter or reflection read or write failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum LetterStoreError {
    /// No application matched, or it belongs to another user.
    #[error("Application not found")]
    ApplicationNotFound,

    /// The store could not be reached.
    #[error("Database error: {0}")]
    DatabaseError(String),
}

/// Reads and writes the two documents that hang off an application.
#[async_trait]
pub trait LetterStore: Send + Sync {
    /// The application's cover letter, if it has one.
    async fn find_letter(
        &self,
        owner: Uuid,
        application_id: Uuid,
    ) -> Result<Option<CoverLetter>, LetterStoreError>;

    /// Writes the cover letter, creating it on first write.
    async fn upsert_letter(
        &self,
        owner: Uuid,
        application_id: Uuid,
        data: PatchCoverLetterData,
    ) -> Result<CoverLetter, LetterStoreError>;

    /// Removes the cover letter. Deleting one that is not there succeeds.
    async fn delete_letter(
        &self,
        owner: Uuid,
        application_id: Uuid,
    ) -> Result<(), LetterStoreError>;

    /// The application's reflection, if it has one.
    async fn find_reflection(
        &self,
        owner: Uuid,
        application_id: Uuid,
    ) -> Result<Option<Reflection>, LetterStoreError>;

    /// Writes the reflection, creating it on first write.
    async fn upsert_reflection(
        &self,
        owner: Uuid,
        application_id: Uuid,
        data: ReflectionData,
    ) -> Result<Reflection, LetterStoreError>;

    /// Removes the reflection.
    ///
    /// Real deletion, not a flag. Someone withdrawing a private note about
    /// their own rejection should not discover it was only hidden.
    async fn delete_reflection(
        &self,
        owner: Uuid,
        application_id: Uuid,
    ) -> Result<(), LetterStoreError>;
}
