use async_trait::async_trait;

use crate::auth::application::{
    domain::entities::UserId,
    ports::outgoing::{user_query::UserQueryError, UserRepositoryError},
};

#[derive(Clone, Debug)]
pub struct UpdateUserOutput {
    pub user_id: UserId,
    pub username: String,
    pub email: String,
    pub full_name: String,
}

#[derive(Clone, Debug)]
pub struct UpdateUserInput {
    pub user_id: UserId,
    pub full_name: String,
}

#[derive(Debug, thiserror::Error, Clone)]
pub enum UpdateUserError {
    #[error("Invalid full name: {0}")]
    InvalidFullName(String),

    #[error("Repository error: {0}")]
    RepositoryError(#[from] UserRepositoryError),

    #[error("Query error: {0}")]
    QueryError(#[from] UserQueryError),
}

#[async_trait]
pub trait UpdateUserProfileUseCase: Send + Sync {
    async fn execute(&self, data: UpdateUserInput) -> Result<UpdateUserOutput, UpdateUserError>;
}
