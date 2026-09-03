use std::sync::Arc;

use crate::multimedia::application::ports::incoming::use_cases::{
    BulkMediaUseCase, CreateUploadMediaUrlUseCase, DeleteMediaUseCase, GetMediaStatusesUseCase,
    GetMediaUsageUseCase, GetMediaUseCase, GetPublicVariantUrlUseCase, GetVariantReadUrlUseCase,
    HardDeleteMediaUseCase, ListMediaUseCase, PatchMediaUseCase, RestoreMediaUseCase,
};

/// This module's use cases, grouped for `AppState`.
#[derive(Clone)]
pub struct MultimediaUseCases {
    /// The [`BulkMediaUseCase`] implementation.
    pub bulk: Arc<dyn BulkMediaUseCase + Send + Sync>,
    /// The [`CreateUploadMediaUrlUseCase`] implementation.
    pub create_signed_post_url: Arc<dyn CreateUploadMediaUrlUseCase + Send + Sync>,
    /// The [`GetVariantReadUrlUseCase`] implementation.
    pub create_signed_get_url: Arc<dyn GetVariantReadUrlUseCase + Send + Sync>,
    /// The [`GetPublicVariantUrlUseCase`] implementation.
    pub get_public_variant_url: Arc<dyn GetPublicVariantUrlUseCase + Send + Sync>,
    /// The [`GetMediaStatusesUseCase`] implementation.
    pub get_media_statuses: Arc<dyn GetMediaStatusesUseCase + Send + Sync>,
    /// The [`PatchMediaUseCase`] implementation.
    pub patch_media: Arc<dyn PatchMediaUseCase + Send + Sync>,
    /// The [`RestoreMediaUseCase`] implementation.
    pub restore_media: Arc<dyn RestoreMediaUseCase + Send + Sync>,
    /// The [`HardDeleteMediaUseCase`] implementation.
    pub hard_delete_media: Arc<dyn HardDeleteMediaUseCase + Send + Sync>,
    /// The [`GetMediaUsageUseCase`] implementation.
    pub get_media_usage: Arc<dyn GetMediaUsageUseCase + Send + Sync>,
    /// The [`ListMediaUseCase`] implementation.
    pub list_media: Arc<dyn ListMediaUseCase + Send + Sync>,
    /// The [`DeleteMediaUseCase`] implementation.
    pub delete_media: Arc<dyn DeleteMediaUseCase + Send + Sync>,
    /// The [`GetMediaUseCase`] implementation.
    pub get_media: Arc<dyn GetMediaUseCase + Send + Sync>,
}
