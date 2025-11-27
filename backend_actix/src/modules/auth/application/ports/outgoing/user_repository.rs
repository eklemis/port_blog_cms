use crate::modules::auth::application::domain::entities::User;
use async_trait::async_trait;
use std::fmt;
use uuid::Uuid;

#[async_trait]
pub trait UserRepository {
    async fn create_user(&self, user: User) -> Result<User, UserRepositoryError>;

    async fn update_password(
        &self,
        user_id: Uuid,
        new_password_hash: String,
    ) -> Result<(), UserRepositoryError>;

    async fn delete_user(&self, user_id: Uuid) -> Result<(), UserRepositoryError>;
    async fn soft_delete_user(&self, user_id: Uuid) -> Result<(), UserRepositoryError>;
    async fn restore_user(&self, user_id: Uuid) -> Result<User, UserRepositoryError>;
    async fn activate_user(&self, user_id: Uuid) -> Result<(), UserRepositoryError>;
}

#[derive(Debug)]
pub enum UserRepositoryError {
    UserAlreadyExists,
    UserNotFound,
    DatabaseError(String),
}

#[cfg(not(tarpaulin_include))]
impl fmt::Display for UserRepositoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UserRepositoryError::UserNotFound => write!(f, "User not found"),
            UserRepositoryError::UserAlreadyExists => write!(f, "User already exists"),
            UserRepositoryError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
        }
    }
}
