//! Re-exports the module's use-case contracts.

mod create_get_variant_url;
mod create_upload_url;
mod delete_media;
mod get_media;
mod list_media;
// The two `*Builder` types are re-exported because `CreateMediaCommand::builder`
// and `CreateAttachmentCommand::builder` are public and return them. Without
// this, a caller outside the module can call `builder()` but cannot name what
// it hands back, so the value can only be used in a single chained expression.
// Surfaced by the `-D warnings` rustdoc gate as a link to a private item.
pub use create_upload_url::{
    make_object_key, CreateAttachmentCommand, CreateAttachmentCommandBuilder, CreateMediaCommand,
    CreateMediaCommandBuilder, CreateMediaResult, CreateUploadMediaUrlUseCase, CreateUrlError,
    UploadUrlCommandError,
};

pub use create_get_variant_url::{
    GetReadUrlError, GetUrlCommand, GetUrlResult, GetVariantReadUrlUseCase,
};

pub use delete_media::{DeleteMediaError, DeleteMediaUseCase};
pub use get_media::{GetMediaError, GetMediaUseCase, MediaDetail};

pub use list_media::{ListMediaCommand, ListMediaError, ListMediaUseCase, MediaItem};
mod get_public_variant_url;
pub use get_public_variant_url::{GetPublicVariantUrlError, GetPublicVariantUrlUseCase};
mod media_lifecycle;
pub use media_lifecycle::{
    GetMediaStatusesUseCase, GetMediaUsageUseCase, HardDeleteMediaUseCase, MediaLifecycleError,
    MediaStatus, MediaUsage, PatchMediaUseCase, RestoreMediaUseCase,
};
