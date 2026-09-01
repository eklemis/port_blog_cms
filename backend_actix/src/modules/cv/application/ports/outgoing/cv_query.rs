//! Read-side port for CVs: listings and single fetches, owner-facing and public.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::cv::domain::entities::CVInfo;
use utoipa::ToSchema;

/// Narrows a CV listing.
#[derive(Debug, Clone, Default)]
pub struct CVListFilter {
    pub search: Option<String>,
}

/// Listing order. Defaults to [`UpdatedNewest`](Self::UpdatedNewest).
#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum CVSort {
    /// Sort by creation date (newest first)
    Newest,
    /// Sort by creation date (oldest first)
    Oldest,
    /// Sort by last update (newest first)
    UpdatedNewest,
    /// Sort by last update (oldest first)
    UpdatedOldest,
}

impl Default for CVSort {
    fn default() -> Self {
        CVSort::UpdatedNewest
    }
}

/// Which page to return. Pages are 1-based; defaults to 20 per page.
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

/// One page of results, plus the totals a client needs to paginate.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CVPageResult<T> {
    /// List of items in the current page
    pub items: Vec<T>,

    /// Current page number
    #[schema(example = 1)]
    pub page: u32,

    /// Number of items per page
    #[schema(example = 10)]
    pub per_page: u32,

    /// Total number of items across all pages
    #[schema(example = 42)]
    pub total: u64,
}

//
// ──────────────────────────────────────────────────────────
// Errors
// ──────────────────────────────────────────────────────────
//

/// Why a CV read failed.
///
/// No "not found" variant: absence is `Ok(None)` or an empty page.
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

/// Reads CVs: owner listings and public single fetches.
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
