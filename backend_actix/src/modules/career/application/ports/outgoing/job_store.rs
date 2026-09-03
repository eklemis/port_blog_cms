//! Read and write side for postings.

use async_trait::async_trait;
use uuid::Uuid;

use crate::career::domain::entities::Job;

/// Fields for a new posting.
#[derive(Debug, Clone, Default)]
pub struct CreateJobData {
    /// Role title.
    pub title: String,
    /// Hiring company.
    pub company: String,
    /// Where the role is.
    pub location: String,
    /// Seniority as advertised.
    pub seniority: String,
    /// Extracted must-haves.
    pub required_skills: Vec<String>,
    /// Extracted nice-to-haves.
    pub nice_to_have: Vec<String>,
    /// Where it was found.
    pub source_url: String,
    /// The posting verbatim.
    pub source_text: String,
}

/// A partial edit. `None` leaves a field alone.
///
/// Plain `Option` rather than the tri-state used elsewhere: none of these are
/// nullable in the database — the absent value is an empty string or an empty
/// list, and "clear it" and "set it to empty" are the same operation.
#[derive(Debug, Clone, Default)]
pub struct PatchJobData {
    /// New title.
    pub title: Option<String>,
    /// New company.
    pub company: Option<String>,
    /// New location.
    pub location: Option<String>,
    /// New seniority.
    pub seniority: Option<String>,
    /// Replacement must-haves.
    pub required_skills: Option<Vec<String>>,
    /// Replacement nice-to-haves.
    pub nice_to_have: Option<Vec<String>>,
    /// New source URL.
    pub source_url: Option<String>,
    /// Replacement source text. Rarely used — see [`Job::source_text`].
    pub source_text: Option<String>,
}

impl PatchJobData {
    /// True when the caller asked for no change at all.
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.company.is_none()
            && self.location.is_none()
            && self.seniority.is_none()
            && self.required_skills.is_none()
            && self.nice_to_have.is_none()
            && self.source_url.is_none()
            && self.source_text.is_none()
    }
}

/// Why a job read or write failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum JobStoreError {
    /// No job matched, or it belongs to another user.
    #[error("Job not found")]
    NotFound,

    /// The store could not be reached.
    #[error("Database error: {0}")]
    DatabaseError(String),
}

/// Stores postings. Every method is owner-scoped in SQL.
#[async_trait]
pub trait JobStore: Send + Sync {
    /// Captures a posting.
    async fn create(&self, owner: Uuid, data: CreateJobData) -> Result<Job, JobStoreError>;

    /// The caller's postings, newest first. Archived ones are excluded.
    async fn list(&self, owner: Uuid) -> Result<Vec<Job>, JobStoreError>;

    /// One posting.
    async fn find(&self, owner: Uuid, job_id: Uuid) -> Result<Option<Job>, JobStoreError>;

    /// Edits a posting.
    async fn patch(
        &self,
        owner: Uuid,
        job_id: Uuid,
        data: PatchJobData,
    ) -> Result<Job, JobStoreError>;

    /// Archives a posting.
    ///
    /// Soft, matching every other resource in this API. Applications keep
    /// pointing at it — an application whose posting vanished would lose the
    /// only record of what was asked for.
    async fn archive(&self, owner: Uuid, job_id: Uuid) -> Result<(), JobStoreError>;
}
