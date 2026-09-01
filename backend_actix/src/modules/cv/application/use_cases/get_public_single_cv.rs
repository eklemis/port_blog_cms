use async_trait::async_trait;
use std::fmt;
use uuid::Uuid;

use crate::cv::domain::entities::CVInfo;

//
// ──────────────────────────────────────────────────────────
// Errors
// ──────────────────────────────────────────────────────────
//

/// Why a public CV fetch failed.
#[derive(Debug, Clone)]
pub enum GetPublicSingleCvError {
    /// No CV matched, or it is not publicly visible. The two are reported the
    /// same way so a private CV cannot be probed for.
    NotFound,
    /// The store could not be reached, or the write failed.
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

/// Fetches one CV for a public reader.
#[async_trait]
pub trait GetPublicSingleCvUseCase: Send + Sync {
    /// Returns the CV if it is publicly visible.
    async fn execute(&self, owner_id: Uuid, cv_id: Uuid) -> Result<CVInfo, GetPublicSingleCvError>;
}
