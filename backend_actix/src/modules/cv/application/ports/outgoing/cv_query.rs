// cv_query.rs
use crate::cv::domain::entities::CVInfo;
use async_trait::async_trait;
use uuid::Uuid;

#[derive(Debug, Clone, thiserror::Error)]
pub enum CVQueryError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Query execution failed: {0}")]
    QueryFailed(String),
}

#[async_trait]
pub trait CVQuery: Send + Sync {
    async fn fetch_cv_by_user_id(&self, user_id: Uuid) -> Result<Vec<CVInfo>, CVQueryError>;
    async fn fetch_cv_by_id(&self, cv_id: Uuid) -> Result<Option<CVInfo>, CVQueryError>;
}
