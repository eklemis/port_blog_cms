//! What the route layer may ask of the AI surfaces.

use async_trait::async_trait;

use crate::ai::application::ports::outgoing::UsageCounterError;
use crate::ai::domain::quota::QuotaState;
use crate::auth::application::domain::entities::UserId;

/// Why a quota operation failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum QuotaError {
    /// The caller has used their allowance for this period.
    #[error("Generation limit reached for this period")]
    Exceeded(Box<QuotaState>),

    /// The counter could not be reached.
    #[error("Quota unavailable: {0}")]
    Unavailable(String),
}

impl From<UsageCounterError> for QuotaError {
    fn from(e: UsageCounterError) -> Self {
        match e {
            UsageCounterError::Unavailable(m) => QuotaError::Unavailable(m),
        }
    }
}

/// Reports a person's standing without spending anything.
#[async_trait]
pub trait GetAiQuotaUseCase: Send + Sync {
    /// The current state. Never refuses on account of the limit — reading
    /// where you stand must work when you have nothing left.
    async fn execute(&self, owner: UserId) -> Result<QuotaState, QuotaError>;
}

/// Spends one generation, refusing when there is nothing left.
///
/// Every AI surface calls this **before** doing any work. It exists now, ahead
/// of the surfaces themselves, because a limit retrofitted onto screens built
/// assuming calls are free is the expensive order to do this in.
#[async_trait]
pub trait ConsumeAiQuotaUseCase: Send + Sync {
    /// Records one generation, or reports the state that refused it.
    async fn execute(&self, owner: UserId) -> Result<QuotaState, QuotaError>;
}
