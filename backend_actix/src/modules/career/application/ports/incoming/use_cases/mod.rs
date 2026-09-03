//! What the route layer may ask the Career Studio to do.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::career::application::ports::outgoing::{
    ApplicationStoreError, CreateApplicationData, CreateJobData, CvSnapshotterError, JobStoreError,
    PatchJobData,
};
use crate::career::domain::entities::{Application, ApplicationStatus, Job};

/// Why a job operation failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum JobError {
    /// No job matched, or it belongs to another user.
    #[error("Job not found")]
    NotFound,

    /// The posting has no title or no company.
    #[error("{0}")]
    Invalid(String),

    /// The store could not be reached.
    #[error("Repository error: {0}")]
    RepositoryError(String),
}

impl From<JobStoreError> for JobError {
    fn from(e: JobStoreError) -> Self {
        match e {
            JobStoreError::NotFound => JobError::NotFound,
            JobStoreError::DatabaseError(m) => JobError::RepositoryError(m),
        }
    }
}

/// Why an application operation failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ApplicationError {
    /// No application matched, or it belongs to another user.
    #[error("Application not found")]
    NotFound,

    /// The posting does not exist, or is not the caller's.
    #[error("Job not found")]
    JobNotFound,

    /// No CV matched the one asked to be frozen.
    #[error("CV not found")]
    CvNotFound,

    /// The application was asked to leave draft with nothing to point at.
    ///
    /// Its own variant rather than a generic validation error because it is
    /// the one refusal a client will hit routinely, and it has a specific
    /// remedy: send `cv_id` so a snapshot can be taken.
    #[error("Leaving draft requires a CV: send cv_id, or attach a snapshot first")]
    SnapshotRequired,

    /// The store could not be reached.
    #[error("Repository error: {0}")]
    RepositoryError(String),
}

impl From<ApplicationStoreError> for ApplicationError {
    fn from(e: ApplicationStoreError) -> Self {
        match e {
            ApplicationStoreError::NotFound => ApplicationError::NotFound,
            ApplicationStoreError::JobNotFound => ApplicationError::JobNotFound,
            ApplicationStoreError::DatabaseError(m) => ApplicationError::RepositoryError(m),
        }
    }
}

impl From<CvSnapshotterError> for ApplicationError {
    fn from(e: CvSnapshotterError) -> Self {
        match e {
            CvSnapshotterError::CvNotFound => ApplicationError::CvNotFound,
            CvSnapshotterError::Failed(m) => ApplicationError::RepositoryError(m),
        }
    }
}

/// An edit to an application, as the route layer expresses it.
#[derive(Debug, Clone, Default)]
pub struct UpdateApplicationInput {
    /// New status.
    pub status: Option<ApplicationStatus>,

    /// A CV to freeze and attach as part of this edit.
    ///
    /// This is how "the snapshot is taken automatically when an application
    /// leaves draft" is expressed: the client names the CV it is applying
    /// with, and the service takes the copy. It is not a stored field — what
    /// gets stored is the resulting snapshot id.
    pub cv_id: Option<Uuid>,

    /// New next action.
    pub next_action: Option<String>,

    /// New due date. `Some(None)` clears it.
    pub next_action_at: Option<Option<DateTime<Utc>>>,
}

/// Captures a posting.
#[async_trait]
pub trait CreateJobUseCase: Send + Sync {
    /// Stores it and returns it.
    async fn execute(&self, owner: UserId, data: CreateJobData) -> Result<Job, JobError>;
}

/// Lists the caller's postings.
#[async_trait]
pub trait GetJobsUseCase: Send + Sync {
    /// Newest first.
    async fn execute(&self, owner: UserId) -> Result<Vec<Job>, JobError>;
}

/// Reads one posting.
#[async_trait]
pub trait GetJobUseCase: Send + Sync {
    /// Returns it, or `NotFound`.
    async fn execute(&self, owner: UserId, job_id: Uuid) -> Result<Job, JobError>;
}

