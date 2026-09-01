//! Object-storage outgoing ports.

mod storage_query;
pub use storage_query::{
    ManifestInfo, MediaInfo, MediaInfoError, SignUrlError, StorageQuery, StorageQueryError,
};
