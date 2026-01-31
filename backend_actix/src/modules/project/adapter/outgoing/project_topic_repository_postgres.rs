use async_trait::async_trait;
use sea_orm::{
    ConnectionTrait, DatabaseBackend, DatabaseConnection, DbErr, Statement, TransactionTrait,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::modules::project::application::ports::outgoing::project_topic_repository::{
    ProjectTopicRepository, ProjectTopicRepositoryError,
};

#[derive(Clone)]
pub struct ProjectTopicRepositoryPostgres {
    db: Arc<DatabaseConnection>,
}

impl ProjectTopicRepositoryPostgres {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    // =====================================================
    // SQL builders
    // =====================================================

    /// Guarded, idempotent insert:
    /// - project must exist, belong to owner, and not be deleted
    /// - topic must exist and belong to owner
    /// - on conflict (project_id, topic_id) do nothing
    fn guarded_insert_stmt(owner: Uuid, project_id: Uuid, topic_id: Uuid) -> Statement {
        Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"
            INSERT INTO project_topics (project_id, topic_id)
            SELECT p.id, t.id
            FROM projects p
            JOIN topics t
              ON t.id = $3
             AND t.user_id = $1
            WHERE p.id = $2
              AND p.user_id = $1
              AND p.is_deleted = false
            ON CONFLICT (project_id, topic_id) DO NOTHING
            "#,
            vec![owner.into(), project_id.into(), topic_id.into()],
        )
    }

    /// Deterministic probe:
    /// - project_ok: project exists, owned, not deleted
    /// - topic_ok: topic exists, owned
    /// - link_exists: link already exists (idempotent success)
    fn probe_stmt(owner: Uuid, project_id: Uuid, topic_id: Uuid) -> Statement {
        Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"
            SELECT
              EXISTS (
                SELECT 1
                FROM projects p
                WHERE p.id = $2
                  AND p.user_id = $1
                  AND p.is_deleted = false
              ) AS project_ok,
              EXISTS (
                SELECT 1
                FROM topics t
                WHERE t.id = $3
                  AND t.user_id = $1
              ) AS topic_ok,
              EXISTS (
                SELECT 1
                FROM project_topics pt
                WHERE pt.project_id = $2
                  AND pt.topic_id = $3
              ) AS link_exists
            "#,
            vec![owner.into(), project_id.into(), topic_id.into()],
        )
    }

    fn map_db_err(e: DbErr) -> ProjectTopicRepositoryError {
        ProjectTopicRepositoryError::DatabaseError(e.to_string())
    }

    /// Resolve why guarded insert affected 0 rows.
    /// Priority rule (as you requested):
    /// 1) TopicNotFound
    /// 2) ProjectNotFound (means: not found OR not owner OR deleted)
    /// 3) link exists => idempotent Ok
    async fn resolve_insert_failure<C>(
        conn: &C,
        owner: Uuid,
        project_id: Uuid,
        topic_id: Uuid,
    ) -> Result<(), ProjectTopicRepositoryError>
    where
        C: ConnectionTrait,
    {
        let row = conn
            .query_one(Self::probe_stmt(owner, project_id, topic_id))
            .await
            .map_err(Self::map_db_err)?
            .ok_or_else(|| {
                ProjectTopicRepositoryError::DatabaseError(
                    "Probe query returned no rows".to_string(),
                )
            })?;

        let project_ok: bool = row.try_get("", "project_ok").unwrap_or(false);
        let topic_ok: bool = row.try_get("", "topic_ok").unwrap_or(false);
        let link_exists: bool = row.try_get("", "link_exists").unwrap_or(false);

        if !topic_ok {
            return Err(ProjectTopicRepositoryError::TopicNotFound);
        }
        if !project_ok {
            return Err(ProjectTopicRepositoryError::ProjectNotFound);
        }
        if link_exists {
            return Ok(()); // idempotent success
        }

        Err(ProjectTopicRepositoryError::DatabaseError(
            "Unexpected insert resolution state".to_string(),
        ))
    }

    /// Ensure project is valid for owner (exists, owned, not deleted).
    /// Used where we must return ProjectNotFound even if no rows were affected by delete.
    async fn ensure_project_ok<C>(
        conn: &C,
        owner: Uuid,
        project_id: Uuid,
    ) -> Result<(), ProjectTopicRepositoryError>
    where
        C: ConnectionTrait,
    {
        let stmt = Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM projects p
                WHERE p.id = $2
                  AND p.user_id = $1
                  AND p.is_deleted = false
            ) AS project_ok
            "#,
            vec![owner.into(), project_id.into()],
        );

        let row = conn
            .query_one(stmt)
            .await
            .map_err(Self::map_db_err)?
            .ok_or_else(|| {
                ProjectTopicRepositoryError::DatabaseError(
                    "Project existence probe returned no rows".to_string(),
                )
            })?;

        let project_ok: bool = row.try_get("", "project_ok").unwrap_or(false);
        if !project_ok {
            return Err(ProjectTopicRepositoryError::ProjectNotFound);
        }
        Ok(())
    }
}

