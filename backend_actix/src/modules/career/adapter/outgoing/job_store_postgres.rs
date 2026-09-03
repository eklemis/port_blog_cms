//! The SeaORM implementation of [`JobStore`].
//!
//! Every statement carries `user_id = $owner`, so a posting belonging to
//! somebody else matches no row and reports `NotFound`. That clause is the
//! access control — there is no separate ownership read.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::career::adapter::outgoing::sea_orm_entity::jobs::{
    ActiveModel as JobActive, Column as JobColumn, Entity as JobEntity, Model as JobModel,
};
use crate::career::application::ports::outgoing::{
    CreateJobData, JobStore, JobStoreError, PatchJobData,
};
use crate::career::domain::entities::Job;

/// The SeaORM implementation of the matching outgoing port.
#[derive(Clone)]
pub struct JobStorePostgres {
    db: Arc<DatabaseConnection>,
}

impl JobStorePostgres {
    /// Builds it from the ports it depends on.
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

fn db_err(e: sea_orm::DbErr) -> JobStoreError {
    JobStoreError::DatabaseError(e.to_string())
}

/// A malformed skills array is read as empty rather than failing the whole
/// row: the posting's own text is the record that matters, and the extracted
/// lists can be re-derived.
fn strings(value: &serde_json::Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn to_domain(m: JobModel) -> Job {
    Job {
        id: m.id,
        user_id: m.user_id,
        title: m.title,
        company: m.company,
        location: m.location,
        seniority: m.seniority,
        required_skills: strings(&m.required_skills),
        nice_to_have: strings(&m.nice_to_have),
        source_url: m.source_url,
        source_text: m.source_text,
        created_at: m.created_at.with_timezone(&Utc),
        updated_at: m.updated_at.with_timezone(&Utc),
    }
}

fn now() -> DateTime<Utc> {
    Utc::now()
}

#[async_trait]
impl JobStore for JobStorePostgres {
    async fn create(&self, owner: Uuid, data: CreateJobData) -> Result<Job, JobStoreError> {
        let model = JobActive {
            id: Set(Uuid::new_v4()),
            user_id: Set(owner),
            title: Set(data.title),
            company: Set(data.company),
            location: Set(data.location),
            seniority: Set(data.seniority),
            required_skills: Set(serde_json::json!(data.required_skills)),
            nice_to_have: Set(serde_json::json!(data.nice_to_have)),
            source_url: Set(data.source_url),
            source_text: Set(data.source_text),
            is_deleted: Set(false),
            created_at: Set(now().into()),
            updated_at: Set(now().into()),
        };

        let stored = model.insert(self.db.as_ref()).await.map_err(db_err)?;
        Ok(to_domain(stored))
    }

    async fn list(&self, owner: Uuid) -> Result<Vec<Job>, JobStoreError> {
        let rows = JobEntity::find()
            .filter(JobColumn::UserId.eq(owner))
            .filter(JobColumn::IsDeleted.eq(false))
            .order_by_desc(JobColumn::CreatedAt)
            .all(self.db.as_ref())
            .await
            .map_err(db_err)?;

        Ok(rows.into_iter().map(to_domain).collect())
    }

    async fn find(&self, owner: Uuid, job_id: Uuid) -> Result<Option<Job>, JobStoreError> {
        let row = JobEntity::find_by_id(job_id)
            .filter(JobColumn::UserId.eq(owner))
            .filter(JobColumn::IsDeleted.eq(false))
            .one(self.db.as_ref())
            .await
            .map_err(db_err)?;

        Ok(row.map(to_domain))
    }

    async fn patch(
        &self,
        owner: Uuid,
        job_id: Uuid,
        data: PatchJobData,
    ) -> Result<Job, JobStoreError> {
        let row = JobEntity::find_by_id(job_id)
            .filter(JobColumn::UserId.eq(owner))
            .filter(JobColumn::IsDeleted.eq(false))
            .one(self.db.as_ref())
            .await
            .map_err(db_err)?
            .ok_or(JobStoreError::NotFound)?;

        let mut active: JobActive = row.into();

        if let Some(v) = data.title {
            active.title = Set(v);
        }
        if let Some(v) = data.company {
            active.company = Set(v);
        }
        if let Some(v) = data.location {
            active.location = Set(v);
        }
        if let Some(v) = data.seniority {
            active.seniority = Set(v);
        }
        if let Some(v) = data.required_skills {
            active.required_skills = Set(serde_json::json!(v));
        }
        if let Some(v) = data.nice_to_have {
            active.nice_to_have = Set(serde_json::json!(v));
        }
        if let Some(v) = data.source_url {
            active.source_url = Set(v);
        }
        if let Some(v) = data.source_text {
            active.source_text = Set(v);
        }
        active.updated_at = Set(now().into());

        let stored = active.update(self.db.as_ref()).await.map_err(db_err)?;
        Ok(to_domain(stored))
    }

    async fn archive(&self, owner: Uuid, job_id: Uuid) -> Result<(), JobStoreError> {
        let row = JobEntity::find_by_id(job_id)
            .filter(JobColumn::UserId.eq(owner))
            .filter(JobColumn::IsDeleted.eq(false))
            .one(self.db.as_ref())
            .await
            .map_err(db_err)?
            .ok_or(JobStoreError::NotFound)?;

        let mut active: JobActive = row.into();
        active.is_deleted = Set(true);
        active.updated_at = Set(now().into());
        active.update(self.db.as_ref()).await.map_err(db_err)?;

        Ok(())
    }
}
