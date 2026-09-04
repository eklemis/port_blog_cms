//! One file per surface.

mod quota;

// Glob re-exports carry the hidden `__path_<handler>` structs that
// `#[utoipa::path]` generates alongside each handler; `ApiDoc` needs them.
pub use quota::*;
