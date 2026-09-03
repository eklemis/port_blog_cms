mod bulk_media;
mod delete_media;
mod get_media;
mod get_public_variant;
mod get_variant_url;
mod init_upload;
mod list_media;
mod media_lifecycle;

// Glob re-exports carry the hidden `__path_<handler>` structs that
// `#[utoipa::path]` generates alongside each handler; `ApiDoc` needs them.
pub use bulk_media::*;
pub use delete_media::*;
pub use get_media::*;
pub use get_public_variant::*;
pub use get_variant_url::*;
pub use init_upload::*;
pub use list_media::*;
pub use media_lifecycle::*;
