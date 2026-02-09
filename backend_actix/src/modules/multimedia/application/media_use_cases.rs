use std::sync::Arc;

use crate::multimedia::application::ports::incoming::use_cases::CreateUploadMediaUrlUseCase;

#[derive(Clone)]
pub struct MultimediaUseCases {
    pub create_signed_post_url: Arc<dyn CreateUploadMediaUrlUseCase + Send + Sync>,
}
