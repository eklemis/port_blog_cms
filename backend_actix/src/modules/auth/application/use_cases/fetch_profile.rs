use async_trait::async_trait;

use crate::auth::application::{
    domain::entities::UserId, ports::outgoing::user_query::UserQueryError,
};

#[derive(Clone, Debug)]
pub struct FetchUserOutput {
    pub user_id: UserId,
    pub email: String,
    pub username: String,
    pub full_name: String,
}

#[derive(Debug, thiserror::Error, Clone)]
pub enum FetchUserError {
    #[error("User not found: {0}")]
    UserNotFound(String),

    #[error("Query error: {0}")]
    QueryError(#[from] UserQueryError),
}

#[async_trait]
pub trait FetchUserProfileUseCase: Send + Sync {
    async fn execute(&self, user_id: UserId) -> Result<FetchUserOutput, FetchUserError>;
}
