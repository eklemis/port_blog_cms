//! Asking a model for text, without naming which model.
//!
//! Everything here is in this product's vocabulary rather than any vendor's.
//! That is the point: the adapter behind it can be Anthropic or OpenAI, chosen
//! by configuration, and nothing above this line changes when it is swapped.
//!
//! # What the abstraction can and cannot hide
//!
//! Most of it hides cleanly — a prompt is a prompt and a completion is a
//! completion. Two things do not, and are modelled rather than glossed:
//!
//! - **Caching the stable prefix** is the largest cost lever in this feature,
//!   and the vendors do it differently: one takes explicit breakpoints, the
//!   other caches long prefixes on its own. So this port asks the caller to
//!   *separate* stable context from the volatile instruction, and leaves each
//!   adapter to exploit that separation however its vendor allows. A caller
//!   that lumps them together silently loses the saving on both.
//! - **A refusal is not an error.** At least one vendor returns HTTP 200 with a
//!   marker rather than a failure status, so [`GenerationError::Refused`] is a
//!   distinct variant and adapters must map to it explicitly. Treating
//!   refusals as successes would surface an empty generation to a person.

use async_trait::async_trait;

/// A request for generated text.
///
/// The three text fields are separate so an adapter can cache what is stable
/// and re-send only what changed. Ordering is fixed: `system`, then `context`,
/// then `instruction` — a cache is a prefix match, so shuffling them would
/// throw the saving away.
#[derive(Debug, Clone, Default)]
pub struct GenerationRequest {
    /// Standing instructions. Identical across a whole feature's calls.
    pub system: String,

    /// The expensive, stable material — a CV, a job description.
    ///
    /// **This is the part worth caching**, and the reason it is its own field.
    pub context: String,

    /// What to do this turn. Changes on every call, so it goes last.
    pub instruction: String,

    /// Ceiling on the reply.
    pub max_output_tokens: u32,

    /// A JSON Schema the reply must satisfy.
    ///
    /// Set it wherever the result populates a form: a malformed generation
    /// then fails loudly instead of being regexed out of prose. Both vendors
    /// support this, by different names, and neither supports recursive
    /// schemas or numeric bounds.
    pub schema: Option<serde_json::Value>,
}

/// What the model produced, and what it cost.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Generation {
    /// The text, or the JSON document when a schema was set.
    pub text: String,
    /// What it cost, as far as the vendor reported.
    pub usage: Usage,
}

/// Token counts, as reported by whichever vendor answered.
///
/// Kept because the cached figure is the only way to tell whether prefix
/// caching is actually working. A caching strategy nobody measures is a
/// caching strategy that has quietly stopped.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Usage {
    /// Tokens sent, excluding anything served from cache.
    pub input_tokens: u32,
    /// Tokens generated.
    pub output_tokens: u32,
    /// Tokens that came from the vendor's cache rather than being reprocessed.
    ///
    /// Zero is a warning sign on a repeated call, not a neutral value.
    pub cached_input_tokens: u32,
}

/// Why a generation did not produce text.
#[derive(Debug, Clone, thiserror::Error)]
pub enum GenerationError {
    /// The model declined.
    ///
    /// **Not a transport failure.** At least one vendor reports this as a
    /// successful response carrying a marker, so an adapter that maps only on
    /// HTTP status will never produce this and will hand back an empty string
    /// instead.
    #[error("The model declined this request: {0}")]
    Refused(String),

    /// The vendor rate-limited or ran out of capacity. Worth retrying.
    #[error("The provider is rate limiting: {0}")]
    RateLimited(String),

    /// The request took too long.
    #[error("The provider timed out")]
    Timeout,

    /// Anything else from the vendor — a bad status, an unreachable host.
    #[error("Provider error: {0}")]
    Upstream(String),

    /// The reply arrived but could not be read, or did not match the schema.
    #[error("The provider's reply could not be read: {0}")]
    Malformed(String),
}

/// Produces text from a prompt.
///
/// One method on purpose. Streaming is a separate concern and will be a
/// separate method when the surfaces that need it exist; adding it here now
/// would fix a shape before there is a caller to fix it against.
#[async_trait]
pub trait TextGenerator: Send + Sync {
    /// Which vendor is answering, for logs and the health probe.
    ///
    /// Not for branching on. A caller that behaves differently per vendor has
    /// defeated the point of the port.
    fn provider(&self) -> &'static str;

    /// Generates once.
    async fn generate(&self, request: GenerationRequest) -> Result<Generation, GenerationError>;
}
