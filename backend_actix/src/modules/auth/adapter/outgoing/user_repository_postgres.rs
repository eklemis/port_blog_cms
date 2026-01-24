use async_trait::async_trait;
use sea_orm::ActiveValue::NotSet;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::application::ports::outgoing::user_repository::{CreateUserData, UserResult};
use crate::modules::auth::application::ports::outgoing::user_repository::{
    UserRepository, UserRepositoryError,
};

use super::sea_orm_entity::users::{
    ActiveModel as UserActiveModel, Entity as UserEntity, Model as UserModel,
};

#[derive(Clone, Debug)]
pub struct UserRepositoryPostgres {
    db: Arc<DatabaseConnection>,
}

impl UserRepositoryPostgres {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    // Helper to map SeaORM model to UserResult (for operations that return confirmation)
    fn map_to_user_result(model: UserModel) -> UserResult {
        UserResult {
            id: model.id,
            email: model.email,
            username: model.username,
            full_name: model.full_name,
        }
    }
}

#[async_trait]
impl UserRepository for UserRepositoryPostgres {
    async fn create_user(&self, user: CreateUserData) -> Result<UserResult, UserRepositoryError> {
        let user_id = Uuid::new_v4();
        let active_user = UserActiveModel {
            id: Set(user_id),
            username: Set(user.username),
            email: Set(user.email),
            password_hash: Set(user.password_hash),
            full_name: Set(user.full_name),
            created_at: NotSet,
            updated_at: NotSet,
            is_verified: Set(false),
            is_deleted: Set(false),
        };

        let inserted = active_user.insert(&*self.db).await.map_err(|e| {
            let err_str = e.to_string().to_lowercase();
            if err_str.contains("23505")
                || err_str.contains("duplicate key")
                || err_str.contains("unique constraint")
            {
                return UserRepositoryError::UserAlreadyExists;
            }
            UserRepositoryError::DatabaseError(e.to_string())
        })?;

        Ok(Self::map_to_user_result(inserted))
    }

