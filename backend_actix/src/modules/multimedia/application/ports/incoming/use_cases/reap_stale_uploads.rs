//! Removing registrations whose bytes never arrived.
//!
//! A media row is written when a client asks for an upload URL, before any
//! bytes exist. Only the bucket can move it past `pending`, so if the client
//! never uploads, the row stays that way permanently — and an attachment
//! pointing at it serves an empty `variants` map, which is indistinguishable
//! from one whose variants are still being generated. A client told to poll
//! polls forever.
//!
//! This is the sweep that ends that wait. It is driven from outside — a
//! scheduler calling the admin route — rather than from a timer inside the
//! process, because several instances run at once on Cloud Run and each would
//! otherwise sweep on its own clock.

use async_trait::async_trait;

/// How long a registration may sit in `pending` before it is considered dead.
///
/// The floor is the signed upload URL's own lifetime: before that elapses the
/// client can still legitimately upload. The margin on top covers the gap
/// between the bytes landing and the out-of-band processor claiming the row,
/// during which it is still `pending` through no fault of the uploader.
///
/// Deleting a row whose upload has only just landed would take a file the
/// person successfully uploaded, so this errs long. An hour of stale rows
/// costs nothing; deleting one live upload is a lost file.
pub const DEFAULT_STALE_AFTER_SECS: u64 = 60 * 60;

/// The floor the configured window is held to.
///
/// Anything under the signed URL's lifetime would race uploads that are still
/// valid, so a smaller value is raised to this rather than honoured.
pub const MIN_STALE_AFTER_SECS: u64 = 15 * 60;

/// What a sweep did.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
pub struct ReapOutcome {
    /// How many registrations were removed.
    pub deleted: u64,

    /// The window actually applied, after the floor was enforced.
    ///
    /// Reported so a caller can see that a misconfigured window was corrected
    /// rather than silently obeyed.
    pub older_than_secs: u64,
}

/// Why a sweep could not run.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ReapError {
    /// The store failed.
    #[error("{0}")]
    RepositoryError(String),
}

/// Deletes registrations whose bytes never arrived.
#[async_trait]
pub trait ReapStaleUploadsUseCase: Send + Sync {
    /// Runs one sweep.
    ///
    /// Idempotent: a second call finds nothing and reports zero, which matters
    /// because a scheduler retries.
    async fn execute(&self, older_than_secs: Option<u64>) -> Result<ReapOutcome, ReapError>;
}
