use std::sync::Arc;

use crate::blog::application::ports::incoming::use_cases::{
    ArchiveBlogPostUseCase, AttachBlogPostTopicUseCase, ClearBlogPostTopicsUseCase,
    CreateBlogPostUseCase, DetachBlogPostTopicUseCase, GetBlogPostTopicsUseCase,
    GetBlogPostsUseCase, GetPublicBlogPostUseCase, GetPublicBlogPostsUseCase,
    GetSingleBlogPostUseCase, HardDeleteBlogPostUseCase, PatchBlogPostUseCase,
    RestoreBlogPostUseCase,
};

/// Bundles the blog use cases so `AppState` gains one field rather than
/// thirteen, mirroring `ProjectUseCases` and `MultimediaUseCases`.
#[derive(Clone)]
pub struct BlogUseCases {
    pub create: Arc<dyn CreateBlogPostUseCase + Send + Sync>,
    pub list: Arc<dyn GetBlogPostsUseCase + Send + Sync>,
    pub list_public: Arc<dyn GetPublicBlogPostsUseCase + Send + Sync>,
    pub get_single: Arc<dyn GetSingleBlogPostUseCase + Send + Sync>,
    pub get_public: Arc<dyn GetPublicBlogPostUseCase + Send + Sync>,
    pub patch: Arc<dyn PatchBlogPostUseCase + Send + Sync>,
    pub archive: Arc<dyn ArchiveBlogPostUseCase + Send + Sync>,
    pub restore: Arc<dyn RestoreBlogPostUseCase + Send + Sync>,
    pub hard_delete: Arc<dyn HardDeleteBlogPostUseCase + Send + Sync>,
    pub attach_topic: Arc<dyn AttachBlogPostTopicUseCase + Send + Sync>,
    pub detach_topic: Arc<dyn DetachBlogPostTopicUseCase + Send + Sync>,
    pub clear_topics: Arc<dyn ClearBlogPostTopicsUseCase + Send + Sync>,
    pub get_topics: Arc<dyn GetBlogPostTopicsUseCase + Send + Sync>,
}