/// Edits a posting.
#[async_trait]
pub trait PatchJobUseCase: Send + Sync {
    /// Applies the edit and returns the stored posting.
    async fn execute(
        &self,
        owner: UserId,
        job_id: Uuid,
        data: PatchJobData,
    ) -> Result<Job, JobError>;
}

/// Archives a posting.
#[async_trait]
pub trait ArchiveJobUseCase: Send + Sync {
    /// Soft, like every other archive in this API.
    async fn execute(&self, owner: UserId, job_id: Uuid) -> Result<(), JobError>;
}

/// Starts an application against a posting.
#[async_trait]
pub trait CreateApplicationUseCase: Send + Sync {
    /// Always starts as a draft.
    async fn execute(
        &self,
        owner: UserId,
        data: CreateApplicationData,
    ) -> Result<Application, ApplicationError>;
}

/// Lists the caller's applications.
#[async_trait]
pub trait GetApplicationsUseCase: Send + Sync {
    /// Newest first.
    async fn execute(&self, owner: UserId) -> Result<Vec<Application>, ApplicationError>;
}

/// Reads one application.
#[async_trait]
pub trait GetApplicationUseCase: Send + Sync {
    /// Returns it, or `NotFound`.
    async fn execute(
        &self,
        owner: UserId,
        application_id: Uuid,
    ) -> Result<Application, ApplicationError>;
}

/// Edits an application, taking a CV snapshot when it leaves draft.
#[async_trait]
pub trait PatchApplicationUseCase: Send + Sync {
    /// Applies the edit and returns the stored application.
    ///
    /// **Leaving draft requires a snapshot.** If the application does not
    /// already have one and no `cv_id` is supplied, this refuses with
    /// [`ApplicationError::SnapshotRequired`] rather than storing a row that
    /// will misreport what was sent.
    async fn execute(
        &self,
        owner: UserId,
        application_id: Uuid,
        input: UpdateApplicationInput,
    ) -> Result<Application, ApplicationError>;
}

/// Archives an application.
#[async_trait]
pub trait ArchiveApplicationUseCase: Send + Sync {
    /// Soft, like every other archive in this API.
    async fn execute(&self, owner: UserId, application_id: Uuid) -> Result<(), ApplicationError>;
}

/// A match analysis: what can be measured, and what a model estimated.
///
/// The two halves are reported separately and **never averaged**. Half of this
/// is genuinely measurable and half is an estimate; one blended number would
/// hide which half a person should trust.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct MatchAnalysis {
    /// The deterministic half. Always present.
    pub readability: crate::career::domain::readability::ReadabilityReport,

    /// The estimated half.
    ///
    /// **`null` means "not computed", never "scored zero".** The model-backed
    /// half arrives with the AI proxy; until then this endpoint answers with
    /// the measured half alone, and a client should render one bar rather than
    /// two — not two with one at the floor.
    pub relevance: Option<serde_json::Value>,
}

/// Which CV an analysis should run against.
#[derive(Debug, Clone, Default)]
pub struct AnalyseApplicationInput {
    /// A living CV to analyse — how tailoring works, before anything is sent.
    ///
    /// When absent, the application's own snapshot is used. An application
    /// that has neither cannot be analysed, because there is nothing to read.
    pub cv_id: Option<Uuid>,
}

/// Why an analysis could not be produced.
#[derive(Debug, Clone, thiserror::Error)]
pub enum AnalysisError {
    /// No application matched, or it belongs to another user.
    #[error("Application not found")]
    ApplicationNotFound,

    /// No CV matched the one asked for.
    #[error("CV not found")]
    CvNotFound,

    /// The application is a draft with no CV named and no snapshot to fall
    /// back on.
    #[error("Nothing to analyse: send cv_id, or send the application first")]
    NoCvToAnalyse,

    /// The store could not be reached.
    #[error("Repository error: {0}")]
    RepositoryError(String),
}

/// Analyses a CV against the job an application is for.
#[async_trait]
pub trait AnalyseApplicationUseCase: Send + Sync {
    /// Runs the measurable checks and returns them.
    async fn execute(
        &self,
        owner: UserId,
        application_id: Uuid,
        input: AnalyseApplicationInput,
    ) -> Result<MatchAnalysis, AnalysisError>;
}
