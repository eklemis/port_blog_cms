//! This module's use cases, grouped for `AppState`.

use std::sync::Arc;

use crate::ai::application::ports::incoming::use_cases::{
    ConsumeAiQuotaUseCase, CoverLetterDraftUseCase, ExtractJobUseCase, GetAiQuotaUseCase,
    TailorUseCase,
};
use crate::ai::application::ports::outgoing::TextGenerator;

/// Bundles the AI use cases so `AppState` gains one field.
#[derive(Clone)]
pub struct AiUseCases {
    /// The [`GetAiQuotaUseCase`] implementation.
    pub get_quota: Arc<dyn GetAiQuotaUseCase + Send + Sync>,
    /// The [`ConsumeAiQuotaUseCase`] implementation.
    ///
    /// Held here ready for the generation surfaces: every one of them spends
    /// before it works.
    pub consume_quota: Arc<dyn ConsumeAiQuotaUseCase + Send + Sync>,

    /// Whichever vendor was configured, or `None` when no key is set.
    ///
    /// `None` is a running deployment with generation switched off, not a
    /// broken one: an optional feature without credentials must not stop the
    /// rest of the API from starting.
    pub generator: Option<Arc<dyn TextGenerator>>,

    /// The [`ExtractJobUseCase`] implementation.
    pub extract_job: Arc<dyn ExtractJobUseCase + Send + Sync>,
    /// The [`TailorUseCase`] implementation.
    pub tailor: Arc<dyn TailorUseCase + Send + Sync>,
    /// The [`CoverLetterDraftUseCase`] implementation.
    pub cover_letter: Arc<dyn CoverLetterDraftUseCase + Send + Sync>,
}