#[async_trait]
impl ProjectTopicRepository for ProjectTopicRepositoryPostgres {
    async fn add_project_topic(
        &self,
        owner: UserId,
        project_id: Uuid,
        topic_id: Uuid,
    ) -> Result<(), ProjectTopicRepositoryError> {
        let owner_uuid: Uuid = owner.into();

        let result = self
            .db
            .execute(Self::guarded_insert_stmt(owner_uuid, project_id, topic_id))
            .await
            .map_err(Self::map_db_err)?;

        if result.rows_affected() == 1 {
            return Ok(());
        }

        // 0 affected => either precondition failed or link already existed
        Self::resolve_insert_failure(&*self.db, owner_uuid, project_id, topic_id).await
    }

    async fn remove_project_topic(
        &self,
        owner: UserId,
        project_id: Uuid,
        topic_id: Uuid,
    ) -> Result<(), ProjectTopicRepositoryError> {
        let owner_uuid: Uuid = owner.into();

        // Delete the link directly (idempotent). We can't infer project validity from rows_affected,
        // because "link didn't exist" also yields 0. So we delete first, then validate project.
        let delete_stmt = Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"
            DELETE FROM project_topics
            WHERE project_id = $2
              AND topic_id = $3
            "#,
            vec![owner_uuid.into(), project_id.into(), topic_id.into()],
        );

        self.db
            .execute(delete_stmt)
            .await
            .map_err(Self::map_db_err)?;

        // Enforce correct domain error if project invalid/not owned/deleted.
        // If project is OK, removal is idempotent success.
        Self::ensure_project_ok(&*self.db, owner_uuid, project_id).await?;

