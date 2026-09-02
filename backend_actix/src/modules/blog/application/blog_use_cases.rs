use std::sync::Arc;

use crate::blog::application::ports::incoming::use_cases::{
    ArchiveBlogPostUseCase, AttachBlogPostTopicUseCase, ClearBlogPostTopicsUseCase,
    CreateBlogPostUseCase, DetachBlogPostTopicUseCase, GetBlogPostTopicsUseCase,
    GetBlogPostsUseCase, GetPublicBlogPostUseCase, GetPublicBlogPostsUseCase,
    GetSingleBlogPostUseCase, HardDeleteBlogPostUseCase, PatchBlogPostUseCase,
    RestoreBlogPostUseCase, SlugAvailableUseCase,
};

/// Bundles the blog use cases so `AppState` gains one field rather than
/// thirteen, mirroring `ProjectUseCases` and `MultimediaUseCases`.
#[derive(Clone)]
pub struct BlogUseCases {
    /// The [`CreateBlogPostUseCase`] implementation.
    pub create: Arc<dyn CreateBlogPostUseCase + Send + Sync>,
    /// The [`GetBlogPostsUseCase`] implementation.
    pub list: Arc<dyn GetBlogPostsUseCase + Send + Sync>,
    /// The [`GetPublicBlogPostsUseCase`] implementation.
    pub list_public: Arc<dyn GetPublicBlogPostsUseCase + Send + Sync>,
    /// The [`GetSingleBlogPostUseCase`] implementation.
    pub get_single: Arc<dyn GetSingleBlogPostUseCase + Send + Sync>,
    /// The [`GetPublicBlogPostUseCase`] implementation.
    pub get_public: Arc<dyn GetPublicBlogPostUseCase + Send + Sync>,
    /// The [`PatchBlogPostUseCase`] implementation.
    pub patch: Arc<dyn PatchBlogPostUseCase + Send + Sync>,
    /// The [`ArchiveBlogPostUseCase`] implementation.
    pub archive: Arc<dyn ArchiveBlogPostUseCase + Send + Sync>,
    /// The [`RestoreBlogPostUseCase`] implementation.
    pub restore: Arc<dyn RestoreBlogPostUseCase + Send + Sync>,
    /// The [`HardDeleteBlogPostUseCase`] implementation.
    pub hard_delete: Arc<dyn HardDeleteBlogPostUseCase + Send + Sync>,
    /// The [`AttachBlogPostTopicUseCase`] implementation.
    pub attach_topic: Arc<dyn AttachBlogPostTopicUseCase + Send + Sync>,
    /// The [`DetachBlogPostTopicUseCase`] implementation.
    pub detach_topic: Arc<dyn DetachBlogPostTopicUseCase + Send + Sync>,
    /// The [`ClearBlogPostTopicsUseCase`] implementation.
    pub clear_topics: Arc<dyn ClearBlogPostTopicsUseCase + Send + Sync>,
    /// The [`SlugAvailableUseCase`] implementation.
    pub slug_available: Arc<dyn SlugAvailableUseCase + Send + Sync>,
    /// The [`GetBlogPostTopicsUseCase`] implementation.
    pub get_topics: Arc<dyn GetBlogPostTopicsUseCase + Send + Sync>,
}
