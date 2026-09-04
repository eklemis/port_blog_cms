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

/// Job fields pulled out of a posting.
///
/// Every field is optional in practice — a posting that omits the seniority
/// leaves it empty rather than making something up — which is why the schema
/// asks for empty strings and lists rather than nulls.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct ExtractedJob {
    /// Role title.
    pub title: String,
    /// Hiring company.
    pub company: String,
    /// Where the role is.
    pub location: String,
    /// Seniority as advertised.
    pub seniority: String,
    /// Must-haves.
    pub required_skills: Vec<String>,
    /// Nice-to-haves.
    pub nice_to_have: Vec<String>,
}

/// Why a generation did not happen.
///
/// Distinct from the port's `GenerationError` because two of these never reach
/// a provider at all — the allowance and the missing configuration — and a
/// client needs to tell "you have run out" from "the model refused".
#[derive(Debug, Clone, thiserror::Error)]
pub enum AiError {
    /// No provider is configured on this deployment.
    #[error("Generation is not configured on this deployment")]
    Disabled,

    /// The caller has used their allowance.
    #[error("Generation limit reached for this period")]
    QuotaExceeded(Box<QuotaState>),

    /// The posting could not be fetched from its URL.
    #[error("{0}")]
    FetchFailed(String),

    /// The model declined.
    #[error("The model declined this request: {0}")]
    Refused(String),

    /// The provider failed or was unreachable.
    #[error("Provider error: {0}")]
    Upstream(String),

    /// The provider took too long.
    #[error("The provider timed out")]
    Timeout,

    /// The request did not carry enough to work with.
    #[error("{0}")]
    Invalid(String),
}

impl From<QuotaError> for AiError {
    fn from(e: QuotaError) -> Self {
        match e {
            QuotaError::Exceeded(state) => AiError::QuotaExceeded(state),
            QuotaError::Unavailable(m) => AiError::Upstream(m),
        }
    }
}

impl From<crate::ai::application::ports::outgoing::GenerationError> for AiError {
    fn from(e: crate::ai::application::ports::outgoing::GenerationError) -> Self {
        use crate::ai::application::ports::outgoing::GenerationError as E;
        match e {
            E::Refused(m) => AiError::Refused(m),
            E::Timeout => AiError::Timeout,
            E::RateLimited(m) | E::Upstream(m) | E::Malformed(m) => AiError::Upstream(m),
        }
    }
}

/// What to read a job posting from.
#[derive(Debug, Clone, Default)]
pub struct ExtractJobInput {
    /// The posting, pasted. The primary path.
    pub text: Option<String>,
    /// A link to it. Usually fails — most boards block automated fetches — so
    /// the caller is expected to fall back to pasting.
    pub url: Option<String>,
}

/// Reads a posting into typed fields.
#[async_trait]
pub trait ExtractJobUseCase: Send + Sync {
    /// Extracts, spending one generation.
    async fn execute(&self, owner: UserId, input: ExtractJobInput)
        -> Result<ExtractedJob, AiError>;
}

/// What a tailoring or letter-writing pass works from.
#[derive(Debug, Clone, Default)]
pub struct DraftingInput {
    /// The application being worked on.
    pub application_id: uuid::Uuid,
    /// A living CV to work from. Falls back to the application's snapshot.
    pub cv_id: Option<uuid::Uuid>,
    /// What to do this turn, in the author's words. Optional; each surface has
    /// a sensible default instruction.
    pub instruction: Option<String>,
    /// The language to write in.
    ///
    /// Explicit, never inferred — a half-written document is not evidence, and
    /// a CV that mixes languages is normal.
    pub language: Option<String>,
}

/// Suggests changes to a CV for one job.
#[async_trait]
pub trait TailorUseCase: Send + Sync {
    /// Streams suggestions, spending one generation.
    async fn execute(
        &self,
        owner: UserId,
        input: DraftingInput,
    ) -> Result<crate::ai::application::ports::outgoing::GenerationStream, AiError>;
}

/// Drafts a cover letter for one application.
#[async_trait]
pub trait CoverLetterDraftUseCase: Send + Sync {
    /// Streams the letter, spending one generation.
    async fn execute(
        &self,
        owner: UserId,
        input: DraftingInput,
    ) -> Result<crate::ai::application::ports::outgoing::GenerationStream, AiError>;
}
