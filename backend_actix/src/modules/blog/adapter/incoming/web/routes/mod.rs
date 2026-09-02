mod archive_blog_post;
mod blog_post_topics;
mod create_blog_post;
/// Listing blog posts, owner-facing and public.
pub mod get_blog_posts;
mod get_public_blog_post;
mod get_public_blog_posts;
mod get_single_blog_post;
mod hard_delete_blog_post;
mod patch_blog_post;
mod restore_blog_post;
mod slug_available;

// Glob re-exports carry the hidden `__path_<handler>` structs that
// `#[utoipa::path]` generates alongside each handler; `ApiDoc` needs them.
pub use archive_blog_post::*;
pub use blog_post_topics::*;
pub use create_blog_post::*;
pub use get_blog_posts::*;
pub use get_public_blog_post::*;
pub use get_public_blog_posts::*;
pub use get_single_blog_post::*;
pub use hard_delete_blog_post::*;
pub use patch_blog_post::*;
pub use restore_blog_post::*;
pub use slug_available::*;
