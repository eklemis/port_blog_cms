use async_trait::async_trait;
use std::fmt;
use uuid::Uuid;

use crate::cv::domain::entities::CVInfo;

//
// ──────────────────────────────────────────────────────────
// Errors
// ──────────────────────────────────────────────────────────
//

#[derive(Debug, Clone)]
pub enum GetPublicSingleCvError {
    NotFound,
    RepositoryError(String),
}

impl fmt::Display for GetPublicSingleCvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GetPublicSingleCvError::NotFound => write!(f, "cv not found"),
            GetPublicSingleCvError::RepositoryError(msg) => write!(f, "repository error: {}", msg),
        }
    }
}

//
// ──────────────────────────────────────────────────────────
// Use case trait
// ──────────────────────────────────────────────────────────
//

#[async_trait]
pub trait GetPublicSingleCvUseCase: Send + Sync {
    async fn execute(&self, owner_id: Uuid, cv_id: Uuid) -> Result<CVInfo, GetPublicSingleCvError>;
}
