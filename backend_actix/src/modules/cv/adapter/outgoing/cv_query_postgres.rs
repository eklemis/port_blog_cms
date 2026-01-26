use crate::cv::{
    application::ports::outgoing::{CVQuery, CVQueryError},
    domain::entities::CVInfo,
};
use async_trait::async_trait;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use std::sync::Arc;
use uuid::Uuid;

// Bring in the entity we just defined above:
use super::sea_orm_entity::{Column as CvColumn, Entity as CvEntity, Model as CvModel};

#[derive(Debug, Clone)]
pub struct CVQueryPostgres {
    db: Arc<DatabaseConnection>,
}

impl CVQueryPostgres {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl CVQuery for CVQueryPostgres {
    async fn fetch_cv_by_user_id(&self, user_id: Uuid) -> Result<Vec<CVInfo>, CVQueryError> {
        let models: Vec<CvModel> = CvEntity::find()
            .filter(CvColumn::UserId.eq(user_id))
            .all(&*self.db)
            .await
            .map_err(|err| CVQueryError::DatabaseError(err.to_string()))?;

        Ok(models.into_iter().map(|m| m.to_domain()).collect())
    }

    async fn fetch_cv_by_id(&self, cv_id: Uuid) -> Result<Option<CVInfo>, CVQueryError> {
        let model: Option<CvModel> = CvEntity::find_by_id(cv_id)
            .one(&*self.db)
            .await
            .map_err(|err| CVQueryError::DatabaseError(err.to_string()))?;

        Ok(model.map(|m| m.to_domain()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sea_orm::{DatabaseBackend, DbErr, MockDatabase};
    use uuid::Uuid;

    fn create_cv_model(cv_id: Uuid, user_id: Uuid) -> CvModel {
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
            is_deleted: false,
        }
    }

    // ==================== fetch_cv_by_id tests ====================

    #[tokio::test]
    async fn test_fetch_cv_by_id_success() {
        let cv_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![create_cv_model(cv_id, user_id)]])
            .into_connection();

        let query = CVQueryPostgres::new(Arc::new(db));

        let result = query.fetch_cv_by_id(cv_id).await;

        assert!(result.is_ok());
        let cv_info = result.unwrap();
        assert!(cv_info.is_some());
        let cv = cv_info.unwrap();
        assert_eq!(cv.id, cv_id);
        assert_eq!(cv.user_id, user_id);
        assert_eq!(cv.bio, "Test bio");
        assert_eq!(cv.display_name, "Test User");
        assert_eq!(cv.role, "Developer");
    }

    #[tokio::test]
    async fn test_fetch_cv_by_id_not_found() {
        let cv_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<CvModel>::new()])
            .into_connection();

        let query = CVQueryPostgres::new(Arc::new(db));

        let result = query.fetch_cv_by_id(cv_id).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_fetch_cv_by_id_database_error() {
        let cv_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors(vec![DbErr::Custom("Connection failed".to_string())])
            .into_connection();

        let query = CVQueryPostgres::new(Arc::new(db));

        let result = query.fetch_cv_by_id(cv_id).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            CVQueryError::DatabaseError(msg) => {
                assert!(msg.contains("Connection failed"));
            }
            _ => panic!("Expected DatabaseError variant"),
        }
    }

    // ==================== fetch_cv_by_user_id tests ====================

    #[tokio::test]
    async fn test_fetch_cv_by_user_id_success() {
        let user_id = Uuid::new_v4();
        let cv_id_1 = Uuid::new_v4();
        let cv_id_2 = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![
                create_cv_model(cv_id_1, user_id),
                create_cv_model(cv_id_2, user_id),
            ]])
            .into_connection();

        let query = CVQueryPostgres::new(Arc::new(db));

        let result = query.fetch_cv_by_user_id(user_id).await;

        assert!(result.is_ok());
        let cvs = result.unwrap();
        assert_eq!(cvs.len(), 2);
        assert_eq!(cvs[0].id, cv_id_1);
        assert_eq!(cvs[1].id, cv_id_2);
        assert_eq!(cvs[0].user_id, user_id);
        assert_eq!(cvs[1].user_id, user_id);
    }

    #[tokio::test]
    async fn test_fetch_cv_by_user_id_empty() {
        let user_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<CvModel>::new()])
            .into_connection();

        let query = CVQueryPostgres::new(Arc::new(db));

        let result = query.fetch_cv_by_user_id(user_id).await;

        assert!(result.is_ok());
        let cvs = result.unwrap();
        assert!(cvs.is_empty());
    }

    #[tokio::test]
    async fn test_fetch_cv_by_user_id_single_result() {
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![create_cv_model(cv_id, user_id)]])
            .into_connection();

        let query = CVQueryPostgres::new(Arc::new(db));

        let result = query.fetch_cv_by_user_id(user_id).await;

        assert!(result.is_ok());
        let cvs = result.unwrap();
        assert_eq!(cvs.len(), 1);
        assert_eq!(cvs[0].id, cv_id);
        assert_eq!(cvs[0].user_id, user_id);
    }

    #[tokio::test]
    async fn test_fetch_cv_by_user_id_database_error() {
        let user_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors(vec![DbErr::Custom("Connection failed".to_string())])
            .into_connection();

        let query = CVQueryPostgres::new(Arc::new(db));

        let result = query.fetch_cv_by_user_id(user_id).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            CVQueryError::DatabaseError(msg) => {
                assert!(msg.contains("Connection failed"));
            }
            _ => panic!("Expected DatabaseError variant"),
        }
    }
}
