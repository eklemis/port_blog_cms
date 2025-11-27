use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::sync::Arc;
use uuid::Uuid;

use crate::modules::auth::application::domain::entities::User;
use crate::modules::auth::application::ports::outgoing::user_repository::{
    UserRepository, UserRepositoryError,
};

use super::sea_orm_entity::user::{
    ActiveModel as UserActiveModel, Column as UserColumn, Entity as UserEntity, Model as UserModel,
};

#[derive(Clone, Debug)]
pub struct UserRepositoryPostgres {
    db: Arc<DatabaseConnection>,
}

impl UserRepositoryPostgres {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Helper: Convert `UserModel` to `User` domain entity
    fn map_to_domain(model: UserModel) -> User {
        User {
            id: model.id,
            username: model.username,
            email: model.email,
            password_hash: model.password_hash,
            created_at: model.created_at.with_timezone(&chrono::Utc), // Convert FixedOffset → Utc
            updated_at: model.updated_at.with_timezone(&chrono::Utc), // Convert FixedOffset → Utc
            is_verified: false,
            is_deleted: false,
        }
    }
}

#[async_trait]
impl UserRepository for UserRepositoryPostgres {
    async fn create_user(&self, user: User) -> Result<User, UserRepositoryError> {
        let active_user = UserActiveModel {
            id: Set(user.id),
            username: Set(user.username),
            email: Set(user.email),
            password_hash: Set(user.password_hash),
            created_at: Set(user.created_at.into()),
            updated_at: Set(user.updated_at.into()),
            is_verified: Set(false),
            is_deleted: Set(false),
        };

        let inserted = active_user.insert(&*self.db).await.map_err(|e| {
            let err_str = e.to_string().to_lowercase();

            // Check for unique constraint violation (works for both real Postgres and MockDatabase)
            if err_str.contains("23505")
                || err_str.contains("duplicate key")
                || err_str.contains("unique constraint")
            {
                return UserRepositoryError::UserAlreadyExists;
            }

            // Any other database error
            UserRepositoryError::DatabaseError(e.to_string())
        })?;

        Ok(Self::map_to_domain(inserted))
    }

