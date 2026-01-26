// cv_archiver_postgres.rs
use crate::cv::application::ports::outgoing::{CVArchiver, CVArchiverError};
use crate::cv::domain::entities::CVInfo;
use async_trait::async_trait;
use sea_orm::{DatabaseBackend, DatabaseConnection, FromQueryResult, Statement};
use std::sync::Arc;
use uuid::Uuid;

use super::sea_orm_entity::Model as CvModel;

#[derive(Debug, Clone)]
pub struct CVArchiverPostgres {
    db: Arc<DatabaseConnection>,
}

impl CVArchiverPostgres {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl CVArchiver for CVArchiverPostgres {
    async fn soft_delete(&self, cv_id: Uuid) -> Result<(), CVArchiverError> {
        #[derive(FromQueryResult)]
        struct IdResult {
            id: Uuid,
        }

        let result = IdResult::find_by_statement(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"UPDATE resumes SET is_deleted = true, updated_at = NOW() WHERE id = $1 AND is_deleted = false RETURNING id"#,
            [cv_id.into()],
        ))
        .one(&*self.db)
        .await
        .map_err(|e| CVArchiverError::DatabaseError(e.to_string()))?;

        match result {
            Some(_) => Ok(()),
            None => {
                // Check if CV exists but is already archived
                let exists = self.cv_exists(cv_id).await?;
                if exists {
                    Err(CVArchiverError::AlreadyArchived)
                } else {
                    Err(CVArchiverError::NotFound)
                }
            }
        }
    }

    async fn hard_delete(&self, cv_id: Uuid) -> Result<(), CVArchiverError> {
        #[derive(FromQueryResult)]
        struct IdResult {
            id: Uuid,
        }

        let result = IdResult::find_by_statement(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"DELETE FROM resumes WHERE id = $1 RETURNING id"#,
            [cv_id.into()],
        ))
        .one(&*self.db)
        .await
        .map_err(|e| CVArchiverError::DatabaseError(e.to_string()))?;

        if result.is_none() {
            return Err(CVArchiverError::NotFound);
        }

        Ok(())
    }

    async fn restore(&self, cv_id: Uuid) -> Result<CVInfo, CVArchiverError> {
        let result = CvModel::find_by_statement(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"UPDATE resumes SET is_deleted = false, updated_at = NOW() WHERE id = $1 AND is_deleted = true RETURNING *"#,
            [cv_id.into()],
        ))
        .one(&*self.db)
        .await
        .map_err(|e| CVArchiverError::DatabaseError(e.to_string()))?;

        match result {
            Some(model) => Ok(model.to_domain()),
            None => {
                // Check if CV exists but is not archived
                let exists = self.cv_exists(cv_id).await?;
                if exists {
                    Err(CVArchiverError::NotArchived)
                } else {
                    Err(CVArchiverError::NotFound)
                }
            }
        }
    }
}

impl CVArchiverPostgres {
    async fn cv_exists(&self, cv_id: Uuid) -> Result<bool, CVArchiverError> {
        #[derive(FromQueryResult)]
        struct ExistsResult {
            id: Uuid,
        }

        let result = ExistsResult::find_by_statement(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"SELECT id FROM resumes WHERE id = $1"#,
            [cv_id.into()],
        ))
        .one(&*self.db)
        .await
        .map_err(|e| CVArchiverError::DatabaseError(e.to_string()))?;

        Ok(result.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sea_orm::{DatabaseBackend, DbErr, MockDatabase};
    use uuid::Uuid;

    fn create_cv_model(cv_id: Uuid, user_id: Uuid, is_deleted: bool) -> CvModel {
        let now = Utc::now().fixed_offset();
        CvModel {
            id: cv_id,
            user_id,
            bio: "Test bio".to_string(),
            display_name: "Test User".to_string(),
            role: "Developer".to_string(),
            photo_url: "https://example.com/photo.jpg".to_string(),
            core_skills: serde_json::json!([]),
            educations: serde_json::json!([]),
            experiences: serde_json::json!([]),
            highlighted_projects: serde_json::json!([]),
            contact_info: serde_json::json!([]),
            created_at: now,
            updated_at: now,
            is_deleted,
        }
    }

    // ==================== soft_delete tests ====================

    #[tokio::test]
    async fn test_soft_delete_success() {
        let cv_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![create_cv_model(cv_id, user_id, true)]])
            .into_connection();

        let archiver = CVArchiverPostgres::new(Arc::new(db));

