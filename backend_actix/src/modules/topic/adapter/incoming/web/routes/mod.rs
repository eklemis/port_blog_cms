mod create_topic;
mod get_topic_usage;
mod get_topics;
mod patch_topic;
mod soft_delete_topic;

// Glob re-exports carry the hidden `__path_<handler>` structs that
// `#[utoipa::path]` generates alongside each handler; `ApiDoc` needs them.
pub use create_topic::*;
pub use get_topic_usage::*;
pub use get_topics::*;
pub use patch_topic::*;
pub use soft_delete_topic::*;
