mod create_topic;
mod get_topics;
mod soft_delete_topic;

// Glob re-exports carry the hidden `__path_<handler>` structs that
// `#[utoipa::path]` generates alongside each handler; `ApiDoc` needs them.
pub use create_topic::*;
pub use get_topics::*;
pub use soft_delete_topic::*;
