//! Capturing, listing, editing and archiving postings.

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::career::application::ports::incoming::use_cases::{
    ArchiveJobUseCase, CreateJobUseCase, GetJobUseCase, GetJobsUseCase, JobError, PatchJobUseCase,
};
use crate::career::application::ports::outgoing::{CreateJobData, JobStore, PatchJobData};
use crate::career::domain::entities::Job;

/// A posting needs at least these two to be findable in a list.
fn validate(title: &str, company: &str) -> Result<(), JobError> {
    if title.trim().is_empty() {
        return Err(JobError::Invalid("A job needs a title".into()));
    }
    if company.trim().is_empty() {
        return Err(JobError::Invalid("A job needs a company".into()));
    }
    Ok(())
}

/// Implements the corresponding use-case contract.
pub struct JobService<S> {
    store: S,
}

impl<S> JobService<S> {
    /// Builds it from the ports it depends on.
    pub fn new(store: S) -> Self {
        Self { store }
    }
}

#[async_trait]
impl<S> CreateJobUseCase for JobService<S>
where
    S: JobStore + Send + Sync,
{
    async fn execute(&self, owner: UserId, mut data: CreateJobData) -> Result<Job, JobError> {
        validate(&data.title, &data.company)?;
        data.title = data.title.trim().to_string();
        data.company = data.company.trim().to_string();

        // source_text is stored exactly as given, including whitespace. It is
        // a record of what was published, not a field to tidy.
        Ok(self.store.create(owner.value(), data).await?)
    }
}

#[async_trait]
impl<S> GetJobsUseCase for JobService<S>
where
    S: JobStore + Send + Sync,
{
    async fn execute(&self, owner: UserId) -> Result<Vec<Job>, JobError> {
        Ok(self.store.list(owner.value()).await?)
    }
}

#[async_trait]
impl<S> GetJobUseCase for JobService<S>
where
    S: JobStore + Send + Sync,
{
    async fn execute(&self, owner: UserId, job_id: Uuid) -> Result<Job, JobError> {
        self.store
            .find(owner.value(), job_id)
            .await?
            .ok_or(JobError::NotFound)
    }
}

#[async_trait]
impl<S> PatchJobUseCase for JobService<S>
where
    S: JobStore + Send + Sync,
{
    async fn execute(
        &self,
        owner: UserId,
        job_id: Uuid,
        data: PatchJobData,
    ) -> Result<Job, JobError> {
        if let Some(title) = &data.title {
            validate(title, "placeholder")?;
        }
        if let Some(company) = &data.company {
            validate("placeholder", company)?;
        }

        // An empty patch would bump updated_at and report success for having
        // done nothing, so it is a read instead.
        if data.is_empty() {
            return self
                .store
                .find(owner.value(), job_id)
                .await?
                .ok_or(JobError::NotFound);
        }

        Ok(self.store.patch(owner.value(), job_id, data).await?)
    }
}

#[async_trait]
impl<S> ArchiveJobUseCase for JobService<S>
where
    S: JobStore + Send + Sync,
{
    async fn execute(&self, owner: UserId, job_id: Uuid) -> Result<(), JobError> {
        Ok(self.store.archive(owner.value(), job_id).await?)
    }
}
