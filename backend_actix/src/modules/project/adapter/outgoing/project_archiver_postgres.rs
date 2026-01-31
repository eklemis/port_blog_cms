// src/modules/project/adapter/outgoing/project_archiver_postgres.rs

use async_trait::async_trait;
use sea_orm::{sea_query::Expr, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter};
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::modules::project::adapter::outgoing::sea_orm_entity::projects::{Column, Entity};
use crate::modules::project::application::ports::outgoing::project_archiver::{
    ProjectArchiver, ProjectArchiverError,
};

#[derive(Clone)]
pub struct ProjectArchiverPostgres {
    db: Arc<DatabaseConnection>,
}

impl ProjectArchiverPostgres {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ProjectArchiver for ProjectArchiverPostgres {
    async fn soft_delete(
        &self,
        owner: UserId,
        project_id: Uuid,
    ) -> Result<(), ProjectArchiverError> {
        let owner_uuid: Uuid = owner.into();

        let res = Entity::update_many()
            .col_expr(Column::IsDeleted, Expr::value(true))
            .filter(Column::Id.eq(project_id))
            .filter(Column::UserId.eq(owner_uuid))
            .filter(Column::IsDeleted.eq(false)) // state-aware
            .exec(&*self.db)
            .await
            .map_err(map_db_err)?;

        if res.rows_affected == 0 {
            return Err(ProjectArchiverError::NotFound);
        }

        Ok(())
    }

    async fn hard_delete(
        &self,
        owner: UserId,
        project_id: Uuid,
    ) -> Result<(), ProjectArchiverError> {
        let owner_uuid: Uuid = owner.into();

        let res = Entity::delete_many()
            .filter(Column::Id.eq(project_id))
            .filter(Column::UserId.eq(owner_uuid))
            .exec(&*self.db)
            .await
            .map_err(map_db_err)?;

        if res.rows_affected == 0 {
            return Err(ProjectArchiverError::NotFound);
        }

        Ok(())
    }

    async fn restore(&self, owner: UserId, project_id: Uuid) -> Result<(), ProjectArchiverError> {
        let owner_uuid: Uuid = owner.into();

        let res = Entity::update_many()
            .col_expr(Column::IsDeleted, Expr::value(false))
            .filter(Column::Id.eq(project_id))
            .filter(Column::UserId.eq(owner_uuid))
            .filter(Column::IsDeleted.eq(true)) // state-aware
            .exec(&*self.db)
            .await
            .map_err(map_db_err)?;

        if res.rows_affected == 0 {
            return Err(ProjectArchiverError::NotFound);
        }

        Ok(())
    }
}

fn map_db_err(e: DbErr) -> ProjectArchiverError {
    ProjectArchiverError::DatabaseError(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{DatabaseBackend, DbErr, MockDatabase, MockExecResult};
    use std::sync::Arc;
    use uuid::Uuid;

    fn mock_db_with_exec(rows_affected: u64) -> sea_orm::DatabaseConnection {
        MockDatabase::new(DatabaseBackend::Postgres)
            .append_exec_results([MockExecResult {
                last_insert_id: 0,
                rows_affected,
            }])
            .into_connection()
    }

    fn mock_db_with_exec_error(msg: &str) -> sea_orm::DatabaseConnection {
        MockDatabase::new(DatabaseBackend::Postgres)
            .append_exec_errors([DbErr::Custom(msg.to_string())])
            .into_connection()
    }

    // =========================================================================
    // soft_delete
    // =========================================================================

    #[tokio::test]
    async fn test_soft_delete_success() {
        let owner = UserId::from(Uuid::new_v4());
        let project_id = Uuid::new_v4();

        let db = mock_db_with_exec(1);
        let archiver = ProjectArchiverPostgres::new(Arc::new(db));

        let res = archiver.soft_delete(owner, project_id).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_soft_delete_not_found() {
        let owner = UserId::from(Uuid::new_v4());
        let project_id = Uuid::new_v4();

        let db = mock_db_with_exec(0);
        let archiver = ProjectArchiverPostgres::new(Arc::new(db));

        let res = archiver.soft_delete(owner, project_id).await;
        assert!(matches!(res.unwrap_err(), ProjectArchiverError::NotFound));
    }

    #[tokio::test]
    async fn test_soft_delete_database_error() {
        let owner = UserId::from(Uuid::new_v4());
        let project_id = Uuid::new_v4();

        let db = mock_db_with_exec_error("connection error");
        let archiver = ProjectArchiverPostgres::new(Arc::new(db));

        let res = archiver.soft_delete(owner, project_id).await;
        assert!(matches!(
            res.unwrap_err(),
            ProjectArchiverError::DatabaseError(_)
        ));
    }

    // =========================================================================
    // restore
    // =========================================================================

    #[tokio::test]
    async fn test_restore_success() {
        let owner = UserId::from(Uuid::new_v4());
        let project_id = Uuid::new_v4();

        let db = mock_db_with_exec(1);
        let archiver = ProjectArchiverPostgres::new(Arc::new(db));

        let res = archiver.restore(owner, project_id).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_restore_not_found() {
        let owner = UserId::from(Uuid::new_v4());
        let project_id = Uuid::new_v4();

        let db = mock_db_with_exec(0);
        let archiver = ProjectArchiverPostgres::new(Arc::new(db));

        let res = archiver.restore(owner, project_id).await;
        assert!(matches!(res.unwrap_err(), ProjectArchiverError::NotFound));
    }

    #[tokio::test]
    async fn test_restore_database_error() {
        let owner = UserId::from(Uuid::new_v4());
        let project_id = Uuid::new_v4();

        let db = mock_db_with_exec_error("connection error");
        let archiver = ProjectArchiverPostgres::new(Arc::new(db));

        let res = archiver.restore(owner, project_id).await;
        assert!(matches!(
            res.unwrap_err(),
            ProjectArchiverError::DatabaseError(_)
        ));
    }

    // =========================================================================
    // hard_delete
    // =========================================================================

    #[tokio::test]
    async fn test_hard_delete_success() {
        let owner = UserId::from(Uuid::new_v4());
        let project_id = Uuid::new_v4();

        let db = mock_db_with_exec(1);
        let archiver = ProjectArchiverPostgres::new(Arc::new(db));

        let res = archiver.hard_delete(owner, project_id).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_hard_delete_not_found() {
        let owner = UserId::from(Uuid::new_v4());
        let project_id = Uuid::new_v4();

        let db = mock_db_with_exec(0);
        let archiver = ProjectArchiverPostgres::new(Arc::new(db));

        let res = archiver.hard_delete(owner, project_id).await;
        assert!(matches!(res.unwrap_err(), ProjectArchiverError::NotFound));
    }

    #[tokio::test]
    async fn test_hard_delete_database_error() {
        let owner = UserId::from(Uuid::new_v4());
        let project_id = Uuid::new_v4();

        let db = mock_db_with_exec_error("connection error");
        let archiver = ProjectArchiverPostgres::new(Arc::new(db));

        let res = archiver.hard_delete(owner, project_id).await;
        assert!(matches!(
            res.unwrap_err(),
            ProjectArchiverError::DatabaseError(_)
        ));
    }
}
