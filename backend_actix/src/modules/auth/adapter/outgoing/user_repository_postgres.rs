use async_trait::async_trait;
use sea_orm::ActiveValue::NotSet;
use sea_orm::{
    ActiveModelTrait, DatabaseBackend, DatabaseConnection, FromQueryResult, Set, Statement,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::application::ports::outgoing::user_repository::{CreateUserData, UserResult};
use crate::modules::auth::application::ports::outgoing::user_repository::{
    UserRepository, UserRepositoryError,
};

use super::sea_orm_entity::users::{ActiveModel as UserActiveModel, Model as UserModel};

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
        let result = UserModel::find_by_statement(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                r#"UPDATE users SET password_hash = $1, updated_at = NOW() WHERE id = $2 AND is_deleted = false RETURNING id"#,
                [new_password_hash.into(), user_id.into()],
            ))
            .one(&*self.db)
            .await
            .map_err(|e| UserRepositoryError::DatabaseError(e.to_string()))?;

        if result.is_none() {
            return Err(UserRepositoryError::UserNotFound);
        }

        Ok(())
    }

    async fn delete_user(&self, user_id: Uuid) -> Result<(), UserRepositoryError> {
        #[derive(FromQueryResult)]
        struct DeleteResult {
            id: Uuid,
        }

        let result = DeleteResult::find_by_statement(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"DELETE FROM users WHERE id = $1 RETURNING id"#,
            [user_id.into()],
        ))
        .one(&*self.db)
        .await
        .map_err(|e| UserRepositoryError::DatabaseError(e.to_string()))?;

        if result.is_none() {
            return Err(UserRepositoryError::UserNotFound);
        }

        Ok(())
    }
    async fn soft_delete_user(&self, user_id: Uuid) -> Result<(), UserRepositoryError> {
        #[derive(FromQueryResult)]
        struct UpdateResult {
            id: Uuid,
        }

        let result = UpdateResult::find_by_statement(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                r#"UPDATE users SET is_deleted = true, updated_at = NOW() WHERE id = $1 AND is_deleted = false RETURNING id"#,
                [user_id.into()],
            ))
            .one(&*self.db)
            .await
            .map_err(|e| UserRepositoryError::DatabaseError(e.to_string()))?;

        if result.is_none() {
            return Err(UserRepositoryError::UserNotFound);
        }

        Ok(())
    }
    async fn restore_user(&self, user_id: Uuid) -> Result<UserResult, UserRepositoryError> {
        let result = UserModel::find_by_statement(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                r#"UPDATE users SET is_deleted = false, updated_at = NOW() WHERE id = $1 AND is_deleted = true RETURNING *"#,
                [user_id.into()],
            ))
            .one(&*self.db)
            .await
            .map_err(|e| UserRepositoryError::DatabaseError(e.to_string()))?;

        result
            .map(Self::map_to_user_result)
            .ok_or(UserRepositoryError::UserNotFound)
    }
    async fn activate_user(&self, user_id: Uuid) -> Result<UserResult, UserRepositoryError> {
        let result = UserModel::find_by_statement(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                r#"UPDATE users SET is_verified = true, updated_at = NOW() WHERE id = $1 AND is_deleted = false RETURNING *"#,
                [user_id.into()],
            ))
            .one(&*self.db)
            .await
            .map_err(|e| UserRepositoryError::DatabaseError(e.to_string()))?;

        result
            .map(Self::map_to_user_result)
            .ok_or(UserRepositoryError::UserNotFound)
    }
    async fn set_full_name(
        &self,
        user_id: Uuid,
        full_name: String,
    ) -> Result<UserResult, UserRepositoryError> {
        let result = UserModel::find_by_statement(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                r#"UPDATE users SET full_name = $1, updated_at = NOW() WHERE id = $2 AND is_deleted = false RETURNING *"#,
                [full_name.into(), user_id.into()],
            ))
            .one(&*self.db)
            .await
            .map_err(|e| UserRepositoryError::DatabaseError(e.to_string()))?;

        result
            .map(Self::map_to_user_result)
            .ok_or(UserRepositoryError::UserNotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, FixedOffset, Utc};
    use sea_orm::{DatabaseBackend, DbErr, MockDatabase, MockExecResult};
    use uuid::Uuid;

    fn create_test_user_data() -> CreateUserData {
        CreateUserData {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "hashed_password".to_string(),
            full_name: "Gregor Brenkenstein".to_string(),
        }
    }

    fn to_fixed_offset(dt: DateTime<Utc>) -> chrono::DateTime<FixedOffset> {
        dt.fixed_offset()
    }

    fn create_user_model(user_id: Uuid) -> UserModel {
        let now = Utc::now();
        UserModel {
            id: user_id,
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "hashedpassword".to_string(),
            full_name: "Test User".to_string(),
            created_at: to_fixed_offset(now),
            updated_at: to_fixed_offset(now),
            is_verified: false,
            is_deleted: false,
        }
    }

    // ==================== create_user tests ====================

    #[tokio::test]
    async fn test_create_user_success() {
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

        let result = repository.create_user(user_data.clone()).await;

        assert!(result.is_ok());
        let user_result = result.unwrap();
        assert_eq!(user_result.username, user_data.username);
        assert_eq!(user_result.email, user_data.email);
        assert_eq!(user_result.full_name, user_data.full_name);
    }

    #[tokio::test]
    async fn test_create_user_duplicate_key_error() {
        let user_data = create_test_user_data();

        let mock_db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors([DbErr::Custom(
                "duplicate key value violates unique constraint".to_string(),
            )])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(mock_db));

        let result = repository.create_user(user_data).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            UserRepositoryError::UserAlreadyExists
        ));
    }

    #[tokio::test]
    async fn test_create_user_database_error() {
        let user_data = create_test_user_data();

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

    // ==================== update_password tests ====================

    #[tokio::test]
    async fn test_update_password_success() {
        let user_id = Uuid::new_v4();
        let new_password_hash = "new_hashed_password".to_string();

        // Single query returns the updated model
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![create_user_model(user_id)]])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(db));

        let result = repository.update_password(user_id, new_password_hash).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_password_user_not_found() {
        let user_id = Uuid::new_v4();
        let new_password_hash = "new_hashed_password".to_string();

        // Empty result means user not found
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<UserModel>::new()])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(db));

        let result = repository.update_password(user_id, new_password_hash).await;

        assert!(matches!(result, Err(UserRepositoryError::UserNotFound)));
    }

    #[tokio::test]
    async fn test_update_password_database_error() {
        let user_id = Uuid::new_v4();
        let new_password_hash = "new_hashed_password".to_string();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors([DbErr::Custom("connection error".to_string())])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(db));

        let result = repository.update_password(user_id, new_password_hash).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            UserRepositoryError::DatabaseError(msg) => {
                assert!(msg.contains("connection error"));
            }
            _ => panic!("Expected DatabaseError variant"),
        }
    }

    // ==================== delete_user tests ====================

    #[tokio::test]
    async fn test_delete_user_success() {
        let user_id = Uuid::new_v4();

        // MockDatabase requires ModelTrait, so use UserModel
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![create_user_model(user_id)]])
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
    async fn test_delete_user_database_error() {
        let user_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors([DbErr::Custom("delete failed".to_string())])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(db));

        let result = repository.delete_user(user_id).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            UserRepositoryError::DatabaseError(msg) => {
                assert!(msg.contains("delete failed"));
            }
            _ => panic!("Expected DatabaseError variant"),
        }
    }

    // ==================== soft_delete_user tests ====================

    #[tokio::test]
    async fn test_soft_delete_user_success() {
        let user_id = Uuid::new_v4();

        // MockDatabase requires ModelTrait, so use UserModel
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![create_user_model(user_id)]])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(db));

        let result = repository.soft_delete_user(user_id).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_soft_delete_user_not_found() {
        let user_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<UserModel>::new()])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(db));

        let result = repository.soft_delete_user(user_id).await;

        assert!(matches!(
            result.unwrap_err(),
            UserRepositoryError::UserNotFound
        ));
    }

    #[tokio::test]
    async fn test_soft_delete_user_database_error() {
        let user_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors([DbErr::Custom("connection error".to_string())])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(db));

        let result = repository.soft_delete_user(user_id).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            UserRepositoryError::DatabaseError(msg) => {
                assert!(msg.contains("connection error"));
            }
            _ => panic!("Expected DatabaseError variant"),
        }
    }

    // ==================== restore_user tests ====================

    #[tokio::test]
    async fn test_restore_user_success() {
        let user_id = Uuid::new_v4();

        // UPDATE...RETURNING * returns full model
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![create_user_model(user_id)]])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(db));

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

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<UserModel>::new()])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(db));

        let result = repository.restore_user(user_id).await;

        assert!(matches!(
            result.unwrap_err(),
            UserRepositoryError::UserNotFound
        ));
    }

    #[tokio::test]
    async fn test_restore_user_database_error() {
        let user_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors([DbErr::Custom("connection timeout".to_string())])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(db));

        let result = repository.restore_user(user_id).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            UserRepositoryError::DatabaseError(msg) => {
                assert!(msg.contains("connection timeout"));
            }
            _ => panic!("Expected DatabaseError variant"),
        }
    }

    // ==================== activate_user tests ====================

    #[tokio::test]
    async fn test_activate_user_success() {
        let user_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![create_user_model(user_id)]])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(db));

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

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<UserModel>::new()])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(db));

        let result = repository.activate_user(user_id).await;

        assert!(matches!(
            result.unwrap_err(),
            UserRepositoryError::UserNotFound
        ));
    }

    #[tokio::test]
    async fn test_activate_user_database_error() {
        let user_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors([DbErr::Custom("connection timeout".to_string())])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(db));

        let result = repository.activate_user(user_id).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            UserRepositoryError::DatabaseError(msg) => {
                assert!(msg.contains("connection timeout"));
            }
            _ => panic!("Expected DatabaseError variant"),
        }
    }

    // ==================== set_full_name tests ====================

    #[tokio::test]
    async fn test_set_full_name_success() {
        let user_id = Uuid::new_v4();
        let new_full_name = "Updated Name".to_string();
        let now = Utc::now();

        let updated_model = UserModel {
            id: user_id,
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "hashedpassword".to_string(),
            full_name: new_full_name.clone(),
            created_at: to_fixed_offset(now),
            updated_at: to_fixed_offset(now),
            is_verified: false,
            is_deleted: false,
        };

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![updated_model]])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(db));

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

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<UserModel>::new()])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(db));

        let result = repository.set_full_name(user_id, new_full_name).await;

        assert!(matches!(
            result.unwrap_err(),
            UserRepositoryError::UserNotFound
        ));
    }

    #[tokio::test]
    async fn test_set_full_name_database_error() {
        let user_id = Uuid::new_v4();
        let new_full_name = "Updated Name".to_string();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_errors([DbErr::Custom("connection timeout".to_string())])
            .into_connection();

        let repository = UserRepositoryPostgres::new(Arc::new(db));

        let result = repository.set_full_name(user_id, new_full_name).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            UserRepositoryError::DatabaseError(msg) => {
                assert!(msg.contains("connection timeout"));
            }
            _ => panic!("Expected DatabaseError variant"),
        }
    }

    // ==================== helper function tests ====================

    #[test]
    fn test_map_to_user_result() {
        let user_id = Uuid::new_v4();
        let user_model = create_user_model(user_id);

        let user_result = UserRepositoryPostgres::map_to_user_result(user_model.clone());

        assert_eq!(user_result.id, user_model.id);
        assert_eq!(user_result.username, user_model.username);
        assert_eq!(user_result.email, user_model.email);
        assert_eq!(user_result.full_name, user_model.full_name);
    }
}
