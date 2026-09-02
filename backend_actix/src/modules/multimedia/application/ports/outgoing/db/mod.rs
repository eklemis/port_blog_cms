//! Database-backed outgoing ports.

mod media_query;
mod media_repository;

pub use media_repository::{
    MediaRepository, MediaRepositoryError, MediaVariantRecord, NewMedia, NewMediaAttachment,
    PatchAttachmentData, RecordMediaError, RecordMediaTx, RecordedMedia, UpdateMediaStateData,
};

pub use media_query::{MediaAttachment, MediaQuery, MediaQueryError, MediaUsageRow, StoredVariant};
