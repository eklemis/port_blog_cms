use crate::modules::auth::application::domain::entities::User;
use async_trait::async_trait;
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
}

#[derive(Debug)]
pub enum UserRepositoryError {
    UserAlreadyExists,
    UserNotFound,
    DatabaseError(String),
}
