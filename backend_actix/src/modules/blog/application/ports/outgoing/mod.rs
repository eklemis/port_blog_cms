//! What blog needs from the outside: a post store split into write, read, lifecycle and topic-link ports.

mod blog_post_archiver;
mod blog_post_query;
mod blog_post_repository;
mod blog_post_topic_repository;
mod draft_preview_store;

pub use blog_post_archiver::{BlogPostArchiver, BlogPostArchiverError};
pub use blog_post_query::{
    BlogPageRequest, BlogPageResult, BlogPostCard, BlogPostListFilter, BlogPostQuery,
    BlogPostQueryError, BlogPostSort, BlogPostView,
};
pub use blog_post_repository::{
    BlogPatchField, BlogPostRepository, BlogPostRepositoryError, CreateBlogPostData,
    PatchBlogPostData,
};
pub use blog_post_topic_repository::{BlogPostTopicRepository, BlogPostTopicRepositoryError};
pub use draft_preview_store::{
    DraftPreview, DraftPreviewStore, DraftPreviewStoreError, LivePreview, PreviewMediaResolver,
};
