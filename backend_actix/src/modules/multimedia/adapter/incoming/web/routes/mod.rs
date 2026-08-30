mod delete_media;
mod get_variant_url;
mod init_upload;
mod list_media;

// Glob re-exports carry the hidden `__path_<handler>` structs that
// `#[utoipa::path]` generates alongside each handler; `ApiDoc` needs them.
pub use delete_media::*;
pub use get_variant_url::*;
pub use init_upload::*;
pub use list_media::*;
