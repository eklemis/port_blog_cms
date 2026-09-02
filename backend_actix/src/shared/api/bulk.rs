//! The shape every "apply one operation to many items" endpoint returns.
//!
//! A bulk call is not one operation on a set — it is many operations that each
//! succeed or fail on their own. Reporting a single status for the batch would
//! force a client to re-fetch everything to find out what actually happened, so
//! these endpoints answer **200 with a per-item outcome** whenever the request
//! itself was well formed, and reserve 4xx for a request that was not.
//!
//! The corollary matters: `success: true` on a bulk response means the batch
//! was processed, **not** that every item succeeded. Clients must read
//! [`BulkOutcome::failed`].

use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::shared::api::ErrorCode;

/// Most ids one bulk request may carry.
///
/// Bounded because each id is a separate database write: an unbounded list is
/// an unbounded transaction and a request that cannot be timed out sensibly.
/// A console selecting more than this should page its calls.
pub const MAX_BULK_IDS: usize = 100;

/// What happened to each item in a bulk request.
#[derive(Debug, Clone, Default, Serialize, ToSchema)]
pub struct BulkOutcome {
    /// Ids the operation applied to, in request order.
    pub succeeded: Vec<Uuid>,

    /// Ids the operation did not apply to, each with the reason.
    ///
    /// Empty on a fully successful batch. **Never assume it is** — a partial
    /// failure is the ordinary case here, not an exceptional one.
    pub failed: Vec<BulkFailure>,
}

/// One item that did not succeed, and why.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BulkFailure {
    /// The item this is about.
    pub id: Uuid,

    /// The same vocabulary `error.code` uses, so a client branches on bulk
    /// failures exactly as it branches on single-item ones.
    #[schema(value_type = String, example = "POST_NOT_FOUND")]
    pub code: ErrorCode,

    /// Prose detail. May change; branch on `code`.
    pub message: String,
}

impl BulkOutcome {
    /// Records an item the operation applied to.
    pub fn succeed(&mut self, id: Uuid) {
        self.succeeded.push(id);
    }

    /// Records an item it did not.
    pub fn fail(&mut self, id: Uuid, code: ErrorCode, message: impl Into<String>) {
        self.failed.push(BulkFailure {
            id,
            code,
            message: message.into(),
        });
    }

    /// True when nothing failed.
    pub fn is_complete(&self) -> bool {
        self.failed.is_empty()
    }
}

/// Why a bulk request was rejected before any item was touched.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BulkRequestError {
    /// The id list was empty.
    #[error("A bulk request must carry at least one id")]
    Empty,

    /// The id list was longer than [`MAX_BULK_IDS`].
    #[error("A bulk request carries at most {MAX_BULK_IDS} ids, got {0}")]
    TooLarge(usize),
}

impl BulkRequestError {
    /// The code this maps to in an error response.
    pub fn code(&self) -> ErrorCode {
        match self {
            BulkRequestError::Empty => ErrorCode::BulkEmpty,
            BulkRequestError::TooLarge(_) => ErrorCode::BulkTooLarge,
        }
    }
}

/// Checks the id list and removes duplicates, preserving request order.
///
/// Duplicates are dropped rather than rejected: a console that selects the same
/// row twice means it once. Left in, the second attempt would report a
/// not-found for an item the first call had just hard-deleted, which reads as a
/// failure when nothing went wrong.
pub fn prepare_ids(ids: Vec<Uuid>) -> Result<Vec<Uuid>, BulkRequestError> {
    if ids.is_empty() {
        return Err(BulkRequestError::Empty);
    }
    if ids.len() > MAX_BULK_IDS {
        return Err(BulkRequestError::TooLarge(ids.len()));
    }

    let mut seen = std::collections::HashSet::with_capacity(ids.len());
    Ok(ids.into_iter().filter(|id| seen.insert(*id)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicates_collapse_and_order_survives() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();

        assert_eq!(prepare_ids(vec![a, b, a]).unwrap(), vec![a, b]);
    }

    #[test]
    fn an_empty_batch_is_rejected() {
        assert_eq!(prepare_ids(vec![]), Err(BulkRequestError::Empty));
        assert_eq!(BulkRequestError::Empty.code(), ErrorCode::BulkEmpty);
    }

    #[test]
    fn a_batch_over_the_cap_is_rejected_before_anything_is_touched() {
        let ids: Vec<Uuid> = (0..MAX_BULK_IDS + 1).map(|_| Uuid::new_v4()).collect();
        let n = ids.len();

        assert_eq!(prepare_ids(ids), Err(BulkRequestError::TooLarge(n)));
    }

    #[test]
    fn a_batch_exactly_at_the_cap_is_allowed() {
        let ids: Vec<Uuid> = (0..MAX_BULK_IDS).map(|_| Uuid::new_v4()).collect();

        assert_eq!(prepare_ids(ids).unwrap().len(), MAX_BULK_IDS);
    }

    /// The code is the contract, so it must serialise as the wire string
    /// rather than the Rust variant name.
    #[test]
    fn a_failure_carries_the_wire_code() {
        let mut outcome = BulkOutcome::default();
        let id = Uuid::new_v4();
        outcome.fail(id, ErrorCode::PostNotFound, "Blog post not found");

        let json = serde_json::to_value(&outcome).unwrap();
        assert_eq!(json["failed"][0]["code"], "POST_NOT_FOUND");
        assert!(!outcome.is_complete());
    }
}