        Ok(())
    }

    async fn clear_project_topics(
        &self,
        owner: UserId,
        project_id: Uuid,
    ) -> Result<(), ProjectTopicRepositoryError> {
        let owner_uuid: Uuid = owner.into();

        let delete_stmt = Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"
            DELETE FROM project_topics
            WHERE project_id = $2
            "#,
            vec![owner_uuid.into(), project_id.into()],
        );

        self.db
            .execute(delete_stmt)
            .await
            .map_err(Self::map_db_err)?;

        // If project is invalid/not owned/deleted => ProjectNotFound.
        // Otherwise idempotent success (even if there were no links).
        Self::ensure_project_ok(&*self.db, owner_uuid, project_id).await?;

        Ok(())
    }

    async fn set_project_topics(
        &self,
        owner: UserId,
        project_id: Uuid,
        topic_ids: Vec<Uuid>,
    ) -> Result<(), ProjectTopicRepositoryError> {
        let owner_uuid: Uuid = owner.into();
        let txn = self.db.begin().await.map_err(Self::map_db_err)?;

        // First validate the project (cheap and avoids ambiguous “empty” sets).
        if let Err(e) = Self::ensure_project_ok(&txn, owner_uuid, project_id).await {
            let _ = txn.rollback().await;
            return Err(e);
        }

        // Clear existing links
        let clear_stmt = Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"
            DELETE FROM project_topics
            WHERE project_id = $2
            "#,
            vec![owner_uuid.into(), project_id.into()],
        );

        if let Err(e) = txn.execute(clear_stmt).await {
            let _ = txn.rollback().await;
            return Err(Self::map_db_err(e));
        }

        // Insert new links
        for topic_id in topic_ids {
            let insert_result = match txn
                .execute(Self::guarded_insert_stmt(owner_uuid, project_id, topic_id))
                .await
            {
                Ok(res) => res,
                Err(e) => {
                    let _ = txn.rollback().await;
                    return Err(Self::map_db_err(e));
                }
            };

            if insert_result.rows_affected() == 1 {
                continue;
            }

            // 0 affected => resolve deterministically within the txn
            match Self::resolve_insert_failure(&txn, owner_uuid, project_id, topic_id).await {
                Ok(()) => {} // link existed => idempotent
                Err(err) => {
                    let _ = txn.rollback().await;
                    return Err(err);
                }
            }
        }

        if let Err(e) = txn.commit().await {
            return Err(Self::map_db_err(e));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::sea_query::Value;
    use sea_orm::{DatabaseBackend, DbErr, MockDatabase, MockExecResult};
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use uuid::Uuid;

    fn ok_exec(rows: u64) -> MockExecResult {
        MockExecResult {
            last_insert_id: 0,
            rows_affected: rows,
        }
    }

    fn row_bool(col: &str, v: bool) -> BTreeMap<String, Value> {
        BTreeMap::from([(col.to_string(), Value::Bool(Some(v)))])
    }

    fn probe_row(project_ok: bool, topic_ok: bool, link_exists: bool) -> BTreeMap<String, Value> {
        BTreeMap::from([
            ("project_ok".to_string(), Value::Bool(Some(project_ok))),
            ("topic_ok".to_string(), Value::Bool(Some(topic_ok))),
            ("link_exists".to_string(), Value::Bool(Some(link_exists))),
        ])
    }

    // =====================================================
    // add_project_topic
    // =====================================================

    #[tokio::test]
    async fn test_add_project_topic_success() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            // guarded insert succeeds
            .append_exec_results([ok_exec(1)])
            .into_connection();

        let repo = ProjectTopicRepositoryPostgres::new(Arc::new(db));

        let res = repo
            .add_project_topic(UserId::from(Uuid::new_v4()), Uuid::new_v4(), Uuid::new_v4())
            .await;

        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_add_project_topic_idempotent_existing_link() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            // guarded insert -> 0
            .append_exec_results([ok_exec(0)])
            // probe -> link exists
            .append_query_results(vec![vec![probe_row(true, true, true)]])
            .into_connection();

        let repo = ProjectTopicRepositoryPostgres::new(Arc::new(db));

        let res = repo
            .add_project_topic(UserId::from(Uuid::new_v4()), Uuid::new_v4(), Uuid::new_v4())
            .await;

        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_add_project_topic_topic_not_found() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_exec_results([ok_exec(0)])
            // topic missing has priority
            .append_query_results(vec![vec![probe_row(true, false, false)]])
            .into_connection();

        let repo = ProjectTopicRepositoryPostgres::new(Arc::new(db));

        let res = repo
            .add_project_topic(UserId::from(Uuid::new_v4()), Uuid::new_v4(), Uuid::new_v4())
            .await;

        assert!(matches!(
            res.unwrap_err(),
            ProjectTopicRepositoryError::TopicNotFound
        ));
    }

    #[tokio::test]
    async fn test_add_project_topic_project_not_found() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_exec_results([ok_exec(0)])
            .append_query_results(vec![vec![probe_row(false, true, false)]])
            .into_connection();

        let repo = ProjectTopicRepositoryPostgres::new(Arc::new(db));

        let res = repo
            .add_project_topic(UserId::from(Uuid::new_v4()), Uuid::new_v4(), Uuid::new_v4())
            .await;

        assert!(matches!(
            res.unwrap_err(),
            ProjectTopicRepositoryError::ProjectNotFound
        ));
    }

    #[tokio::test]
    async fn test_add_project_topic_database_error() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_exec_errors([DbErr::Custom("connection error".to_string())])
            .into_connection();

        let repo = ProjectTopicRepositoryPostgres::new(Arc::new(db));

        let res = repo
            .add_project_topic(UserId::from(Uuid::new_v4()), Uuid::new_v4(), Uuid::new_v4())
            .await;

        assert!(matches!(
            res.unwrap_err(),
            ProjectTopicRepositoryError::DatabaseError(_)
        ));
    }

    // =====================================================
    // remove_project_topic
    // =====================================================

    #[tokio::test]
    async fn test_remove_project_topic_success() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            // delete link (can be 0 or 1; idempotent)
            .append_exec_results([ok_exec(1)])
            // ensure_project_ok query
            .append_query_results(vec![vec![row_bool("project_ok", true)]])
            .into_connection();

        let repo = ProjectTopicRepositoryPostgres::new(Arc::new(db));

        let res = repo
            .remove_project_topic(UserId::from(Uuid::new_v4()), Uuid::new_v4(), Uuid::new_v4())
            .await;

        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_remove_project_topic_project_not_found() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_exec_results([ok_exec(0)])
            .append_query_results(vec![vec![row_bool("project_ok", false)]])
            .into_connection();

        let repo = ProjectTopicRepositoryPostgres::new(Arc::new(db));

        let res = repo
            .remove_project_topic(UserId::from(Uuid::new_v4()), Uuid::new_v4(), Uuid::new_v4())
            .await;

        assert!(matches!(
            res.unwrap_err(),
            ProjectTopicRepositoryError::ProjectNotFound
        ));
    }

    #[tokio::test]
    async fn test_remove_project_topic_database_error_on_delete() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_exec_errors([DbErr::Custom("delete failed".to_string())])
            .into_connection();

        let repo = ProjectTopicRepositoryPostgres::new(Arc::new(db));

        let res = repo
            .remove_project_topic(UserId::from(Uuid::new_v4()), Uuid::new_v4(), Uuid::new_v4())
            .await;

        assert!(matches!(
            res.unwrap_err(),
            ProjectTopicRepositoryError::DatabaseError(_)
        ));
    }

    // =====================================================
    // clear_project_topics
    // =====================================================

    #[tokio::test]
    async fn test_clear_project_topics_success() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_exec_results([ok_exec(2)])
            .append_query_results(vec![vec![row_bool("project_ok", true)]])
            .into_connection();

        let repo = ProjectTopicRepositoryPostgres::new(Arc::new(db));

        let res = repo
            .clear_project_topics(UserId::from(Uuid::new_v4()), Uuid::new_v4())
            .await;

        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_clear_project_topics_project_not_found() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_exec_results([ok_exec(0)])
            .append_query_results(vec![vec![row_bool("project_ok", false)]])
            .into_connection();

        let repo = ProjectTopicRepositoryPostgres::new(Arc::new(db));

        let res = repo
            .clear_project_topics(UserId::from(Uuid::new_v4()), Uuid::new_v4())
            .await;

        assert!(matches!(
            res.unwrap_err(),
            ProjectTopicRepositoryError::ProjectNotFound
        ));
    }

    // =====================================================
    // set_project_topics
    // =====================================================

    #[tokio::test]
    async fn test_set_project_topics_success() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            // ensure_project_ok (inside txn)
            .append_query_results(vec![vec![row_bool("project_ok", true)]])
            // clear
            .append_exec_results([ok_exec(1)])
            // insert topic 1
            .append_exec_results([ok_exec(1)])
            // insert topic 2
            .append_exec_results([ok_exec(1)])
            // commit (transaction)
            .append_exec_results([ok_exec(0)])
            .into_connection();

        let repo = ProjectTopicRepositoryPostgres::new(Arc::new(db));

        let res = repo
            .set_project_topics(
                UserId::from(Uuid::new_v4()),
                Uuid::new_v4(),
                vec![Uuid::new_v4(), Uuid::new_v4()],
            )
            .await;

        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_set_project_topics_project_not_found() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            // ensure_project_ok false -> rollback
            .append_query_results(vec![vec![row_bool("project_ok", false)]])
            // rollback (transaction)
            .append_exec_results([ok_exec(0)])
            .into_connection();

        let repo = ProjectTopicRepositoryPostgres::new(Arc::new(db));

        let res = repo
            .set_project_topics(
                UserId::from(Uuid::new_v4()),
                Uuid::new_v4(),
                vec![Uuid::new_v4()],
            )
            .await;

        assert!(matches!(
            res.unwrap_err(),
            ProjectTopicRepositoryError::ProjectNotFound
        ));
    }

    #[tokio::test]
    async fn test_set_project_topics_topic_not_found_triggers_rollback() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            // ensure_project_ok true
            .append_query_results(vec![vec![row_bool("project_ok", true)]])
            // clear
            .append_exec_results([ok_exec(1)])
            // guarded insert -> 0
            .append_exec_results([ok_exec(0)])
            // probe -> topic missing
            .append_query_results(vec![vec![probe_row(true, false, false)]])
            // rollback (transaction)
            .append_exec_results([ok_exec(0)])
            .into_connection();

        let repo = ProjectTopicRepositoryPostgres::new(Arc::new(db));

        let res = repo
            .set_project_topics(
                UserId::from(Uuid::new_v4()),
                Uuid::new_v4(),
                vec![Uuid::new_v4()],
            )
            .await;

        assert!(matches!(
            res.unwrap_err(),
            ProjectTopicRepositoryError::TopicNotFound
        ));
    }
}
