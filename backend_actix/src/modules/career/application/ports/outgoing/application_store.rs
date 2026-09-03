//! Read and write side for applications.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::career::domain::entities::{Application, ApplicationStatus};

/// Fields for a new application. Always starts as a draft.
#[derive(Debug, Clone)]
pub struct CreateApplicationData {
    /// The posting being applied to.
    pub job_id: Uuid,
    /// What the applicant owes it next.
    pub next_action: String,
    /// When that is due.
    pub next_action_at: Option<DateTime<Utc>>,
}

/// A partial edit.
#[derive(Debug, Clone, Default)]
pub struct PatchApplicationData {
    /// New status.
    pub status: Option<ApplicationStatus>,
    /// The frozen CV to attach. Set by the service, not by the client
    /// directly — see the use-case documentation.
    pub cv_snapshot_id: Option<Uuid>,
    /// When it was sent.
    pub applied_at: Option<DateTime<Utc>>,
    /// New next action. Empty string clears it.
    pub next_action: Option<String>,
    /// New due date. `Some(None)` clears it, which is why this one is
    /// tri-state where the job's fields are not: a date has no empty value.
    pub next_action_at: Option<Option<DateTime<Utc>>>,
}

impl PatchApplicationData {
    /// True when the caller asked for no change at all.
    pub fn is_empty(&self) -> bool {
        self.status.is_none()
            && self.cv_snapshot_id.is_none()
            && self.applied_at.is_none()
            && self.next_action.is_none()
            && self.next_action_at.is_none()
    }
}

/// Why an application read or write failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ApplicationStoreError {
    /// No application matched, or it belongs to another user.
    #[error("Application not found")]
    NotFound,

    /// The job being applied to does not exist, or is not the caller's.
    #[error("Job not found")]
    JobNotFound,

    /// The store could not be reached.
    #[error("Database error: {0}")]
    DatabaseError(String),
}

/// Stores applications. Every method is owner-scoped in SQL.
#[async_trait]
pub trait ApplicationStore: Send + Sync {
    /// Starts an application against a posting.
    async fn create(
        &self,
        owner: Uuid,
        data: CreateApplicationData,
    ) -> Result<Application, ApplicationStoreError>;

    /// The caller's applications, newest first.
    async fn list(&self, owner: Uuid) -> Result<Vec<Application>, ApplicationStoreError>;

    /// One application.
    async fn find(
        &self,
        owner: Uuid,
        application_id: Uuid,
    ) -> Result<Option<Application>, ApplicationStoreError>;

    /// Edits an application.
    async fn patch(
        &self,
        owner: Uuid,
        application_id: Uuid,
        data: PatchApplicationData,
    ) -> Result<Application, ApplicationStoreError>;

    /// Archives an application.
    async fn archive(&self, owner: Uuid, application_id: Uuid)
        -> Result<(), ApplicationStoreError>;
}
