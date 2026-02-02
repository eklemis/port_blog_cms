use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::cv::domain::entities::CVInfo;

//
// ──────────────────────────────────────────────────────────
// Query DTOs
// ──────────────────────────────────────────────────────────
//

#[derive(Debug, Clone, Default)]
pub struct CVListFilter {
    pub search: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub enum CVSort {
    Newest,
    Oldest,
    UpdatedNewest,
    UpdatedOldest,
}

impl Default for CVSort {
    fn default() -> Self {
        CVSort::UpdatedNewest
    }
}

#[derive(Debug, Clone)]
pub struct CVPageRequest {
    pub page: u32,
    pub per_page: u32,
}

impl Default for CVPageRequest {
    fn default() -> Self {
        Self {
            page: 1,
            per_page: 20,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CVPageResult<T> {
    pub items: Vec<T>,
    pub page: u32,
    pub per_page: u32,
    pub total: u64,
}

//
// ──────────────────────────────────────────────────────────
// Errors
// ──────────────────────────────────────────────────────────
//

#[derive(Debug, Clone, thiserror::Error)]
pub enum CVQueryError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Query execution failed: {0}")]
    QueryFailed(String),
}

//
// ──────────────────────────────────────────────────────────
// Port
// ──────────────────────────────────────────────────────────
//

#[async_trait]
pub trait CVQuery: Send + Sync {
    async fn list(
        &self,
        user_id: Uuid,
        filter: CVListFilter,
        sort: CVSort,
        page: CVPageRequest,
    ) -> Result<CVPageResult<CVInfo>, CVQueryError>;

    async fn fetch_cv_by_id(&self, cv_id: Uuid) -> Result<Option<CVInfo>, CVQueryError>;
}
