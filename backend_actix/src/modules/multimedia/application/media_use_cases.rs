use std::sync::Arc;

use crate::multimedia::application::ports::incoming::use_cases::{
    CreateUploadMediaUrlUseCase, GetVariantReadUrlUseCase,
};

#[derive(Clone)]
pub struct MultimediaUseCases {
    pub create_signed_post_url: Arc<dyn CreateUploadMediaUrlUseCase + Send + Sync>,
    pub create_signed_get_url: Arc<dyn GetVariantReadUrlUseCase + Send + Sync>,
}
