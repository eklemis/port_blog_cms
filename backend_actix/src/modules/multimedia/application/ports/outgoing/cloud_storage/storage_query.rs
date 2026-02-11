use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::multimedia::application::domain::entities::{AttachmentTarget, MediaState};

// ============================================================================
// Domain Types
// ============================================================================

/// Manifest information retrieved from cloud storage.
///
/// Contains the processing status and metadata for a media file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ManifestInfo {
    pub media_id: String,
    pub updated_at: String,
    pub status: MediaState,
}

/// Validated data needed to sign URLs for cloud storage operations.
///
/// # Construction
/// - Use `try_new()` which validates that fields are non-empty
/// - Fields are private with getters for encapsulation
///
/// # Example
/// ```ignore
/// let info = MediaInfo::try_new(
///     "blogport-cms-ready".to_string(),
///     "media123/image.webp".to_string(),
///     AttachmentTarget::Resume,
/// )?;
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct MediaInfo {
    bucket_name: String,
    object_name: String,
    attachment_target: AttachmentTarget,
}

impl MediaInfo {
    /// Creates a new MediaInfo, validating that required fields are non-empty.
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

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur when querying cloud storage.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum StorageQueryError {
    #[error("Media ID not found")]
    MediaIdNotFound,

    #[error("Manifest file not found")]
    ManifestNotFound,

    #[error("Network problem occurred")]
    NetworkInterrupted,
}

/// Errors that can occur when constructing MediaInfo.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum MediaInfoError {
    #[error("Field '{0}' cannot be empty")]
    EmptyField(&'static str),
}

/// Errors that can occur when signing URLs.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum SignUrlError {
    #[error("Infrastructure error occurred")]
    Infrastructure,

    #[error("Access denied")]
    AccessDenied,

    #[error("Bucket not found")]
    BucketNotFound,

    #[error("Invalid configuration")]
    Configuration,
}

// ============================================================================
// Port Interface
// ============================================================================

/// Port for querying cloud storage and generating signed URLs.
///
/// Implementations handle:
/// - Generating signed URLs for direct client uploads/downloads
/// - Retrieving manifest files that track processing status
#[async_trait]
pub trait StorageQuery: Send + Sync {
    /// Returns a signed URL for client-side direct uploads.
    ///
    /// The URL allows clients to upload directly to cloud storage
    /// without routing through the backend.
    async fn get_signed_upload_url(&self, media_info: MediaInfo) -> Result<String, SignUrlError>;

    /// Returns a signed URL for client-side direct reads.
    ///
    /// The URL allows clients to download/view media directly from
    /// cloud storage with temporary access credentials.
    async fn get_signed_read_url(&self, media_info: MediaInfo) -> Result<String, SignUrlError>;

    /// Returns the latest manifest info for a media file.
    ///
    /// Manifests are written by the image processing service and contain
    /// the current processing status (pending, processing, ready, failed).
    async fn get_latest_manifest(&self, media_id: &str) -> Result<ManifestInfo, StorageQueryError>;
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------
    // MediaInfo tests
    // -----------------------

    #[test]
    fn test_media_info_try_new_success() {
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
    fn test_media_info_rejects_empty_bucket() {
        let err = MediaInfo::try_new(
            "   ".to_string(),
            "x.png".to_string(),
            AttachmentTarget::User,
        )
        .unwrap_err();

        assert_eq!(err, MediaInfoError::EmptyField("bucket_name"));
    }

    #[test]
    fn test_media_info_rejects_empty_object_name() {
        let err = MediaInfo::try_new(
            "bucket".to_string(),
            "\t".to_string(),
            AttachmentTarget::Project,
        )
        .unwrap_err();

        assert_eq!(err, MediaInfoError::EmptyField("object_name"));
    }

    // -----------------------
    // ManifestInfo tests
    // -----------------------

    #[test]
    fn test_manifest_info_serialization() {
        let manifest = ManifestInfo {
            media_id: "abc123".to_string(),
            updated_at: "2026-02-08T00:00:00Z".to_string(),
            status: MediaState::Ready,
        };

        let json = serde_json::to_string(&manifest).unwrap();
        let deserialized: ManifestInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(manifest, deserialized);
    }

    #[test]
    fn test_manifest_info_all_states() {
        for state in [
            MediaState::Pending,
            MediaState::Processing,
            MediaState::Ready,
            MediaState::Failed,
        ] {
            let manifest = ManifestInfo {
                media_id: "test".to_string(),
                updated_at: "2026-02-08T00:00:00Z".to_string(),
                status: state.clone(),
            };

            let json = serde_json::to_string(&manifest).unwrap();
            let back: ManifestInfo = serde_json::from_str(&json).unwrap();
            assert_eq!(manifest.status, back.status);
        }
    }
}
