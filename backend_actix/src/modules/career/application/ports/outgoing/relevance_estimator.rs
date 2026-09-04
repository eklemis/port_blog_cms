//! Asking a model how well a CV answers a job, without `career` knowing that a
//! model is what answers.

use async_trait::async_trait;
use uuid::Uuid;

use crate::career::domain::relevance::RequirementMatch;

/// Why an estimate could not be produced.
///
/// Each variant is a thing worth telling a person, which is why this is not
/// one opaque string: "you have used your allowance" and "generation is
/// switched off here" call for different words on screen.
#[derive(Debug, Clone, thiserror::Error)]
pub enum RelevanceEstimatorError {
    /// No provider is configured on this deployment.
    #[error("Generation is not configured on this deployment")]
    Disabled,

    /// The caller has used their generation allowance.
    #[error("Generation limit reached for this period")]
    QuotaExceeded,

    /// The model declined.
    #[error("The model declined to answer")]
    Refused,

    /// Anything else — unreachable provider, unreadable reply.
    #[error("{0}")]
    Failed(String),
}

/// Estimates how well a CV answers a posting.
///
/// Returns the **requirements only**. The score is computed by
/// [`RelevanceReport::from_requirements`](crate::career::domain::relevance::RelevanceReport::from_requirements)
/// from those verdicts, so the model is never asked for a number that could
/// disagree with its own rows.
#[async_trait]
pub trait RelevanceEstimator: Send + Sync {
    /// One requirement per thing the posting asks for.
    async fn estimate(
        &self,
        owner: Uuid,
        cv: &str,
        job: &str,
    ) -> Result<Vec<RequirementMatch>, RelevanceEstimatorError>;
}
