use super::sea_orm_entity::user::{Column as UserColumn, Entity as UserEntity, Model as UserModel};
use crate::modules::auth::application::domain::entities::User;
use crate::modules::auth::application::ports::outgoing::UserQuery;
use async_trait::async_trait;
use chrono::Utc;
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

#[async_trait]
impl UserQuery for UserQueryPostgres {
    async fn find_by_id(&self, user_id: Uuid) -> Result<Option<User>, String> {
        let user = UserEntity::find()
            .filter(UserColumn::Id.eq(user_id))
            .one(&*self.db) // Dereference the Arc
            .await
            .map_err(|e| e.to_string())?;

        Ok(user.map(UserQueryPostgres::map_to_domain))
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, String> {
        let user = UserEntity::find()
            .filter(UserColumn::Email.eq(email))
            .one(&*self.db) // Dereference the Arc
            .await
            .map_err(|e| e.to_string())?;

        Ok(user.map(UserQueryPostgres::map_to_domain))
    }
    async fn find_by_username(&self, username: &str) -> Result<Option<User>, String> {
        let user = UserEntity::find()
            .filter(UserColumn::Username.eq(username))
            .one(&*self.db)
            .await
            .map_err(|e| e.to_string())?;

        Ok(user.map(UserQueryPostgres::map_to_domain))
    }
}

impl UserQueryPostgres {
    fn map_to_domain(model: UserModel) -> User {
        User {
            id: model.id,
            username: model.username,
            email: model.email,
            password_hash: model.password_hash,
            created_at: model.created_at.with_timezone(&Utc),
            updated_at: model.created_at.with_timezone(&Utc),
            is_verified: model.is_verified,
            is_deleted: model.is_deleted,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::UserQueryPostgres;
    use super::*;
    use crate::auth::adapter::outgoing::sea_orm_entity::user::Model as UserModel;
    use chrono::Utc;
    use sea_orm::{DatabaseBackend, MockDatabase};
    use uuid::Uuid;

    // Helper function to create a test user model
    fn create_test_user_model(id: Uuid) -> UserModel {
        let now = Utc::now();
        let fixed_offset_now = now.fixed_offset();

        UserModel {
            id,
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "hashed_password".to_string(),
            created_at: fixed_offset_now,
            updated_at: fixed_offset_now,
            is_verified: false,
            is_deleted: false,
        }
    }

    #[tokio::test]
    async fn test_find_by_id_found() {
        // Arrange
        let user_id = Uuid::new_v4();
        let user_model = create_test_user_model(user_id);

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![user_model.clone()]])
            .into_connection();

        let query = UserQueryPostgres::new(Arc::new(db));

        // Act
        let result = query.find_by_id(user_id).await;

        // Assert
        assert!(result.is_ok());
        let user_option = result.unwrap();
        assert!(user_option.is_some());
        let user = user_option.unwrap();
        assert_eq!(user.id, user_id);
        assert_eq!(user.username, "testuser");
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.password_hash, "hashed_password");
    }

    #[tokio::test]
    async fn test_find_by_id_not_found() {
        // Arrange
        let user_id = Uuid::new_v4();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<UserModel>::new()]) // Empty result
            .into_connection();

        let query = UserQueryPostgres::new(Arc::new(db));

        // Act
        let result = query.find_by_id(user_id).await;

        // Assert
        assert!(result.is_ok());
        let user_option = result.unwrap();
        assert!(user_option.is_none());
    }

    #[tokio::test]
    async fn test_find_by_email_found() {
        // Arrange
        let user_id = Uuid::new_v4();
        let email = "test@example.com";
        let user_model = create_test_user_model(user_id);

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![user_model.clone()]])
            .into_connection();

        let query = UserQueryPostgres::new(Arc::new(db));

        // Act
        let result = query.find_by_email(email).await;

        // Assert
        assert!(result.is_ok());
        let user_option = result.unwrap();
        assert!(user_option.is_some());
        let user = user_option.unwrap();
        assert_eq!(user.id, user_id);
        assert_eq!(user.email, email);
    }

    #[tokio::test]
    async fn test_find_by_email_not_found() {
        // Arrange
        let email = "nonexistent@example.com";

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<UserModel>::new()]) // Empty result
            .into_connection();

        let query = UserQueryPostgres::new(Arc::new(db));

        // Act
        let result = query.find_by_email(email).await;

        // Assert
        assert!(result.is_ok());
        let user_option = result.unwrap();
        assert!(user_option.is_none());
    }

    #[test]
    fn test_map_to_domain() {
        // Arrange
        let user_id = Uuid::new_v4();
        let now = Utc::now();
        let fixed_offset_now = now.fixed_offset();

        let user_model = UserModel {
            id: user_id,
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "hashed_password".to_string(),
            created_at: fixed_offset_now,
            updated_at: fixed_offset_now,
            is_verified: false,
            is_deleted: false,
        };

        // Act
        let user = UserQueryPostgres::map_to_domain(user_model);

        // Assert
        assert_eq!(user.id, user_id);
        assert_eq!(user.username, "testuser");
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.password_hash, "hashed_password");
        assert!(user.created_at.timezone() == Utc);
        assert!(user.updated_at.timezone() == Utc);
    }

    #[test]
    fn test_instance_can_be_cloned() {
        // Arrange
        let db = MockDatabase::new(DatabaseBackend::Postgres).into_connection();

        let query = UserQueryPostgres::new(Arc::new(db));

        // Act: Attempt to clone the instance
        let _ = query.clone();

        // Assert: No need to check anything specific, if it compiles
        // we know Arc is working correctly
        assert!(true);
    }

    #[tokio::test]
    async fn test_find_by_username_found() {
        // Arrange
        let user_id = Uuid::new_v4();
        let username = "testuser";
        let user_model = create_test_user_model(user_id);

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![user_model.clone()]])
            .into_connection();

        let query = UserQueryPostgres::new(Arc::new(db));

        // Act
        let result = query.find_by_username(username).await;

        // Assert
        assert!(result.is_ok());
        let user_option = result.unwrap();
        assert!(user_option.is_some());
        let user = user_option.unwrap();
        assert_eq!(user.id, user_id);
        assert_eq!(user.username, username);
    }

    #[tokio::test]
    async fn test_find_by_username_not_found() {
        // Arrange
        let username = "nonexistent_user";

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![Vec::<UserModel>::new()]) // Empty result
            .into_connection();

        let query = UserQueryPostgres::new(Arc::new(db));

        // Act
        let result = query.find_by_username(username).await;

        // Assert
        assert!(result.is_ok());
        let user_option = result.unwrap();
        assert!(user_option.is_none());
    }
}
