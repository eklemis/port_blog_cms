// cv_repository.rs
use crate::cv::domain::entities::CVInfo;
use async_trait::async_trait;
use uuid::Uuid;

#[derive(Debug)]
pub enum CVRepositoryError {
    NotFound,
    DatabaseError(String),
}

#[async_trait]
pub trait CVRepository: Send + Sync {
    async fn fetch_cv_by_user_id(&self, user_id: Uuid) -> Result<Vec<CVInfo>, CVRepositoryError>;
    async fn create_cv(&self, user_id: Uuid, cv_data: CVInfo) -> Result<CVInfo, CVRepositoryError>;
    async fn update_cv(&self, user_id: Uuid, cv_data: CVInfo) -> Result<CVInfo, CVRepositoryError>;
}
