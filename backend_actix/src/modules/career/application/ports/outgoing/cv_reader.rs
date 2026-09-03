//! Reading a CV to analyse it, without `career` learning how CVs are stored.

use async_trait::async_trait;
use uuid::Uuid;

use crate::cv::domain::entities::CVInfo;

/// Why a CV could not be read.
#[derive(Debug, Clone, thiserror::Error)]
pub enum CvReaderError {
    /// The store could not be reached.
    #[error("Read failed: {0}")]
    Failed(String),
}

/// Fetches a CV, living or frozen.
///
/// Both shapes are needed because analysis happens on both sides of sending:
/// while tailoring, the CV is still being edited; afterwards, the only honest
/// thing to analyse is the snapshot that actually went out.
#[async_trait]
pub trait CvReader: Send + Sync {
    /// A CV the caller owns, as it stands now.
    async fn read_cv(&self, owner: Uuid, cv_id: Uuid) -> Result<Option<CVInfo>, CvReaderError>;

    /// A frozen snapshot the caller owns.
    async fn read_snapshot(
        &self,
        owner: Uuid,
        snapshot_id: Uuid,
    ) -> Result<Option<CVInfo>, CvReaderError>;
}
