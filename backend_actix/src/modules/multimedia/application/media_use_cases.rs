use std::sync::Arc;

use crate::multimedia::application::ports::incoming::use_cases::{
    CreateUploadMediaUrlUseCase, DeleteMediaUseCase, GetMediaUseCase, GetPublicVariantUrlUseCase,
    GetVariantReadUrlUseCase, ListMediaUseCase,
};

/// This module's use cases, grouped for `AppState`.
#[derive(Clone)]
pub struct MultimediaUseCases {
    /// The [`CreateUploadMediaUrlUseCase`] implementation.
    pub create_signed_post_url: Arc<dyn CreateUploadMediaUrlUseCase + Send + Sync>,
    /// The [`GetVariantReadUrlUseCase`] implementation.
    pub create_signed_get_url: Arc<dyn GetVariantReadUrlUseCase + Send + Sync>,
    /// The [`GetPublicVariantUrlUseCase`] implementation.
    pub get_public_variant_url: Arc<dyn GetPublicVariantUrlUseCase + Send + Sync>,
    /// The [`ListMediaUseCase`] implementation.
    pub list_media: Arc<dyn ListMediaUseCase + Send + Sync>,
    /// The [`DeleteMediaUseCase`] implementation.
    pub delete_media: Arc<dyn DeleteMediaUseCase + Send + Sync>,
    /// The [`GetMediaUseCase`] implementation.
    pub get_media: Arc<dyn GetMediaUseCase + Send + Sync>,
}
