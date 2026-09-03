//! The SeaORM implementation of [`CvSnapshotStore`].
//!
//! The insert reads the CV through an owner-scoped `SELECT`, so a CV belonging
//! to someone else yields no row, inserts nothing, and reports `CvNotFound`.
//! There is no separate ownership check to forget.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{DatabaseBackend, DatabaseConnection, FromQueryResult, Statement, Value};
use std::sync::Arc;
use uuid::Uuid;

use crate::cv::application::ports::outgoing::{CvSnapshot, CvSnapshotStore, CvSnapshotStoreError};
use crate::cv::domain::entities::CVInfo;

/// The SeaORM implementation of the matching outgoing port.
#[derive(Clone)]
pub struct CvSnapshotStorePostgres {
    db: Arc<DatabaseConnection>,
}

impl CvSnapshotStorePostgres {
    /// Builds it from the ports it depends on.
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[derive(Debug, FromQueryResult)]
struct SnapshotRow {
    id: Uuid,
    cv_id: Uuid,
    user_id: Uuid,
    document: serde_json::Value,
    created_at: DateTime<Utc>,
}

fn db_err(e: sea_orm::DbErr) -> CvSnapshotStoreError {
    CvSnapshotStoreError::DatabaseError(e.to_string())
}

impl TryFrom<SnapshotRow> for CvSnapshot {
    type Error = CvSnapshotStoreError;

    fn try_from(r: SnapshotRow) -> Result<Self, Self::Error> {
        // A snapshot that cannot be read back is a corrupt record, not an
        // empty one. Reporting it as missing would hide the problem behind a
        // 404 and leave the application silently unexplained.
        let document: CVInfo = serde_json::from_value(r.document)
            .map_err(|e| CvSnapshotStoreError::Corrupt(e.to_string()))?;

        Ok(CvSnapshot {
            id: r.id,
            cv_id: r.cv_id,
            user_id: r.user_id,
            document,
            created_at: r.created_at,
        })
    }
}

#[async_trait]
impl CvSnapshotStore for CvSnapshotStorePostgres {
    async fn create(&self, owner: Uuid, cv_id: Uuid) -> Result<CvSnapshot, CvSnapshotStoreError> {
        // The document is assembled in SQL from the CV's own columns rather
        // than read into Rust and written back. One statement, and no window
        // in which the CV could change between the read and the write.
        let sql = r#"
            INSERT INTO cv_snapshots (id, cv_id, user_id, document)
            SELECT $1, r.id, r.user_id, jsonb_build_object(
                       'id', r.id,
                       'user_id', r.user_id,
                       'role', r.role,
                       'display_name', r.display_name,
                       'bio', r.bio,
                       'photo_url', r.photo_url,
                       'core_skills', r.core_skills,
                       'educations', r.educations,
                       'experiences', r.experiences,
                       'highlighted_projects', r.highlighted_projects,
                       'contact_info', r.contact_info
                   )
              FROM resumes r
             WHERE r.id = $2
               AND r.user_id = $3
               AND r.is_deleted = false
         RETURNING id, cv_id, user_id, document, created_at
        "#;

        let row = SnapshotRow::find_by_statement(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            sql,
            [Value::from(Uuid::new_v4()), cv_id.into(), owner.into()],
        ))
        .one(self.db.as_ref())
        .await
        .map_err(db_err)?;

        row.ok_or(CvSnapshotStoreError::CvNotFound)?.try_into()
    }

    async fn find(
        &self,
        owner: Uuid,
        snapshot_id: Uuid,
    ) -> Result<Option<CvSnapshot>, CvSnapshotStoreError> {
        let sql = r#"
            SELECT id, cv_id, user_id, document, created_at
              FROM cv_snapshots
             WHERE id = $1
               AND user_id = $2
        "#;

        let row = SnapshotRow::find_by_statement(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            sql,
            [snapshot_id.into(), owner.into()],
        ))
        .one(self.db.as_ref())
        .await
        .map_err(db_err)?;

        row.map(CvSnapshot::try_from).transpose()
    }
}
