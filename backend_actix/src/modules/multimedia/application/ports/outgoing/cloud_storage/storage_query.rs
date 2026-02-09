use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::multimedia::application::domain::entities::{AttachmentTarget, MediaState};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct ManifestInfo {
    pub media_id: String,
    pub updated_at: String,
    pub status: MediaState,
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum StorageQueryError {
    #[error("Media id not found")]
    MediaIdNotFound,

    #[error("Network problem occurred")]
    NetworkInterrupted,

    #[error("Manifest file not found")]
    ManifestNotFound,
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum MediaInfoError {
    #[error("Field '{0}' cannot be empty")]
    EmptyField(&'static str),
}

/// Signing errors coming from the storage adapter
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum SignUrlError {
    #[error("There is an infrastructure issue")]
    Infrastructure,

    #[error("Access not permitted")]
    AccessDenied,

    #[error("Provided bucket does not exist")]
    BucketNotFound,

    #[error("Request configuration is invalid")]
    Configuration,
}

/// Data needed to sign an upload URL.
///
/// - Construction is validated via `try_new`, so callers never need a builder+unwrap.
/// - Fields are private with getters, keeping the port stable.
#[derive(Debug, Clone, PartialEq)]
pub struct MediaInfo {
    bucket_name: String,
    object_name: String,
    attachment_target: AttachmentTarget,
}

impl MediaInfo {
    pub fn try_new(
        bucket_name: String,
        object_name: String,
        attachment_target: AttachmentTarget,
    ) -> Result<Self, MediaInfoError> {
        if bucket_name.trim().is_empty() {
            return Err(MediaInfoError::EmptyField("bucket_name"));
        }
        if object_name.trim().is_empty() {
            return Err(MediaInfoError::EmptyField("object_name"));
        }

        Ok(Self {
            bucket_name,
            object_name,
            attachment_target,
        })
    }

    pub fn bucket_name(&self) -> &str {
        &self.bucket_name
    }

    pub fn object_name(&self) -> &str {
        &self.object_name
    }

    pub fn attachment_target(&self) -> &AttachmentTarget {
        &self.attachment_target
    }
}

#[async_trait]
pub trait StorageQuery: Send + Sync {
    /// Returns a signed URL for client-side direct uploads.
    async fn get_signed_upload_url(&self, media_info: MediaInfo) -> Result<String, SignUrlError>;

    /// Returns the latest manifest info for `media_id`.
    async fn get_latest_manifest(&self, media_id: &str) -> Result<ManifestInfo, StorageQueryError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_info_try_new_ok() {
        let info = MediaInfo::try_new(
            "bucket".to_string(),
            "obj.png".to_string(),
            AttachmentTarget::Resume,
        )
        .unwrap();

        assert_eq!(info.bucket_name(), "bucket");
        assert_eq!(info.object_name(), "obj.png");
        assert_eq!(info.attachment_target(), &AttachmentTarget::Resume);
    }

    #[test]
    fn test_media_info_try_new_rejects_empty_bucket() {
        let err = MediaInfo::try_new(
            "   ".to_string(),
            "x.png".to_string(),
            AttachmentTarget::User,
        )
        .unwrap_err();
        assert_eq!(err, MediaInfoError::EmptyField("bucket_name"));
    }

    #[test]
    fn test_media_info_try_new_rejects_empty_object_name() {
        let err = MediaInfo::try_new(
            "bucket".to_string(),
            "\t".to_string(),
            AttachmentTarget::Project,
        )
        .unwrap_err();
        assert_eq!(err, MediaInfoError::EmptyField("object_name"));
    }

    #[test]
    fn test_manifest_info_is_serializable_with_media_state() {
        let m = ManifestInfo {
            media_id: "abc".to_string(),
            updated_at: "2026-02-08T00:00:00Z".to_string(),
            status: MediaState::Ready,
        };

        let json = serde_json::to_string(&m).unwrap();
        let back: ManifestInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(m, back);
    }
}
