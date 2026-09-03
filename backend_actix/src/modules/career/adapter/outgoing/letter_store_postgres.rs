//! The SeaORM implementation of [`LetterStore`].
//!
//! Every statement joins `applications` on `user_id`, so a letter or reflection
//! on somebody else's application matches no row. That join is the access
//! control — there is no separate ownership read, and none to forget.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{
    ConnectionTrait, DatabaseBackend, DatabaseConnection, FromQueryResult, Statement, Value,
};
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

use crate::career::application::ports::outgoing::{
    LetterStore, LetterStoreError, PatchCoverLetterData, ReflectionData,
};
use crate::career::domain::entities::{CoverLetter, CoverLetterStatus, Reflection};

/// The SeaORM implementation of the matching outgoing port.
#[derive(Clone)]
pub struct LetterStorePostgres {
    db: Arc<DatabaseConnection>,
}

impl LetterStorePostgres {
    /// Builds it from the ports it depends on.
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

fn db_err(e: sea_orm::DbErr) -> LetterStoreError {
    LetterStoreError::DatabaseError(e.to_string())
}

#[derive(Debug, FromQueryResult)]
struct LetterRow {
    application_id: Uuid,
    content: String,
    language: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, FromQueryResult)]
struct ReflectionRow {
    application_id: Uuid,
    stage_reached: String,
    what_happened: String,
    what_id_change: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<LetterRow> for CoverLetter {
    fn from(r: LetterRow) -> Self {
        CoverLetter {
            application_id: r.application_id,
            content: r.content,
            language: r.language,
            // The CHECK constraint keeps this in the known set, so an
            // unparseable value means schema and enum have diverged. Draft is
            // the safe reading: it never claims a letter went out.
            status: CoverLetterStatus::from_str(&r.status).unwrap_or_default(),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

impl From<ReflectionRow> for Reflection {
    fn from(r: ReflectionRow) -> Self {
        Reflection {
            application_id: r.application_id,
            stage_reached: r.stage_reached,
            what_happened: r.what_happened,
            what_id_change: r.what_id_change,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[async_trait]
impl LetterStore for LetterStorePostgres {
    async fn find_letter(
        &self,
        owner: Uuid,
        application_id: Uuid,
    ) -> Result<Option<CoverLetter>, LetterStoreError> {
        let row = LetterRow::find_by_statement(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"
            SELECT l.application_id, l.content, l.language, l.status,
                   l.created_at, l.updated_at
              FROM cover_letters l
              JOIN applications a ON a.id = l.application_id
             WHERE l.application_id = $1
               AND a.user_id = $2
            "#,
            [application_id.into(), owner.into()],
        ))
        .one(self.db.as_ref())
        .await
        .map_err(db_err)?;

        Ok(row.map(Into::into))
    }

    async fn upsert_letter(
        &self,
        owner: Uuid,
        application_id: Uuid,
        data: PatchCoverLetterData,
    ) -> Result<CoverLetter, LetterStoreError> {
        // COALESCE on the update side is what makes this a patch: a field the
        // caller did not mention keeps whatever is stored. On insert the
        // parameters fall back to the column defaults instead.
        let row = LetterRow::find_by_statement(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"
            INSERT INTO cover_letters (application_id, user_id, content, language, status)
            SELECT a.id, a.user_id,
                   COALESCE($3, ''), COALESCE($4, 'en'), COALESCE($5, 'draft')
              FROM applications a
             WHERE a.id = $1
               AND a.user_id = $2
               AND a.is_deleted = false
            ON CONFLICT (application_id) DO UPDATE
               SET content    = COALESCE(EXCLUDED.content, cover_letters.content),
                   language   = COALESCE(EXCLUDED.language, cover_letters.language),
                   status     = COALESCE(EXCLUDED.status, cover_letters.status),
                   updated_at = now()
         RETURNING application_id, content, language, status, created_at, updated_at
            "#,
            [
                application_id.into(),
                owner.into(),
                data.content.map(Value::from).unwrap_or(Value::String(None)),
                data.language
                    .map(Value::from)
                    .unwrap_or(Value::String(None)),
                data.status
                    .map(|s| Value::from(s.to_string()))
                    .unwrap_or(Value::String(None)),
            ],
        ))
        .one(self.db.as_ref())
        .await
        .map_err(db_err)?;

        row.map(Into::into)
            .ok_or(LetterStoreError::ApplicationNotFound)
    }

    async fn delete_letter(
        &self,
        owner: Uuid,
        application_id: Uuid,
    ) -> Result<(), LetterStoreError> {
        self.db
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                r#"
                DELETE FROM cover_letters l
                 USING applications a
                 WHERE l.application_id = $1
                   AND a.id = l.application_id
                   AND a.user_id = $2
                "#,
                [application_id.into(), owner.into()],
            ))
            .await
            .map_err(db_err)?;

        Ok(())
    }

    async fn find_reflection(
        &self,
        owner: Uuid,
        application_id: Uuid,
    ) -> Result<Option<Reflection>, LetterStoreError> {
        let row = ReflectionRow::find_by_statement(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"
            SELECT r.application_id, r.stage_reached, r.what_happened,
                   r.what_id_change, r.created_at, r.updated_at
              FROM reflections r
              JOIN applications a ON a.id = r.application_id
             WHERE r.application_id = $1
               AND a.user_id = $2
            "#,
            [application_id.into(), owner.into()],
        ))
        .one(self.db.as_ref())
        .await
        .map_err(db_err)?;

        Ok(row.map(Into::into))
    }

    async fn upsert_reflection(
        &self,
        owner: Uuid,
        application_id: Uuid,
        data: ReflectionData,
    ) -> Result<Reflection, LetterStoreError> {
        // Written whole, unlike the letter: the three answers are given in one
        // sitting, so the update replaces rather than coalesces.
        let row = ReflectionRow::find_by_statement(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"
            INSERT INTO reflections (application_id, user_id, stage_reached,
                                     what_happened, what_id_change)
            SELECT a.id, a.user_id, $3, $4, $5
              FROM applications a
             WHERE a.id = $1
               AND a.user_id = $2
               AND a.is_deleted = false
            ON CONFLICT (application_id) DO UPDATE
               SET stage_reached  = EXCLUDED.stage_reached,
                   what_happened  = EXCLUDED.what_happened,
                   what_id_change = EXCLUDED.what_id_change,
                   updated_at     = now()
         RETURNING application_id, stage_reached, what_happened, what_id_change,
                   created_at, updated_at
            "#,
            [
                application_id.into(),
                owner.into(),
                Value::from(data.stage_reached),
                Value::from(data.what_happened),
                Value::from(data.what_id_change),
            ],
        ))
        .one(self.db.as_ref())
        .await
        .map_err(db_err)?;

        row.map(Into::into)
            .ok_or(LetterStoreError::ApplicationNotFound)
    }

    async fn delete_reflection(
        &self,
        owner: Uuid,
        application_id: Uuid,
    ) -> Result<(), LetterStoreError> {
        // A real DELETE, not a flag. Someone withdrawing a private note about
        // their own rejection should not later discover it was only hidden.
        self.db
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                r#"
                DELETE FROM reflections r
                 USING applications a
                 WHERE r.application_id = $1
                   AND a.id = r.application_id
                   AND a.user_id = $2
                "#,
                [application_id.into(), owner.into()],
            ))
            .await
            .map_err(db_err)?;

        Ok(())
    }
}
