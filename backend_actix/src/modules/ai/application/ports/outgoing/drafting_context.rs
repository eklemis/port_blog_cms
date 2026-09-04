//! The material a drafting pass works from, without `ai` learning where any of
//! it is stored.

use async_trait::async_trait;
use uuid::Uuid;

/// Everything a tailoring or letter-writing pass reads.
///
/// Rendered to text here rather than passed as structs, because what goes into
/// a prompt should be decided by the module that owns the data — not assembled
/// from field names by the module that happens to be calling a model.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DraftingMaterial {
    /// The CV, as prose.
    pub cv: String,
    /// The posting, verbatim where one was kept.
    pub job: String,
    /// The letter as it currently stands, when there is one to revise.
    pub existing_letter: Option<String>,
}

/// Why the material could not be gathered.
#[derive(Debug, Clone, thiserror::Error)]
pub enum DraftingContextError {
    /// No application matched, or it belongs to another user.
    #[error("Application not found")]
    NotFound,

    /// The application is a draft with no CV named and no snapshot.
    #[error("Nothing to work from: send cv_id, or send the application first")]
    NoCv,

    /// A store could not be reached.
    #[error("Could not gather the material: {0}")]
    Failed(String),
}

/// Gathers what a drafting pass needs.
#[async_trait]
pub trait DraftingContextReader: Send + Sync {
    /// Loads the CV, the posting and any existing letter.
    async fn load(
        &self,
        owner: Uuid,
        application_id: Uuid,
        cv_id: Option<Uuid>,
    ) -> Result<DraftingMaterial, DraftingContextError>;
}

/// Fetches a job posting from a URL.
///
/// Its own port because it fails most of the time and that is expected: most
/// boards block automated fetches or sit behind a login. Keeping it separate
/// means the failure is a clean, specific answer rather than a retry loop, and
/// the caller is one paste away from succeeding.
#[async_trait]
pub trait PostingFetcher: Send + Sync {
    /// The posting's text, or why it could not be had.
    async fn fetch(&self, url: &str) -> Result<String, String>;
}