    async fn update_password(
        &self,
        user_id: Uuid,
        new_password_hash: String,
    ) -> Result<(), UserRepositoryError> {
        let user = UserEntity::find_by_id(user_id)
            .one(&*self.db)
            .await
            .map_err(|e| UserRepositoryError::DatabaseError(e.to_string()))?
            .ok_or(UserRepositoryError::UserNotFound)?;

        let mut active_user: UserActiveModel = user.into();
        active_user.password_hash = Set(new_password_hash);
        active_user.updated_at = Set(chrono::Utc::now().into()); // Update timestamp

        active_user
            .update(&*self.db)
            .await
            .map_err(|e| UserRepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn delete_user(&self, user_id: Uuid) -> Result<(), UserRepositoryError> {
        let user = UserEntity::find_by_id(user_id)
            .one(&*self.db)
            .await
            .map_err(|e| UserRepositoryError::DatabaseError(e.to_string()))?
            .ok_or(UserRepositoryError::UserNotFound)?;

        let active_user: UserActiveModel = user.into();
        active_user
            .delete(&*self.db)
            .await
            .map_err(|e| UserRepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }
    async fn soft_delete_user(&self, user_id: Uuid) -> Result<(), UserRepositoryError> {
        let user = UserEntity::find_by_id(user_id)
            .one(&*self.db)
            .await
            .map_err(|e| UserRepositoryError::DatabaseError(e.to_string()))?
            .ok_or(UserRepositoryError::UserNotFound)?;

        let mut active_user: UserActiveModel = user.into();
        active_user.is_deleted = Set(true);

        active_user
            .update(&*self.db)
            .await
            .map_err(|e| UserRepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }
    async fn restore_user(&self, user_id: Uuid) -> Result<UserResult, UserRepositoryError> {
        let user = UserEntity::find_by_id(user_id)
            .one(&*self.db)
            .await
            .map_err(|e| UserRepositoryError::DatabaseError(e.to_string()))?
            .ok_or(UserRepositoryError::UserNotFound)?;

        let mut active_user: UserActiveModel = user.into();
        active_user.is_deleted = Set(false); // ✅ Restore user

        let restored = active_user
            .update(&*self.db)
            .await
            .map_err(|e| UserRepositoryError::DatabaseError(e.to_string()))?;

        Ok(Self::map_to_user_result(restored))
    }
    async fn activate_user(&self, user_id: Uuid) -> Result<UserResult, UserRepositoryError> {
        let user = UserEntity::find_by_id(user_id)
            .one(&*self.db)
            .await
            .map_err(|e| UserRepositoryError::DatabaseError(e.to_string()))?
            .ok_or(UserRepositoryError::UserNotFound)?;

        let mut active_user: UserActiveModel = user.into();

        active_user.is_verified = Set(true);

        let activated = active_user
            .update(&*self.db)
            .await
            .map_err(|e| UserRepositoryError::DatabaseError(e.to_string()))?;

        Ok(Self::map_to_user_result(activated))
    }
    async fn set_full_name(
        &self,
        user_id: Uuid,
        full_name: String,
    ) -> Result<UserResult, UserRepositoryError> {
        let user = UserEntity::find_by_id(user_id)
            .one(&*self.db)
            .await
            .map_err(|e| UserRepositoryError::DatabaseError(e.to_string()))?
            .ok_or(UserRepositoryError::UserNotFound)?;

        let mut active_user: UserActiveModel = user.into();

        active_user.full_name = Set(full_name);

        let updated = active_user
            .update(&*self.db)
            .await
            .map_err(|e| UserRepositoryError::DatabaseError(e.to_string()))?;

        Ok(Self::map_to_user_result(updated))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, FixedOffset, Utc};
    use sea_orm::{DatabaseBackend, MockDatabase, MockExecResult};
    use uuid::Uuid;

    // Mock dependencies and test data creation helper
    fn create_test_user_data() -> CreateUserData {
        CreateUserData {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "hashed_password".to_string(),
            full_name: "Gregor Brenkenstein".to_string(),
        }
    }

    // Helper function to convert timestamps for mock database
    fn to_fixed_offset(dt: DateTime<Utc>) -> chrono::DateTime<FixedOffset> {
        dt.fixed_offset()
    }

    #[tokio::test]
    async fn test_create_user_success() {
        // Arrange
        let user_data = create_test_user_data();
        let user_id = Uuid::new_v4();
        let curr_time = chrono::Utc::now();

        let mock_user_model = UserModel {
            id: user_id,
            username: user_data.username.clone(),
            email: user_data.email.clone(),
            password_hash: user_data.password_hash.clone(),
            full_name: user_data.full_name.clone(),
            created_at: curr_time.into(),
            updated_at: curr_time.into(),
            is_verified: false,
            is_deleted: false,
        };

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![mock_user_model.clone()]])
            .append_exec_results(vec![MockExecResult {
                last_insert_id: 1,
                rows_affected: 1,
            }])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(db));

        // Act
        let result = repository.create_user(user_data.clone()).await;

        // Assert
        assert!(result.is_ok());
        let user_result = result.unwrap();
        assert_eq!(user_result.username, user_data.username);
        assert_eq!(user_result.email, user_data.email);
        assert_eq!(user_result.full_name, user_data.full_name);
        // UserResult doesn't include password_hash (good for security)
    }

    #[tokio::test]
    async fn test_create_user_duplicate_key_error() {
        let user_data = create_test_user_data();

        use sea_orm::DbErr;

        let mock_db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors([DbErr::Custom(
                "duplicate key value violates unique constraint".to_string(),
            )])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(mock_db));

        let result = repository.create_user(user_data).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, UserRepositoryError::UserAlreadyExists));
    }

    #[tokio::test]
    async fn test_create_user_database_error() {
        let user_data = create_test_user_data();

        use sea_orm::DbErr;

        let mock_db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors([DbErr::Custom("connection timeout".to_string())])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(mock_db));

        let result = repository.create_user(user_data).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            UserRepositoryError::DatabaseError(msg) => {
                assert!(msg.contains("connection timeout"));
            }
            _ => panic!("Expected DatabaseError variant"),
        }
    }

    #[tokio::test]
    async fn test_update_password_success() {
        // Arrange
        let user_id = Uuid::new_v4();
        let new_password_hash = "new_hashed_password".to_string();
        let now = Utc::now();

        let mock_user_model = UserModel {
            id: user_id,
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "old_password_hash".to_string(),
            full_name: "Test User".to_string(),
            created_at: to_fixed_offset(now),
            updated_at: to_fixed_offset(now),
            is_verified: false,
            is_deleted: false,
        };

        let updated_mock_user_model = UserModel {
            id: user_id,
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: new_password_hash.clone(),
            full_name: "Test User".to_string(),
            created_at: to_fixed_offset(now),
            updated_at: to_fixed_offset(Utc::now()),
            is_verified: false,
            is_deleted: false,
        };

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![mock_user_model]])
            .append_query_results(vec![vec![updated_mock_user_model]])
            .append_exec_results(vec![MockExecResult {
                last_insert_id: 0,
                rows_affected: 1,
            }])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(db));

        // Act
        let result = repository
            .update_password(user_id, new_password_hash.clone())
            .await;

        // Assert
        assert!(result.is_ok(), "Failed to update password: {:?}", result);
    }

    #[tokio::test]
    async fn test_update_password_user_not_found() {
        let user_id = Uuid::new_v4();
        let new_password_hash = "new_hashed_password".to_string();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<UserModel>::new()])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(db));

        let result = repository.update_password(user_id, new_password_hash).await;

        assert!(matches!(result, Err(UserRepositoryError::UserNotFound)));
    }

    #[tokio::test]
    async fn test_delete_user_success() {
        let user_id = Uuid::new_v4();
        let now = Utc::now();

        let mock_user_model = UserModel {
            id: user_id,
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "hashed_password".to_string(),
            full_name: "Test User".to_string(),
            created_at: to_fixed_offset(now),
            updated_at: to_fixed_offset(now),
            is_verified: false,
            is_deleted: false,
        };

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![mock_user_model]])
            .append_exec_results(vec![MockExecResult {
                last_insert_id: 1,
                rows_affected: 1,
            }])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(db));

        let result = repository.delete_user(user_id).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_user_not_found() {
        let user_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<UserModel>::new()])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(db));

        let result = repository.delete_user(user_id).await;

        assert!(matches!(result, Err(UserRepositoryError::UserNotFound)));
    }

    #[tokio::test]
    async fn test_soft_delete_user_success() {
        let user_id = Uuid::new_v4();
        let now = Utc::now();

        let mock_db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results([vec![UserModel {
                id: user_id,
                username: "testuser".to_string(),
                email: "test@example.com".to_string(),
                password_hash: "hashedpassword".to_string(),
                full_name: "Test User".to_string(),
                created_at: to_fixed_offset(now),
                updated_at: to_fixed_offset(now),
                is_verified: false,
                is_deleted: false,
            }]])
            .append_query_results([vec![UserModel {
                id: user_id,
                username: "testuser".to_string(),
                email: "test@example.com".to_string(),
                password_hash: "hashedpassword".to_string(),
                full_name: "Test User".to_string(),
                created_at: to_fixed_offset(now),
                updated_at: to_fixed_offset(now),
                is_verified: false,
                is_deleted: true,
            }]])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(mock_db));

        let result = repository.soft_delete_user(user_id).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_soft_delete_user_not_found() {
        let user_id = Uuid::new_v4();

        let mock_db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results([Vec::<UserModel>::new()])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(mock_db));

        let result = repository.soft_delete_user(user_id).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            UserRepositoryError::UserNotFound
        ));
    }

    #[tokio::test]
    async fn test_soft_delete_user_database_error_on_find() {
        let user_id = Uuid::new_v4();

        use sea_orm::DbErr;

        let mock_db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors([DbErr::Custom("connection error".to_string())])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(mock_db));

        let result = repository.soft_delete_user(user_id).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            UserRepositoryError::DatabaseError(msg) => {
                assert!(msg.contains("connection error"));
            }
            _ => panic!("Expected DatabaseError variant"),
        }
    }

    #[tokio::test]
    async fn test_soft_delete_user_database_error_on_update() {
        use sea_orm::DbErr;

        let user_id = Uuid::new_v4();
        let now = Utc::now();

        let mock_db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results([vec![UserModel {
                id: user_id,
                username: "testuser".to_string(),
                email: "test@example.com".to_string(),
                password_hash: "hashedpassword".to_string(),
                full_name: "Test User".to_string(),
                created_at: to_fixed_offset(now),
                updated_at: to_fixed_offset(now),
                is_verified: false,
                is_deleted: false,
            }]])
            .append_query_errors([DbErr::Custom("update failed".to_string())])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(mock_db));

        let result = repository.soft_delete_user(user_id).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            UserRepositoryError::DatabaseError(msg) => {
                assert!(msg.contains("update failed"));
            }
            _ => panic!("Expected DatabaseError variant"),
        }
    }

    #[tokio::test]
    async fn test_restore_user_success() {
        let user_id = Uuid::new_v4();
        let now = Utc::now();

        let mock_db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results([vec![UserModel {
                id: user_id,
                username: "testuser".to_string(),
                email: "test@example.com".to_string(),
                password_hash: "hashedpassword".to_string(),
                full_name: "Test User".to_string(),
                created_at: to_fixed_offset(now),
                updated_at: to_fixed_offset(now),
                is_verified: false,
                is_deleted: true,
            }]])
            .append_query_results([vec![UserModel {
                id: user_id,
                username: "testuser".to_string(),
                email: "test@example.com".to_string(),
                password_hash: "hashedpassword".to_string(),
                full_name: "Test User".to_string(),
                created_at: to_fixed_offset(now),
                updated_at: to_fixed_offset(Utc::now()),
                is_verified: false,
                is_deleted: false,
            }]])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(mock_db));

        let result = repository.restore_user(user_id).await;

        assert!(result.is_ok());
        let restored_user = result.unwrap();
        assert_eq!(restored_user.id, user_id);
        assert_eq!(restored_user.username, "testuser");
        assert_eq!(restored_user.email, "test@example.com");
        assert_eq!(restored_user.full_name, "Test User");
    }

    #[tokio::test]
    async fn test_restore_user_not_found() {
        let user_id = Uuid::new_v4();

        let mock_db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results([Vec::<UserModel>::new()])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(mock_db));

        let result = repository.restore_user(user_id).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            UserRepositoryError::UserNotFound
        ));
    }

    #[tokio::test]
    async fn test_restore_user_database_error_on_find() {
        use sea_orm::DbErr;

        let user_id = Uuid::new_v4();

        let mock_db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors([DbErr::Custom("connection timeout".to_string())])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(mock_db));

        let result = repository.restore_user(user_id).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            UserRepositoryError::DatabaseError(msg) => {
                assert!(msg.contains("connection timeout"));
            }
            _ => panic!("Expected DatabaseError variant"),
        }
    }

    #[tokio::test]
    async fn test_restore_user_database_error_on_update() {
        use sea_orm::DbErr;

        let user_id = Uuid::new_v4();
        let now = Utc::now();

        let mock_db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results([vec![UserModel {
                id: user_id,
                username: "testuser".to_string(),
                email: "test@example.com".to_string(),
                password_hash: "hashedpassword".to_string(),
                full_name: "Test User".to_string(),
                created_at: to_fixed_offset(now),
                updated_at: to_fixed_offset(now),
                is_verified: false,
                is_deleted: true,
            }]])
            .append_query_errors([DbErr::Custom("update operation failed".to_string())])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(mock_db));

        let result = repository.restore_user(user_id).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            UserRepositoryError::DatabaseError(msg) => {
                assert!(msg.contains("update operation failed"));
            }
            _ => panic!("Expected DatabaseError variant"),
        }
    }

    #[tokio::test]
    async fn test_activate_user_success() {
        let user_id = Uuid::new_v4();
        let now = Utc::now();

        let mock_db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results([vec![UserModel {
                id: user_id,
                username: "testuser".to_string(),
                email: "test@example.com".to_string(),
                password_hash: "hashedpassword".to_string(),
                full_name: "Test User".to_string(),
                created_at: to_fixed_offset(now),
                updated_at: to_fixed_offset(now),
                is_verified: false,
                is_deleted: false,
            }]])
            .append_query_results([vec![UserModel {
                id: user_id,
                username: "testuser".to_string(),
                email: "test@example.com".to_string(),
                password_hash: "hashedpassword".to_string(),
                full_name: "Test User".to_string(),
                created_at: to_fixed_offset(now),
                updated_at: to_fixed_offset(Utc::now()),
                is_verified: true,
                is_deleted: false,
            }]])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(mock_db));

        let result = repository.activate_user(user_id).await;

        assert!(result.is_ok());
        let activated_user = result.unwrap();
        assert_eq!(activated_user.id, user_id);
        assert_eq!(activated_user.username, "testuser");
        assert_eq!(activated_user.email, "test@example.com");
    }

    #[tokio::test]
    async fn test_activate_user_not_found() {
        let user_id = Uuid::new_v4();

        let mock_db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results([Vec::<UserModel>::new()])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(mock_db));

        let result = repository.activate_user(user_id).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            UserRepositoryError::UserNotFound
        ));
    }

    #[tokio::test]
    async fn test_activate_user_database_error_on_find() {
        use sea_orm::DbErr;

        let user_id = Uuid::new_v4();

        let mock_db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors([DbErr::Custom("connection timeout".to_string())])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(mock_db));

        let result = repository.activate_user(user_id).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            UserRepositoryError::DatabaseError(msg) => {
                assert!(msg.contains("connection timeout"));
            }
            _ => panic!("Expected DatabaseError variant"),
        }
    }

    #[tokio::test]
    async fn test_activate_user_database_error_on_update() {
        use sea_orm::DbErr;

        let user_id = Uuid::new_v4();
        let now = Utc::now();

        let mock_db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results([vec![UserModel {
                id: user_id,
                username: "testuser".to_string(),
                email: "test@example.com".to_string(),
                password_hash: "hashedpassword".to_string(),
                full_name: "Test User".to_string(),
                created_at: to_fixed_offset(now),
                updated_at: to_fixed_offset(now),
                is_verified: false,
                is_deleted: false,
            }]])
            .append_query_errors([DbErr::Custom("update operation failed".to_string())])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(mock_db));

        let result = repository.activate_user(user_id).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            UserRepositoryError::DatabaseError(msg) => {
                assert!(msg.contains("update operation failed"));
            }
            _ => panic!("Expected DatabaseError variant"),
        }
    }

    #[tokio::test]
    async fn test_set_full_name_success() {
        let user_id = Uuid::new_v4();
        let now = Utc::now();
        let new_full_name = "Updated Name".to_string();

        let mock_db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results([vec![UserModel {
                id: user_id,
                username: "testuser".to_string(),
                email: "test@example.com".to_string(),
                password_hash: "hashedpassword".to_string(),
                full_name: "Old Name".to_string(),
                created_at: to_fixed_offset(now),
                updated_at: to_fixed_offset(now),
                is_verified: false,
                is_deleted: false,
            }]])
            .append_query_results([vec![UserModel {
                id: user_id,
                username: "testuser".to_string(),
                email: "test@example.com".to_string(),
                password_hash: "hashedpassword".to_string(),
                full_name: new_full_name.clone(),
                created_at: to_fixed_offset(now),
                updated_at: to_fixed_offset(Utc::now()),
                is_verified: false,
                is_deleted: false,
            }]])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(mock_db));

        let result = repository
            .set_full_name(user_id, new_full_name.clone())
            .await;

        assert!(result.is_ok());
        let updated_user = result.unwrap();
        assert_eq!(updated_user.id, user_id);
        assert_eq!(updated_user.full_name, new_full_name);
        assert_eq!(updated_user.username, "testuser");
        assert_eq!(updated_user.email, "test@example.com");
    }

    #[tokio::test]
    async fn test_set_full_name_not_found() {
        let user_id = Uuid::new_v4();
        let new_full_name = "Updated Name".to_string();

        let mock_db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results([Vec::<UserModel>::new()])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(mock_db));

        let result = repository.set_full_name(user_id, new_full_name).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            UserRepositoryError::UserNotFound
        ));
    }

    #[tokio::test]
    async fn test_set_full_name_database_error_on_find() {
        use sea_orm::DbErr;

        let user_id = Uuid::new_v4();
        let new_full_name = "Updated Name".to_string();

        let mock_db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors([DbErr::Custom("connection timeout".to_string())])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(mock_db));

        let result = repository.set_full_name(user_id, new_full_name).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            UserRepositoryError::DatabaseError(msg) => {
                assert!(msg.contains("connection timeout"));
            }
            _ => panic!("Expected DatabaseError variant"),
        }
    }

    #[tokio::test]
    async fn test_set_full_name_database_error_on_update() {
        use sea_orm::DbErr;

        let user_id = Uuid::new_v4();
        let now = Utc::now();
        let new_full_name = "Updated Name".to_string();

        let mock_db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results([vec![UserModel {
                id: user_id,
                username: "testuser".to_string(),
                email: "test@example.com".to_string(),
                password_hash: "hashedpassword".to_string(),
                full_name: "Old Name".to_string(),
                created_at: to_fixed_offset(now),
                updated_at: to_fixed_offset(now),
                is_verified: false,
                is_deleted: false,
            }]])
            .append_query_errors([DbErr::Custom("update operation failed".to_string())])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(mock_db));

        let result = repository.set_full_name(user_id, new_full_name).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            UserRepositoryError::DatabaseError(msg) => {
                assert!(msg.contains("update operation failed"));
            }
            _ => panic!("Expected DatabaseError variant"),
        }
    }

    // Test the new map_to_user_result helper
    #[test]
    fn test_map_to_user_result() {
        let now = Utc::now();
        let fix_off_now = now.fixed_offset();

        let user_model = UserModel {
            id: Uuid::new_v4(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "hashed_password".to_string(),
            full_name: "Test User".to_string(),
            created_at: fix_off_now,
            updated_at: fix_off_now,
            is_verified: false,
            is_deleted: false,
        };

        let user_result = UserRepositoryPostgres::map_to_user_result(user_model.clone());

        assert_eq!(user_result.id, user_model.id);
        assert_eq!(user_result.username, user_model.username);
        assert_eq!(user_result.email, user_model.email);
        assert_eq!(user_result.full_name, user_model.full_name);
        // UserResult intentionally doesn't include password_hash, is_verified, is_deleted, etc.
    }
}
