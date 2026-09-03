//! The SeaORM implementation of [`ApplicationStore`].
//!
//! Owner-scoped in the same way as the job store: every statement carries
//! `user_id = $owner`.

use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder,
};
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

use crate::career::adapter::outgoing::sea_orm_entity::applications::{
    ActiveModel as AppActive, Column as AppColumn, Entity as AppEntity, Model as AppModel,
};
use crate::career::adapter::outgoing::sea_orm_entity::jobs::{
    Column as JobColumn, Entity as JobEntity,
};
use crate::career::application::ports::outgoing::{
    ApplicationStore, ApplicationStoreError, CreateApplicationData, PatchApplicationData,
};
use crate::career::domain::entities::{Application, ApplicationStatus};

/// The SeaORM implementation of the matching outgoing port.
#[derive(Clone)]
pub struct ApplicationStorePostgres {
    db: Arc<DatabaseConnection>,
}

impl ApplicationStorePostgres {
    /// Builds it from the ports it depends on.
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

fn db_err(e: sea_orm::DbErr) -> ApplicationStoreError {
    ApplicationStoreError::DatabaseError(e.to_string())
}

fn to_domain(m: AppModel) -> Application {
    Application {
        id: m.id,
        user_id: m.user_id,
        job_id: m.job_id,
        cv_snapshot_id: m.cv_snapshot_id,
        // The CHECK constraint keeps this in the known set, so an unparseable
        // value would mean the schema and the enum have diverged. Falling back
        // to Draft is the safest reading: it understates progress rather than
        // inventing it, and it cannot make an unsent application look sent.
        status: ApplicationStatus::from_str(&m.status).unwrap_or_default(),
        applied_at: m.applied_at.map(|t| t.with_timezone(&Utc)),
        next_action: m.next_action,
        next_action_at: m.next_action_at.map(|t| t.with_timezone(&Utc)),
        created_at: m.created_at.with_timezone(&Utc),
        updated_at: m.updated_at.with_timezone(&Utc),
    }
}

#[async_trait]
impl ApplicationStore for ApplicationStorePostgres {
    async fn create(
        &self,
        owner: Uuid,
        data: CreateApplicationData,
    ) -> Result<Application, ApplicationStoreError> {
        // Checked here rather than left to the foreign key, so applying to
        // someone else's posting is JobNotFound instead of a constraint
        // violation surfacing as a 500.
        let job_exists = JobEntity::find_by_id(data.job_id)
            .filter(JobColumn::UserId.eq(owner))
            .filter(JobColumn::IsDeleted.eq(false))
            .one(self.db.as_ref())
            .await
            .map_err(db_err)?
            .is_some();

        if !job_exists {
            return Err(ApplicationStoreError::JobNotFound);
        }

        let model = AppActive {
            id: Set(Uuid::new_v4()),
            user_id: Set(owner),
            job_id: Set(data.job_id),
            cv_snapshot_id: Set(None),
            status: Set(ApplicationStatus::Draft.to_string()),
            applied_at: Set(None),
            next_action: Set(data.next_action),
            next_action_at: Set(data.next_action_at.map(Into::into)),
            is_deleted: Set(false),
            created_at: Set(Utc::now().into()),
            updated_at: Set(Utc::now().into()),
        };

        let stored = model.insert(self.db.as_ref()).await.map_err(db_err)?;
        Ok(to_domain(stored))
    }

    async fn list(&self, owner: Uuid) -> Result<Vec<Application>, ApplicationStoreError> {
        let rows = AppEntity::find()
            .filter(AppColumn::UserId.eq(owner))
            .filter(AppColumn::IsDeleted.eq(false))
            .order_by_desc(AppColumn::CreatedAt)
            .all(self.db.as_ref())
            .await
            .map_err(db_err)?;

        Ok(rows.into_iter().map(to_domain).collect())
    }

    async fn find(
        &self,
        owner: Uuid,
        application_id: Uuid,
    ) -> Result<Option<Application>, ApplicationStoreError> {
        let row = AppEntity::find_by_id(application_id)
            .filter(AppColumn::UserId.eq(owner))
            .filter(AppColumn::IsDeleted.eq(false))
            .one(self.db.as_ref())
            .await
            .map_err(db_err)?;

        Ok(row.map(to_domain))
    }

    async fn patch(
        &self,
        owner: Uuid,
        application_id: Uuid,
        data: PatchApplicationData,
    ) -> Result<Application, ApplicationStoreError> {
        let row = AppEntity::find_by_id(application_id)
            .filter(AppColumn::UserId.eq(owner))
            .filter(AppColumn::IsDeleted.eq(false))
            .one(self.db.as_ref())
            .await
            .map_err(db_err)?
            .ok_or(ApplicationStoreError::NotFound)?;

        let mut active: AppActive = row.into();

        if let Some(v) = data.status {
            active.status = Set(v.to_string());
        }
        if let Some(v) = data.cv_snapshot_id {
            active.cv_snapshot_id = Set(Some(v));
        }
        if let Some(v) = data.applied_at {
            active.applied_at = Set(Some(v.into()));
        }
        if let Some(v) = data.next_action {
            active.next_action = Set(v);
        }
        if let Some(v) = data.next_action_at {
            active.next_action_at = Set(v.map(Into::into));
        }
        active.updated_at = Set(Utc::now().into());

        let stored = active.update(self.db.as_ref()).await.map_err(db_err)?;
        Ok(to_domain(stored))
    }

    async fn archive(
        &self,
        owner: Uuid,
        application_id: Uuid,
    ) -> Result<(), ApplicationStoreError> {
        let row = AppEntity::find_by_id(application_id)
            .filter(AppColumn::UserId.eq(owner))
            .filter(AppColumn::IsDeleted.eq(false))
            .one(self.db.as_ref())
            .await
            .map_err(db_err)?
            .ok_or(ApplicationStoreError::NotFound)?;

        let mut active: AppActive = row.into();
        active.is_deleted = Set(true);
        active.updated_at = Set(Utc::now().into());
        active.update(self.db.as_ref()).await.map_err(db_err)?;

        Ok(())
    }
}