    async fn update_password(
        &self,
        user_id: Uuid,
        new_password_hash: String,
    ) -> Result<(), UserRepositoryError> {
        let user = UserEntity::find()
            .filter(UserColumn::Id.eq(user_id))
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
        let user = UserEntity::find()
            .filter(UserColumn::Id.eq(user_id))
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
        let user = UserEntity::find()
            .filter(UserColumn::Id.eq(user_id))
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
    async fn restore_user(&self, user_id: Uuid) -> Result<User, UserRepositoryError> {
        let user = UserEntity::find()
            .filter(UserColumn::Id.eq(user_id))
            .one(&*self.db)
            .await
            .map_err(|e| UserRepositoryError::DatabaseError(e.to_string()))? // ✅ Convert DbErr
            .ok_or(UserRepositoryError::UserNotFound)?; // ✅ Handle missing user case

        let mut active_user: UserActiveModel = user.into();
        active_user.is_deleted = Set(false); // ✅ Restore user
        active_user.updated_at = Set(chrono::Utc::now().into());

        let updated_user = active_user
            .update(&*self.db)
            .await
            .map_err(|e| UserRepositoryError::DatabaseError(e.to_string()))?; // ✅ Convert DbErr

        Ok(Self::map_to_domain(updated_user))
    }
    async fn activate_user(&self, user_id: Uuid) -> Result<(), UserRepositoryError> {
        let user = UserEntity::find()
            .filter(UserColumn::Id.eq(user_id))
            .one(&*self.db)
            .await
            .map_err(|e| UserRepositoryError::DatabaseError(e.to_string()))? // ✅ Convert DbErr
            .ok_or(UserRepositoryError::UserNotFound)?; // ✅ Handle missing user case

        let mut active_user: UserActiveModel = user.into();

        active_user.is_verified = Set(true);
        active_user
            .update(&*self.db)
            .await
            .map_err(|e| UserRepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, FixedOffset, Utc};
    use sea_orm::{DatabaseBackend, MockDatabase, MockExecResult};
    use uuid::Uuid;

    // Mock dependencies and test data creation helper
    fn create_test_user() -> User {
        let now = Utc::now();
        User {
            id: Uuid::new_v4(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "hashed_password".to_string(),
            created_at: now,
            updated_at: now,
            is_verified: false,
            is_deleted: false,
        }
    }

    // Helper function to convert timestamps for mock database
    fn to_naive_datetime(dt: DateTime<Utc>) -> chrono::DateTime<FixedOffset> {
        dt.fixed_offset()
    }

    #[tokio::test]
    async fn test_create_user_success() {
        // Arrange
        let user = create_test_user();

        let mock_user_model = UserModel {
            id: user.id,
            username: user.username.clone(),
            email: user.email.clone(),
            password_hash: user.password_hash.clone(),
            created_at: to_naive_datetime(user.created_at),
            updated_at: to_naive_datetime(user.updated_at),
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
        let result = repository.create_user(user.clone()).await;

        // Assert
        assert!(result.is_ok());
        let created_user = result.unwrap();
        assert_eq!(created_user.id, user.id);
        assert_eq!(created_user.username, user.username);
        assert_eq!(created_user.email, user.email);
        assert_eq!(created_user.created_at, user.created_at);
    }

    #[tokio::test]
    async fn test_create_user_duplicate_key_error() {
        let user = create_test_user();

        use sea_orm::DbErr;

        // Setup mock database
        let mock_db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors([DbErr::Custom(
                "duplicate key value violates unique constraint".to_string(),
            )])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(mock_db));

        let result = repository.create_user(user).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        println!("Error received: {:?}", error);
        assert!(matches!(error, UserRepositoryError::UserAlreadyExists));
    }
    #[tokio::test]
    async fn test_create_user_database_error() {
        let user = create_test_user();

        use sea_orm::DbErr;

        let mock_db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors([DbErr::Custom("connection timeout".to_string())])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(mock_db));

        let result = repository.create_user(user).await;

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
        let user = create_test_user();
        let new_password_hash = "new_hashed_password".to_string();

        // Create a mock user to be found
        let mock_user_model = UserModel {
            id: user.id,
            username: user.username.clone(),
            email: user.email.clone(),
            password_hash: user.password_hash.clone(),
            created_at: to_naive_datetime(user.created_at),
            updated_at: to_naive_datetime(user.updated_at),
            is_verified: false,
            is_deleted: false,
        };

        // Create an updated mock user (with the new password)
        let updated_mock_user_model = UserModel {
            id: user.id,
            username: user.username.clone(),
            email: user.email.clone(),
            password_hash: new_password_hash.clone(),
            created_at: to_naive_datetime(user.created_at),
            updated_at: to_naive_datetime(Utc::now()), // Use fresh timestamp for updated_at
            is_verified: false,
            is_deleted: false,
        };

        // In SeaORM Mock, we need to provide all expected query results
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            // First result - for finding the user
            .append_query_results(vec![vec![mock_user_model]])
            // Second result - for returning the updated model after update
            .append_query_results(vec![vec![updated_mock_user_model]])
            // Exec result for the update operation
            .append_exec_results(vec![MockExecResult {
                last_insert_id: 0,
                rows_affected: 1,
            }])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(db));

        // Act
        let result = repository
            .update_password(user.id, new_password_hash.clone())
            .await;

        // Assert
        assert!(result.is_ok(), "Failed to update password: {:?}", result);
    }

    #[tokio::test]
    async fn test_update_password_user_not_found() {
        // Arrange
        let user_id = Uuid::new_v4();
        let new_password_hash = "new_hashed_password".to_string();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<UserModel>::new()]) // Empty result set
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(db));

        // Act
        let result = repository.update_password(user_id, new_password_hash).await;

        // Assert
        assert!(matches!(result, Err(UserRepositoryError::UserNotFound)));
    }

    #[tokio::test]
    async fn test_delete_user_success() {
        // Arrange
        let user = create_test_user();

        // Create a mock user to be found and deleted
        let mock_user_model = UserModel {
            id: user.id,
            username: user.username.clone(),
            email: user.email.clone(),
            password_hash: user.password_hash.clone(),
            created_at: to_naive_datetime(user.created_at),
            updated_at: to_naive_datetime(user.updated_at),
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

        // Act
        let result = repository.delete_user(user.id).await;

        // Assert
        assert!(result.is_ok());
    }
    #[tokio::test]
    async fn test_soft_delete_user_success() {
        let user = create_test_user();
        let user_id = Uuid::new_v4();

        let mock_db = MockDatabase::new(DatabaseBackend::Postgres)
            // Mock the find query - return a user
            .append_query_results([vec![UserModel {
                id: user_id,
                username: "testuser".to_string(),
                email: "test@example.com".to_string(),
                password_hash: "hashedpassword".to_string(),
                created_at: to_naive_datetime(user.created_at),
                updated_at: to_naive_datetime(user.updated_at),
                is_verified: false,
                is_deleted: false,
            }]])
            // Mock the update query - return updated user
            .append_query_results([vec![UserModel {
                id: user_id,
                username: "testuser".to_string(),
                email: "test@example.com".to_string(),
                password_hash: "hashedpassword".to_string(),
                created_at: to_naive_datetime(user.created_at),
                updated_at: to_naive_datetime(user.updated_at),
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
            // Mock the find query - return empty (user not found)
            .append_query_results([Vec::<UserModel>::new()])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(mock_db));

        let result = repository.soft_delete_user(user_id).await;

        assert!(result.is_err());
        let error_str = result.unwrap_err();
        println!("Actual error : {:?}", error_str);
        assert!(matches!(error_str, UserRepositoryError::UserNotFound));
    }

    #[tokio::test]
    async fn test_soft_delete_user_database_error_on_find() {
        let user_id = Uuid::new_v4();

        use sea_orm::DbErr;

        let mock_db = MockDatabase::new(DatabaseBackend::Postgres)
            // Mock a database error on the find query
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

        let user = create_test_user();
        let user_id = Uuid::new_v4();

        let mock_db = MockDatabase::new(DatabaseBackend::Postgres)
            // Mock the find query - return a user
            .append_query_results([vec![UserModel {
                id: user_id,
                username: "testuser".to_string(),
                email: "test@example.com".to_string(),
                password_hash: "hashedpassword".to_string(),
                created_at: to_naive_datetime(user.created_at),
                updated_at: to_naive_datetime(user.updated_at),
                is_verified: false,
                is_deleted: false,
            }]])
            // Mock a database error on the update query
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
    async fn test_delete_user_not_found() {
        // Arrange
        let user_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<UserModel>::new()]) // Empty result set
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(db));

        // Act
        let result = repository.delete_user(user_id).await;

        // Assert
        assert!(matches!(result, Err(UserRepositoryError::UserNotFound)));
    }

    #[tokio::test]
    async fn test_restore_user_success() {
        let user_id = Uuid::new_v4();
        let now = Utc::now();

        let mock_db = MockDatabase::new(DatabaseBackend::Postgres)
            // Mock the find query - return a deleted user
            .append_query_results([vec![UserModel {
                id: user_id,
                username: "testuser".to_string(),
                email: "test@example.com".to_string(),
                password_hash: "hashedpassword".to_string(),
                created_at: to_naive_datetime(now),
                updated_at: to_naive_datetime(now),
                is_verified: false,
                is_deleted: true,
            }]])
            // Mock the update query - return restored user
            .append_query_results([vec![UserModel {
                id: user_id,
                username: "testuser".to_string(),
                email: "test@example.com".to_string(),
                password_hash: "hashedpassword".to_string(),
                created_at: to_naive_datetime(now),
                updated_at: to_naive_datetime(Utc::now()),
                is_verified: false,
                is_deleted: false,
            }]])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(mock_db));

        let result = repository.restore_user(user_id).await;

        assert!(result.is_ok());
        let restored_user = result.unwrap();
        assert_eq!(restored_user.id, user_id);
        assert_eq!(restored_user.is_deleted, false);
    }

    #[tokio::test]
    async fn test_restore_user_not_found() {
        let user_id = Uuid::new_v4();

        let mock_db = MockDatabase::new(DatabaseBackend::Postgres)
            // Mock the find query - return empty (user not found)
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
            // Mock a database error on the find query
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
            // Mock the find query - return a deleted user
            .append_query_results([vec![UserModel {
                id: user_id,
                username: "testuser".to_string(),
                email: "test@example.com".to_string(),
                password_hash: "hashedpassword".to_string(),
                created_at: to_naive_datetime(now),
                updated_at: to_naive_datetime(now),
                is_verified: false,
                is_deleted: true,
            }]])
            // Mock a database error on the update query
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
            // Mock the find query - return an unverified user
            .append_query_results([vec![UserModel {
                id: user_id,
                username: "testuser".to_string(),
                email: "test@example.com".to_string(),
                password_hash: "hashedpassword".to_string(),
                created_at: to_naive_datetime(now),
                updated_at: to_naive_datetime(now),
                is_verified: false,
                is_deleted: false,
            }]])
            // Mock the update query - return activated user
            .append_query_results([vec![UserModel {
                id: user_id,
                username: "testuser".to_string(),
                email: "test@example.com".to_string(),
                password_hash: "hashedpassword".to_string(),
                created_at: to_naive_datetime(now),
                updated_at: to_naive_datetime(Utc::now()),
                is_verified: true,
                is_deleted: false,
            }]])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(mock_db));

        let result = repository.activate_user(user_id).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_activate_user_not_found() {
        let user_id = Uuid::new_v4();

        let mock_db = MockDatabase::new(DatabaseBackend::Postgres)
            // Mock the find query - return empty (user not found)
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
            // Mock a database error on the find query
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
            // Mock the find query - return an unverified user
            .append_query_results([vec![UserModel {
                id: user_id,
                username: "testuser".to_string(),
                email: "test@example.com".to_string(),
                password_hash: "hashedpassword".to_string(),
                created_at: to_naive_datetime(now),
                updated_at: to_naive_datetime(now),
                is_verified: false,
                is_deleted: false,
            }]])
            // Mock a database error on the update query
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
    // Helper method test to ensure domain mapping works correctly
    #[test]
    fn test_map_to_domain() {
        // Arrange
        let now = Utc::now();
        let fix_off_now = now.fixed_offset();

        let user_model = UserModel {
            id: Uuid::new_v4(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "hashed_password".to_string(),
            created_at: fix_off_now,
            updated_at: fix_off_now,
            is_verified: false,
            is_deleted: false,
        };

        // Act
        let user = UserRepositoryPostgres::map_to_domain(user_model.clone());

        // Assert
        assert_eq!(user.id, user_model.id);
        assert_eq!(user.username, user_model.username);
        assert_eq!(user.email, user_model.email);
        assert_eq!(user.password_hash, user_model.password_hash);
    }
}
