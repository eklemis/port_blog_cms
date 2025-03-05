use crate::modules::auth::application::domain::entities::User;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait UserQuery {
    async fn find_by_id(&self, user_id: Uuid) -> Result<Option<User>, String>;
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, String>;
    async fn find_by_username(&self, username: &str) -> Result<Option<User>, String>;
}
