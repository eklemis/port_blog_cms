//! The draft-preview use cases, grouped for `AppState`.

use std::sync::Arc;

use crate::blog::application::ports::incoming::use_cases::{
    GetDraftPreviewUseCase, ReadDraftPreviewUseCase, ReadPreviewMediaUseCase,
    RevokeDraftPreviewUseCase, ShareDraftUseCase,
};

/// Bundles the four preview use cases so `AppState` gains one field.
#[derive(Clone)]
pub struct BlogPreviewUseCases {
    /// The [`ShareDraftUseCase`] implementation.
    pub share: Arc<dyn ShareDraftUseCase + Send + Sync>,
    /// The [`GetDraftPreviewUseCase`] implementation.
    pub get: Arc<dyn GetDraftPreviewUseCase + Send + Sync>,
    /// The [`RevokeDraftPreviewUseCase`] implementation.
    pub revoke: Arc<dyn RevokeDraftPreviewUseCase + Send + Sync>,
    /// The [`ReadDraftPreviewUseCase`] implementation.
    pub read: Arc<dyn ReadDraftPreviewUseCase + Send + Sync>,
    /// The [`ReadPreviewMediaUseCase`] implementation.
    pub read_media: Arc<dyn ReadPreviewMediaUseCase + Send + Sync>,
}
