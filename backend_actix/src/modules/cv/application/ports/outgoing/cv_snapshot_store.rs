//! Storage for immutable CV snapshots.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::cv::domain::entities::CVInfo;

/// A CV frozen at the moment an application was sent.
#[derive(Debug, Clone)]
pub struct CvSnapshot {
    /// The snapshot's own identifier. What an application points at.
    pub id: Uuid,

    /// The CV this was taken from. The living CV may since have changed —
    /// that is the entire point of storing this separately.
    pub cv_id: Uuid,

    /// The owner.
    pub user_id: Uuid,

    /// The CV exactly as it stood.
    pub document: CVInfo,

    /// When it was taken. Doubles as the "as sent" date the tracker shows.
    pub created_at: DateTime<Utc>,
}

/// Why a snapshot read or write failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum CvSnapshotStoreError {
    /// No CV matched, or it belongs to another user.
    #[error("CV not found")]
    CvNotFound,

    /// No snapshot matched that id.
    #[error("Snapshot not found")]
    SnapshotNotFound,

    /// The stored document could not be read back into a CV.
    #[error("Stored snapshot is unreadable: {0}")]
    Corrupt(String),

    /// The store could not be reached.
    #[error("Database error: {0}")]
    DatabaseError(String),
}

/// Writes and reads snapshots.
///
/// There is deliberately no update and no delete. A snapshot that could be
/// edited would not answer the question it exists to answer, and one that
/// could be removed would leave an application pointing at nothing — which is
/// why the foreign key restricts deletion rather than cascading it.
#[async_trait]
pub trait CvSnapshotStore: Send + Sync {
    /// Freezes a CV, returning the new snapshot.
    ///
    /// Owner-scoped: a CV belonging to someone else is `CvNotFound`.
    async fn create(&self, owner: Uuid, cv_id: Uuid) -> Result<CvSnapshot, CvSnapshotStoreError>;

    /// Reads one snapshot back.
    ///
    /// Owner-scoped. A snapshot is a record of what *you* sent, so it is not
    /// public — the CV it was taken from may be, but the two are different
    /// documents by the time anyone asks.
    async fn find(
        &self,
        owner: Uuid,
        snapshot_id: Uuid,
    ) -> Result<Option<CvSnapshot>, CvSnapshotStoreError>;
}
