//! One file per resource.

mod applications;
mod jobs;

// Glob re-exports carry the hidden `__path_<handler>` structs that
// `#[utoipa::path]` generates alongside each handler; `ApiDoc` needs them.
pub use applications::*;
pub use jobs::*;
