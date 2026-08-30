use std::sync::Arc;

use crate::multimedia::application::ports::incoming::use_cases::{
    CreateUploadMediaUrlUseCase, DeleteMediaUseCase, GetMediaUseCase, GetVariantReadUrlUseCase,
    ListMediaUseCase,
};

#[derive(Clone)]
pub struct MultimediaUseCases {
    pub create_signed_post_url: Arc<dyn CreateUploadMediaUrlUseCase + Send + Sync>,
    pub create_signed_get_url: Arc<dyn GetVariantReadUrlUseCase + Send + Sync>,
    pub list_media: Arc<dyn ListMediaUseCase + Send + Sync>,
    pub delete_media: Arc<dyn DeleteMediaUseCase + Send + Sync>,
    pub get_media: Arc<dyn GetMediaUseCase + Send + Sync>,
}
