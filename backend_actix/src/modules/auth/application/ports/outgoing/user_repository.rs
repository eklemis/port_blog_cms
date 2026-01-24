// application/ports/outgoing/user_repository.rs
use async_trait::async_trait;
use uuid::Uuid;

// Input DTO for creating a user
#[derive(Debug, Clone)]
pub struct CreateUserData {
    pub email: String,
    pub username: String,
    pub password_hash: String,
    pub full_name: String,
}

// Unified output DTO for all user operations that return user data
// This represents the essential user information after any state change
#[derive(Debug, Clone)]
pub struct UserResult {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub full_name: String,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum UserRepositoryError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("User not found")]
    UserNotFound,

    #[error("User already exists")]
    UserAlreadyExists,
}

#[async_trait]
pub trait UserRepository: Send + Sync {
    // Operations that confirm the new state by returning user data
    async fn create_user(&self, data: CreateUserData) -> Result<UserResult, UserRepositoryError>;
    async fn restore_user(&self, user_id: Uuid) -> Result<UserResult, UserRepositoryError>;
    async fn activate_user(&self, user_id: Uuid) -> Result<UserResult, UserRepositoryError>;
    async fn set_full_name(
        &self,
        user_id: Uuid,
        full_name: String,
    ) -> Result<UserResult, UserRepositoryError>;

    // Operations that don't need to return user data (pure commands)
    async fn update_password(
        &self,
        user_id: Uuid,
        new_password_hash: String,
    ) -> Result<(), UserRepositoryError>;
    async fn delete_user(&self, user_id: Uuid) -> Result<(), UserRepositoryError>;
    async fn soft_delete_user(&self, user_id: Uuid) -> Result<(), UserRepositoryError>;
}
