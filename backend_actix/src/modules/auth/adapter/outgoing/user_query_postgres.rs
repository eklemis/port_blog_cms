use super::sea_orm_entity::users::{
    Column as UserColumn, Entity as UserEntity, Model as UserModel,
};
use crate::auth::application::ports::outgoing::user_query::UserQueryError;
use crate::auth::application::ports::outgoing::user_query::UserQueryResult;
use crate::modules::auth::application::ports::outgoing::UserQuery;
use async_trait::async_trait;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct UserQueryPostgres {
    db: Arc<DatabaseConnection>, // Wrap in Arc
}

impl UserQueryPostgres {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}
impl UserQueryPostgres {
    /// Helper to map SeaORM model to UserQueryResult
    fn map_to_query_result(model: UserModel) -> UserQueryResult {
        UserQueryResult {
            id: model.id,
            email: model.email,
            username: model.username,
            password_hash: model.password_hash,
            full_name: model.full_name,
            created_at: model.created_at.with_timezone(&chrono::Utc),
            updated_at: model.updated_at.with_timezone(&chrono::Utc),
            is_verified: model.is_verified,
            is_deleted: model.is_deleted,
        }
    }
}

#[async_trait]
impl UserQuery for UserQueryPostgres {
    async fn find_by_id(&self, user_id: Uuid) -> Result<Option<UserQueryResult>, UserQueryError> {
        let user = UserEntity::find_by_id(user_id)
            .one(&*self.db)
            .await
            .map_err(|e| UserQueryError::DatabaseError(e.to_string()))?;

        Ok(user.map(Self::map_to_query_result))
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<UserQueryResult>, UserQueryError> {
        let user = UserEntity::find()
            .filter(UserColumn::Email.eq(email))
            .one(&*self.db)
            .await
            .map_err(|e| UserQueryError::DatabaseError(e.to_string()))?;

        Ok(user.map(Self::map_to_query_result))
    }
    async fn find_by_username(
        &self,
        username: &str,
    ) -> Result<Option<UserQueryResult>, UserQueryError> {
        let user = UserEntity::find()
            .filter(UserColumn::Username.eq(username))
            .one(&*self.db)
            .await
            .map_err(|e| UserQueryError::DatabaseError(e.to_string()))?;

        Ok(user.map(Self::map_to_query_result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sea_orm::{DatabaseBackend, MockDatabase};

    fn create_mock_user_model(id: Uuid) -> UserModel {
        let now = Utc::now();
        UserModel {
            id,
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "hashed_password".to_string(),
            full_name: "Test User".to_string(),
            created_at: now.into(),
            updated_at: now.into(),
            is_verified: true,
            is_deleted: false,
        }
    }

    #[tokio::test]
    async fn test_find_by_id_success() {
        let user_id = Uuid::new_v4();
        let mock_user = create_mock_user_model(user_id);

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![mock_user.clone()]])
            .into_connection();

        let query = UserQueryPostgres::new(Arc::new(db));
        let result = query.find_by_id(user_id).await;

        assert!(result.is_ok());
        let user_result = result.unwrap();
        assert!(user_result.is_some());

        let user = user_result.unwrap();
        assert_eq!(user.id, user_id);
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.username, "testuser");
    }

    #[tokio::test]
    async fn test_find_by_id_not_found() {
        let user_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<UserModel>::new()])
            .into_connection();

        let query = UserQueryPostgres::new(Arc::new(db));
        let result = query.find_by_id(user_id).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_find_by_id_database_error() {
        use sea_orm::DbErr;

        let user_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors(vec![DbErr::Custom("connection timeout".to_string())])
            .into_connection();

        let query = UserQueryPostgres::new(Arc::new(db));
        let result = query.find_by_id(user_id).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            UserQueryError::DatabaseError(msg) => {
                assert!(msg.contains("connection timeout"));
            }
            _ => panic!("Expected DatabaseError"),
        }
    }

    #[tokio::test]
    async fn test_find_by_email_success() {
        let user_id = Uuid::new_v4();
        let mock_user = create_mock_user_model(user_id);

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![mock_user.clone()]])
            .into_connection();

        let query = UserQueryPostgres::new(Arc::new(db));
        let result = query.find_by_email("test@example.com").await;

        assert!(result.is_ok());
        let user_result = result.unwrap();
        assert!(user_result.is_some());

        let user = user_result.unwrap();
        assert_eq!(user.email, "test@example.com");
    }

    #[tokio::test]
    async fn test_find_by_email_not_found() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<UserModel>::new()])
            .into_connection();

        let query = UserQueryPostgres::new(Arc::new(db));
        let result = query.find_by_email("nonexistent@example.com").await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_find_by_username_success() {
        let user_id = Uuid::new_v4();
        let mock_user = create_mock_user_model(user_id);

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![mock_user.clone()]])
            .into_connection();

        let query = UserQueryPostgres::new(Arc::new(db));
        let result = query.find_by_username("testuser").await;

        assert!(result.is_ok());
        let user_result = result.unwrap();
        assert!(user_result.is_some());

        let user = user_result.unwrap();
        assert_eq!(user.username, "testuser");
    }

    #[tokio::test]
    async fn test_find_by_username_not_found() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<UserModel>::new()])
            .into_connection();

        let query = UserQueryPostgres::new(Arc::new(db));
        let result = query.find_by_username("nonexistent").await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_map_to_query_result() {
        let user_id = Uuid::new_v4();
        let now = Utc::now();

        let model = UserModel {
            id: user_id,
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "hashed_password".to_string(),
            full_name: "Test User".to_string(),
            created_at: now.into(),
            updated_at: now.into(),
            is_verified: true,
            is_deleted: false,
        };

        let query_result = UserQueryPostgres::map_to_query_result(model.clone());

        assert_eq!(query_result.id, model.id);
        assert_eq!(query_result.username, model.username);
        assert_eq!(query_result.email, model.email);
        assert_eq!(query_result.password_hash, model.password_hash);
        assert_eq!(query_result.full_name, model.full_name);
        assert_eq!(query_result.is_verified, model.is_verified);
        assert_eq!(query_result.is_deleted, model.is_deleted);
    }
}
