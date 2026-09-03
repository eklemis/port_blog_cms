use async_trait::async_trait;
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::blog::application::ports::outgoing::{BlogPostArchiver, BlogPostArchiverError};

/// The SeaORM implementation of the matching outgoing port.
#[derive(Clone)]
pub struct BlogPostArchiverPostgres {
    db: Arc<DatabaseConnection>,
}

impl BlogPostArchiverPostgres {
    /// Builds it from the ports it depends on.
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Runs an owner-scoped statement and turns "no rows touched" into
    /// NotFound, which covers both a missing post and one owned by someone
    /// else without distinguishing them.
    async fn exec_scoped(
        &self,
        sql: &str,
        owner: UserId,
        post_id: Uuid,
    ) -> Result<(), BlogPostArchiverError> {
        let result = self
            .db
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                sql,
                [post_id.into(), owner.value().into()],
            ))
            .await
            .map_err(|e| BlogPostArchiverError::DatabaseError(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(BlogPostArchiverError::NotFound);
        }

        Ok(())
    }
}

#[async_trait]
impl BlogPostArchiver for BlogPostArchiverPostgres {
    async fn unpublish(&self, owner: UserId, post_id: Uuid) -> Result<(), BlogPostArchiverError> {
        // No `AND published_at IS NOT NULL`: unpublishing a draft is a
        // success, not a miss, so the only reason to touch no rows is that the
        // post does not exist or is not the caller's.
        self.exec_scoped(
            r#"UPDATE blog_posts SET published_at = NULL, updated_at = NOW()
               WHERE id = $1 AND user_id = $2 AND is_deleted = false"#,
            owner,
            post_id,
        )
        .await
    }

    async fn soft_delete(&self, owner: UserId, post_id: Uuid) -> Result<(), BlogPostArchiverError> {
        self.exec_scoped(
            r#"UPDATE blog_posts SET is_deleted = true, updated_at = NOW()
               WHERE id = $1 AND user_id = $2 AND is_deleted = false"#,
            owner,
            post_id,
        )
        .await
    }

    async fn restore(&self, owner: UserId, post_id: Uuid) -> Result<(), BlogPostArchiverError> {
        self.exec_scoped(
            r#"UPDATE blog_posts SET is_deleted = false, updated_at = NOW()
               WHERE id = $1 AND user_id = $2 AND is_deleted = true"#,
            owner,
            post_id,
        )
        .await
    }

    async fn hard_delete(&self, owner: UserId, post_id: Uuid) -> Result<(), BlogPostArchiverError> {
        // blog_post_topics cascades on the foreign key, so links go with it.
        self.exec_scoped(
            r#"DELETE FROM blog_posts WHERE id = $1 AND user_id = $2"#,
            owner,
            post_id,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{DatabaseBackend, DbErr, MockDatabase, MockExecResult};

    fn db_with_rows(rows_affected: u64) -> DatabaseConnection {
        MockDatabase::new(DatabaseBackend::Postgres)
            .append_exec_results([MockExecResult {
                last_insert_id: 0,
                rows_affected,
            }])
            .into_connection()
    }

    fn db_with_error(msg: &str) -> DatabaseConnection {
        MockDatabase::new(DatabaseBackend::Postgres)
            .append_exec_errors([DbErr::Custom(msg.to_string())])
            .into_connection()
    }

    fn archiver(db: DatabaseConnection) -> BlogPostArchiverPostgres {
        BlogPostArchiverPostgres::new(Arc::new(db))
    }

    #[tokio::test]
    async fn soft_delete_succeeds_when_a_row_is_archived() {
        let a = archiver(db_with_rows(1));
        assert!(a
            .soft_delete(UserId::from(Uuid::new_v4()), Uuid::new_v4())
            .await
            .is_ok());
    }

    /// Every statement is owner-scoped, so touching no rows covers a missing
    /// post and one owned by someone else alike — the caller cannot tell them
    /// apart, which is deliberate.
    #[tokio::test]
    async fn soft_delete_reports_not_found_when_nothing_matches() {
        let a = archiver(db_with_rows(0));
        let err = a
            .soft_delete(UserId::from(Uuid::new_v4()), Uuid::new_v4())
            .await
            .unwrap_err();
        assert!(matches!(err, BlogPostArchiverError::NotFound));
    }

    #[tokio::test]
    async fn restore_succeeds_when_a_row_is_restored() {
        let a = archiver(db_with_rows(1));
        assert!(a
            .restore(UserId::from(Uuid::new_v4()), Uuid::new_v4())
            .await
            .is_ok());
    }

    /// `restore` carries `AND is_deleted = true`, so restoring a live post
    /// matches nothing. Reported as NotFound rather than silently succeeding.
    #[tokio::test]
    async fn restore_reports_not_found_for_a_post_that_is_not_archived() {
        let a = archiver(db_with_rows(0));
        let err = a
            .restore(UserId::from(Uuid::new_v4()), Uuid::new_v4())
            .await
            .unwrap_err();
        assert!(matches!(err, BlogPostArchiverError::NotFound));
    }

    #[tokio::test]
    async fn hard_delete_succeeds_when_a_row_is_removed() {
        let a = archiver(db_with_rows(1));
        assert!(a
            .hard_delete(UserId::from(Uuid::new_v4()), Uuid::new_v4())
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn hard_delete_reports_not_found_when_nothing_matches() {
        let a = archiver(db_with_rows(0));
        let err = a
            .hard_delete(UserId::from(Uuid::new_v4()), Uuid::new_v4())
            .await
            .unwrap_err();
        assert!(matches!(err, BlogPostArchiverError::NotFound));
    }

    #[tokio::test]
    async fn database_errors_are_surfaced() {
        let a = archiver(db_with_error("db down"));
        let err = a
            .soft_delete(UserId::from(Uuid::new_v4()), Uuid::new_v4())
            .await
            .unwrap_err();
        assert!(matches!(err, BlogPostArchiverError::DatabaseError(m) if m.contains("db down")));
    }
}
