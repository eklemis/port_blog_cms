// application/ports/outgoing/user_query.rs
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Result DTO for user queries
/// Contains all user data needed for read operations
#[derive(Debug, Clone)]
pub struct UserQueryResult {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub password_hash: String,
    pub full_name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_verified: bool,
    pub is_deleted: bool,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum UserQueryError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Query execution failed: {0}")]
    QueryFailed(String),
}

#[async_trait]
pub trait UserQuery: Send + Sync {
    async fn find_by_id(&self, user_id: Uuid) -> Result<Option<UserQueryResult>, UserQueryError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<UserQueryResult>, UserQueryError>;
    async fn find_by_username(
        &self,
        username: &str,
    ) -> Result<Option<UserQueryResult>, UserQueryError>;
}
