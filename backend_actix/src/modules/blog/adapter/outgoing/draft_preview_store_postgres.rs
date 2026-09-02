//! The SeaORM implementation of [`DraftPreviewStore`].
//!
//! Every author-facing statement joins `blog_posts` on `user_id`, so a post
//! belonging to somebody else touches no rows and reports `PostNotFound`. That
//! join is the access control — there is no separate ownership read to forget.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{
    ConnectionTrait, DatabaseBackend, DatabaseConnection, FromQueryResult, Statement, Value,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::blog::application::ports::outgoing::{
    DraftPreview, DraftPreviewStore, DraftPreviewStoreError, LivePreview,
};

/// The SeaORM implementation of the matching outgoing port.
#[derive(Clone)]
pub struct DraftPreviewStorePostgres {
    db: Arc<DatabaseConnection>,
}

impl DraftPreviewStorePostgres {
    /// Builds it from the ports it depends on.
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[derive(Debug, FromQueryResult)]
struct PreviewRow {
    post_id: Uuid,
    token: String,
    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, FromQueryResult)]
struct LivePreviewRow {
    post_id: Uuid,
    token: String,
    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    user_id: Uuid,
}

fn db_err(e: sea_orm::DbErr) -> DraftPreviewStoreError {
    DraftPreviewStoreError::DatabaseError(e.to_string())
}

impl From<PreviewRow> for DraftPreview {
    fn from(r: PreviewRow) -> Self {
        DraftPreview {
            post_id: r.post_id,
            token: r.token,
            expires_at: r.expires_at,
            created_at: r.created_at,
        }
    }
}

#[async_trait]
impl DraftPreviewStore for DraftPreviewStorePostgres {
    async fn upsert(
        &self,
        owner: Uuid,
        post_id: Uuid,
        expires_at: DateTime<Utc>,
        new_token: &str,
    ) -> Result<DraftPreview, DraftPreviewStoreError> {
        // The SELECT in the source is what scopes this to the owner: it yields
        // no row for someone else's post, so the INSERT touches nothing and
        // RETURNING comes back empty.
        //
        // ON CONFLICT updates only `expires_at`, deliberately leaving `token`
        // and `created_at` alone — renewing must not change the link.
        let sql = r#"
            INSERT INTO blog_post_previews (post_id, token, expires_at)
            SELECT p.id, $2, $3
              FROM blog_posts p
             WHERE p.id = $1
               AND p.user_id = $4
               AND p.is_deleted = false
            ON CONFLICT (post_id) DO UPDATE
               SET expires_at = EXCLUDED.expires_at
         RETURNING post_id, token, expires_at, created_at
        "#;

        let row = PreviewRow::find_by_statement(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            sql,
            [
                post_id.into(),
                Value::from(new_token),
                expires_at.into(),
                owner.into(),
            ],
        ))
        .one(self.db.as_ref())
        .await
        .map_err(db_err)?;

        row.map(DraftPreview::from)
            .ok_or(DraftPreviewStoreError::PostNotFound)
    }

    async fn find_for_post(
        &self,
        owner: Uuid,
        post_id: Uuid,
    ) -> Result<Option<DraftPreview>, DraftPreviewStoreError> {
        // Expired rows are returned: the sharing panel distinguishes "expired,
        // renew it" from "never shared".
        let sql = r#"
            SELECT v.post_id, v.token, v.expires_at, v.created_at
              FROM blog_post_previews v
              JOIN blog_posts p ON p.id = v.post_id
             WHERE v.post_id = $1
               AND p.user_id = $2
        "#;

        let row = PreviewRow::find_by_statement(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            sql,
            [post_id.into(), owner.into()],
        ))
        .one(self.db.as_ref())
        .await
        .map_err(db_err)?;

        Ok(row.map(DraftPreview::from))
    }

    async fn revoke(&self, owner: Uuid, post_id: Uuid) -> Result<(), DraftPreviewStoreError> {
        let sql = r#"
            DELETE FROM blog_post_previews v
             USING blog_posts p
             WHERE v.post_id = $1
               AND p.id = v.post_id
               AND p.user_id = $2
        "#;

        // Deleting nothing is success: revoking a post that was never shared
        // leaves the author where they wanted to be.
        self.db
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                sql,
                [post_id.into(), owner.into()],
            ))
            .await
            .map_err(db_err)?;

        Ok(())
    }

    async fn find_live_by_token(
        &self,
        token: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<LivePreview>, DraftPreviewStoreError> {
        // Expiry is filtered in SQL so an expired token is indistinguishable
        // from one that never existed. Archived posts drop out too — an author
        // who archives a draft has withdrawn it.
        let sql = r#"
            SELECT v.post_id, v.token, v.expires_at, v.created_at, p.user_id
              FROM blog_post_previews v
              JOIN blog_posts p ON p.id = v.post_id
             WHERE v.token = $1
               AND v.expires_at > $2
               AND p.is_deleted = false
        "#;

        let row = LivePreviewRow::find_by_statement(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            sql,
            [Value::from(token), now.into()],
        ))
        .one(self.db.as_ref())
        .await
        .map_err(db_err)?;

        Ok(row.map(|r| LivePreview {
            owner_id: r.user_id,
            preview: DraftPreview {
                post_id: r.post_id,
                token: r.token,
                expires_at: r.expires_at,
                created_at: r.created_at,
            },
        }))
    }
}
