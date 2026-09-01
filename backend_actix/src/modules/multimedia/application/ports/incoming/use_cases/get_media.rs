//! Fetching one media item's details.
use async_trait::async_trait;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::multimedia::application::domain::entities::{
    AttachmentTarget, MediaRole, MediaSize, MediaState,
};
use crate::multimedia::application::ports::outgoing::db::{MediaAttachment, MediaQueryError};

/// A single media item with the variant sizes that are ready to read.
///
/// Carries the same fields as a listing row, plus `available_sizes`. Bucket
/// names and object keys are deliberately not exposed: callers reach the bytes
/// through `GET /api/media/{media_id}/{media_size}`, which issues a signed URL,
/// so storage layout stays an internal detail.
/// One media item in full, including which sizes exist.
///
/// `available_sizes` is derived from the variant rows, so an item still being
/// processed comes back with an empty list rather than an error — check
/// `status` to tell "none yet" from "none ever".
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MediaDetail {
    pub media_id: Uuid,
    pub original_filename: String,
    pub status: MediaState,
    pub attachment_target: AttachmentTarget,
    pub attachment_target_id: Uuid,
    pub role: MediaRole,
    pub position: u8,
    pub alt_text: String,
    pub caption: String,

    /// Sizes that can currently be fetched. Empty until processing completes.
    pub available_sizes: Vec<MediaSize>,
}

impl From<MediaAttachment> for MediaDetail {
    fn from(m: MediaAttachment) -> Self {
        Self {
            media_id: m.media_id,
            original_filename: m.original_filename,
            status: m.status,
            attachment_target: m.attachment_target,
            attachment_target_id: m.attachment_target_id,
            role: m.role,
            position: m.position,
            alt_text: m.alt_text,
            caption: m.caption,
            available_sizes: m.variants.into_iter().map(|v| v.size).collect(),
        }
    }
}

/// Why fetching a media item failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum GetMediaError {
    /// Media does not exist, is soft-deleted, or belongs to another user. All
    /// three are reported identically so the endpoint cannot be used to probe
    /// for media ids.
    #[error("Media not found")]
    MediaNotFound,

    #[error("Query error: {0}")]
    QueryError(String),
}

impl From<MediaQueryError> for GetMediaError {
    fn from(e: MediaQueryError) -> Self {
        match e {
            MediaQueryError::MediaNotFound => GetMediaError::MediaNotFound,
            MediaQueryError::DatabaseError(msg) => GetMediaError::QueryError(msg),
        }
    }
}

/// Fetches one media item's details.
#[async_trait]
pub trait GetMediaUseCase: Send + Sync {
    /// Returns the item, scoped to `owner`.
    async fn execute(&self, owner: UserId, media_id: Uuid) -> Result<MediaDetail, GetMediaError>;
}
