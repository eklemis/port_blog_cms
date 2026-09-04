//! This module's use cases, grouped for `AppState`.

use std::sync::Arc;

use crate::ai::application::ports::incoming::use_cases::{
    ConsumeAiQuotaUseCase, GetAiQuotaUseCase,
};

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
}
