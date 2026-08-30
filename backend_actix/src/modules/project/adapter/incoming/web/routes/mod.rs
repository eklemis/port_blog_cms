mod add_project_topic;
mod clear_project_topics;
mod create_project;
mod get_project_topics;
mod get_projects;
mod get_public_projects;
mod get_public_single_project;
mod get_single_project;
mod hard_delete_project;
mod patch_project;
mod remove_project_topic;
mod soft_delete_project;

// Glob re-exports carry the hidden `__path_<handler>` structs that
// `#[utoipa::path]` generates alongside each handler; `ApiDoc` needs them.
pub use add_project_topic::*;
pub use clear_project_topics::*;
pub use create_project::*;
pub use get_project_topics::*;
pub use get_projects::*;
pub use get_public_projects::*;
pub use get_public_single_project::*;
pub use get_single_project::*;
pub use hard_delete_project::*;
pub use patch_project::*;
pub use remove_project_topic::*;
pub use soft_delete_project::*;