        let result = archiver.soft_delete(cv_id).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_soft_delete_already_archived() {
        let cv_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            // First query (UPDATE) returns empty - no rows updated
            .append_query_results(vec![Vec::<CvModel>::new()])
            // Second query (SELECT) returns the CV - it exists but is already archived
            .append_query_results(vec![vec![create_cv_model(cv_id, user_id, true)]])
            .into_connection();

        let archiver = CVArchiverPostgres::new(Arc::new(db));

        let result = archiver.soft_delete(cv_id).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CVArchiverError::AlreadyArchived
        ));
    }

    #[tokio::test]
    async fn test_soft_delete_not_found() {
        let cv_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            // First query (UPDATE) returns empty
            .append_query_results(vec![Vec::<CvModel>::new()])
            // Second query (SELECT) also returns empty - CV doesn't exist
            .append_query_results(vec![Vec::<CvModel>::new()])
            .into_connection();

        let archiver = CVArchiverPostgres::new(Arc::new(db));

        let result = archiver.soft_delete(cv_id).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CVArchiverError::NotFound));
    }

    #[tokio::test]
    async fn test_soft_delete_database_error() {
        let cv_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors(vec![DbErr::Custom("Connection failed".to_string())])
            .into_connection();

        let archiver = CVArchiverPostgres::new(Arc::new(db));

        let result = archiver.soft_delete(cv_id).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            CVArchiverError::DatabaseError(msg) => {
                assert!(msg.contains("Connection failed"));
            }
            _ => panic!("Expected DatabaseError variant"),
        }
    }

    // ==================== hard_delete tests ====================

    #[tokio::test]
    async fn test_hard_delete_success() {
        let cv_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![create_cv_model(cv_id, user_id, false)]])
            .into_connection();

        let archiver = CVArchiverPostgres::new(Arc::new(db));

        let result = archiver.hard_delete(cv_id).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_hard_delete_not_found() {
        let cv_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<CvModel>::new()])
            .into_connection();

        let archiver = CVArchiverPostgres::new(Arc::new(db));

        let result = archiver.hard_delete(cv_id).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CVArchiverError::NotFound));
    }

    #[tokio::test]
    async fn test_hard_delete_database_error() {
        let cv_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors(vec![DbErr::Custom("Connection failed".to_string())])
            .into_connection();

        let archiver = CVArchiverPostgres::new(Arc::new(db));

        let result = archiver.hard_delete(cv_id).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            CVArchiverError::DatabaseError(msg) => {
                assert!(msg.contains("Connection failed"));
            }
            _ => panic!("Expected DatabaseError variant"),
        }
    }

    // ==================== restore tests ====================

    #[tokio::test]
    async fn test_restore_success() {
        let cv_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let restored_model = create_cv_model(cv_id, user_id, false);

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![restored_model]])
            .into_connection();

        let archiver = CVArchiverPostgres::new(Arc::new(db));

        let result = archiver.restore(cv_id).await;

        assert!(result.is_ok());
        let cv_info = result.unwrap();
        assert_eq!(cv_info.id, cv_id);
        assert_eq!(cv_info.user_id, user_id);
        assert_eq!(cv_info.bio, "Test bio");
        assert_eq!(cv_info.display_name, "Test User");
        assert_eq!(cv_info.role, "Developer");
    }

    #[tokio::test]
    async fn test_restore_not_archived() {
        let cv_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            // First query (UPDATE) returns empty - no rows updated
            .append_query_results(vec![Vec::<CvModel>::new()])
            // Second query (SELECT) returns the CV - it exists but is not archived
            .append_query_results(vec![vec![create_cv_model(cv_id, user_id, false)]])
            .into_connection();

        let archiver = CVArchiverPostgres::new(Arc::new(db));

        let result = archiver.restore(cv_id).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CVArchiverError::NotArchived));
    }

    #[tokio::test]
    async fn test_restore_not_found() {
        let cv_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            // First query (UPDATE) returns empty
            .append_query_results(vec![Vec::<CvModel>::new()])
            // Second query (SELECT) also returns empty - CV doesn't exist
            .append_query_results(vec![Vec::<CvModel>::new()])
            .into_connection();

        let archiver = CVArchiverPostgres::new(Arc::new(db));

        let result = archiver.restore(cv_id).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CVArchiverError::NotFound));
    }

    #[tokio::test]
    async fn test_restore_database_error() {
        let cv_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors(vec![DbErr::Custom("Connection failed".to_string())])
            .into_connection();

        let archiver = CVArchiverPostgres::new(Arc::new(db));

        let result = archiver.restore(cv_id).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            CVArchiverError::DatabaseError(msg) => {
                assert!(msg.contains("Connection failed"));
            }
            _ => panic!("Expected DatabaseError variant"),
        }
    }

    // ==================== cv_exists helper tests ====================

    #[tokio::test]
    async fn test_cv_exists_database_error_propagates_in_soft_delete() {
        let cv_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            // First query (UPDATE) returns empty
            .append_query_results(vec![Vec::<CvModel>::new()])
            // Second query (SELECT for cv_exists) fails
            .append_query_errors(vec![DbErr::Custom("Connection lost".to_string())])
            .into_connection();

        let archiver = CVArchiverPostgres::new(Arc::new(db));

        let result = archiver.soft_delete(cv_id).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            CVArchiverError::DatabaseError(msg) => {
                assert!(msg.contains("Connection lost"));
            }
            _ => panic!("Expected DatabaseError variant"),
        }
    }

    #[tokio::test]
    async fn test_cv_exists_database_error_propagates_in_restore() {
        let cv_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            // First query (UPDATE) returns empty
            .append_query_results(vec![Vec::<CvModel>::new()])
            // Second query (SELECT for cv_exists) fails
            .append_query_errors(vec![DbErr::Custom("Connection lost".to_string())])
            .into_connection();

        let archiver = CVArchiverPostgres::new(Arc::new(db));

        let result = archiver.restore(cv_id).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            CVArchiverError::DatabaseError(msg) => {
                assert!(msg.contains("Connection lost"));
            }
            _ => panic!("Expected DatabaseError variant"),
        }
    }
}
