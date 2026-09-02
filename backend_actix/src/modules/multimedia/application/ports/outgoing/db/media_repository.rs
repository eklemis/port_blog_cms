//! Write-side port for media rows and their attachments.
//!
//! An upload is recorded before the bytes arrive: the row is created in a
//! pending state, the client is handed a signed URL, and an out-of-band
//! function flips the state once variants exist. So a media row existing does
//! not mean the file does — see `MediaState`.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::application::domain::entities::UserId,
    multimedia::application::domain::entities::{
        AttachmentTarget, MediaRole, MediaSize, MediaState, MediaStateInfo, MediaVariant,
    },
};

/// A media row to be inserted.
///
/// Dimensions and durations are optional because they are only known for the
/// media types that have them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewMedia {
    /// The uploading user.
    pub owner: UserId,
    /// Initial processing state. A row is written before the bytes arrive, so
    /// this starts pending.
    pub state: MediaState,
    /// Bucket the upload is destined for.
    pub bucket_name: String,
    /// The name the client uploaded under, kept for display.
    pub original_name: String,
    /// MIME type as declared by the client. Never verified against the bytes —
    /// they do not pass through this service.
    pub mime_type: String,
    /// Size as declared by the client.
    pub file_size_bytes: u64,
    /// Declared width. `None` for media with no pixel dimensions.
    pub width_px: Option<u32>,
    /// Declared height. `None` for media with no pixel dimensions.
    pub height_px: Option<u32>,
    /// Declared duration. `None` for still media.
    pub duration_seconds: Option<u64>,
}

/// The attachment row recorded alongside a media row.
///
/// Carries no `media_id`: both rows are written in one transaction, so the id
/// is only known inside it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewMediaAttachment {
    /// The uploading user.
    pub owner: UserId,
    /// What kind of thing this attaches to.
    pub attachment_target: AttachmentTarget,
    /// The id of that thing.
    pub attachment_target_id: Uuid,
    /// What the media is for on its target.
    pub role: MediaRole,
    /// start from 0
    pub position: u8,
    /// Alternative text, if the client supplied one.
    pub alt_text: Option<String>,
    /// Caption, if the client supplied one.
    pub caption: Option<String>,
}

/// Transaction payload: record media + its attachment atomically.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordMediaTx {
    /// The media row.
    pub media: NewMedia,
    /// The attachment row, written in the same transaction.
    pub attachment: NewMediaAttachment,
}

/// Minimal info the use case needs after recording.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedMedia {
    /// The uploading user.
    pub owner: UserId,
    /// Primary key assigned to the new media row.
    pub media_id: Uuid,
    /// Bucket the upload is destined for.
    pub bucket_name: String,
    /// The name the client uploaded under, kept for display.
    pub original_name: String,
    /// What kind of thing this attaches to.
    pub attachment_target: AttachmentTarget,
    /// Initial processing state. A row is written before the bytes arrive, so
    /// this starts pending.
    pub state: MediaState,
}

/// Why a media write failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum MediaRepositoryError {
    /// Media doesn't exist OR doesn't belong to owner.
    #[error("Media not found")]
    NotFound,

    /// The store could not be reached, or the transaction failed.
    #[error("Database error: {0}")]
    DatabaseError(String),
}

/// Why recording an upload failed.
///
/// Separate from [`MediaRepositoryError`] because recording writes two rows in
/// one transaction and can fail in ways a plain read or delete cannot.
#[derive(Debug, Clone, thiserror::Error)]
pub enum RecordMediaError {
    /// The store could not be reached, or the transaction failed.
    #[error("Database error: {0}")]
    DatabaseError(String),
}

/// Make fields public so other modules can construct this cleanly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMediaStateData {
    /// The uploading user.
    pub owner: UserId,
    /// Primary key assigned to the new media row.
    pub media_id: Uuid,
    /// The state the row now holds.
    pub status: MediaState,
}

/// One generated size of a media item, written once processing finishes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaVariantRecord {
    /// The uploading user.
    pub owner: UserId,

    /// The media this variant belongs to. `media_variants.media_id` is NOT
    /// NULL, so without it a variant cannot be attached to anything.
    pub media_id: Uuid,

    /// Which generated size this variant is.
    pub size: MediaSize,
    /// Bucket the upload is destined for.
    pub bucket_name: String,
    /// Object key the variant was written to.
    pub object_key: String,
    /// MIME type as declared by the client. Never verified against the bytes —
    /// they do not pass through this service.
    pub mime_type: String,
    /// Size as declared by the client.
    pub file_size_bytes: u64,
    /// Declared width. `None` for media with no pixel dimensions.
    pub width_px: Option<u32>,
    /// Declared height. `None` for media with no pixel dimensions.
    pub height_px: Option<u32>,
}

/// A partial update to an attachment's metadata.
///
/// Only the attachment row is mutable. The file, its MIME type and its
/// dimensions describe bytes that already exist in the bucket and cannot be
/// edited by changing a database row.
///
/// `alt_text` and `caption` are tri-state, matching `PatchField` in `project`
/// and `BlogPatchField` in `blog`: omitted leaves the value alone, `null`
/// clears it, a value replaces it. `position` has no null — an attachment
/// always has an order — so it is a plain `Option`.
#[derive(Debug, Clone, Default)]
pub struct PatchAttachmentData {
    /// New alternative text. `None` leaves it; `Some(None)` clears it.
    pub alt_text: Option<Option<String>>,
    /// New caption. `None` leaves it; `Some(None)` clears it.
    pub caption: Option<Option<String>>,
    /// New display position. `None` leaves it.
    pub position: Option<i32>,
}

impl PatchAttachmentData {
    /// True when the caller asked for no change at all.
    ///
    /// Worth checking before issuing an UPDATE: an empty patch would otherwise
    /// bump `updated_at` and report success for having done nothing.
    pub fn is_empty(&self) -> bool {
        self.alt_text.is_none() && self.caption.is_none() && self.position.is_none()
    }
}

/// Writes media rows, their attachments and their variants.
///
/// Reads belong to [`MediaQuery`](super::media_query::MediaQuery).
#[async_trait]
pub trait MediaRepository: Send + Sync {
    /// Store a row into media and media attachment with transaction
    async fn record_media_tx(&self, tx: RecordMediaTx) -> Result<RecordedMedia, RecordMediaError>;

    /// Moves an item to a new processing state.
    ///
    /// Called by the out-of-band processor once variants exist, or when
    /// generation fails.
    async fn set_media_state(
        &self,
        data: UpdateMediaStateData,
    ) -> Result<MediaStateInfo, MediaRepositoryError>;

    /// Records one generated size.
    async fn record_single_variant(
        &self,
        data: MediaVariantRecord,
    ) -> Result<MediaVariant, MediaRepositoryError>;

    /// Records several generated sizes in one transaction.
    async fn record_variants(
        &self,
        data: Vec<MediaVariantRecord>,
    ) -> Result<Vec<MediaVariant>, MediaRepositoryError>;

    /// Marks a media row deleted by stamping `deleted_at`.
    ///
    /// Storage objects are left in place: the upload bucket is reaped by a GCS
    /// lifecycle rule, and every read path already filters `deleted_at IS NULL`,
    /// so the media disappears from listings and signed-URL requests at once.
    ///
    /// Scoped by owner, so another user's media reports `NotFound` rather than
    /// revealing that it exists.
    async fn soft_delete(&self, owner: UserId, media_id: Uuid) -> Result<(), MediaRepositoryError>;

    /// Applies a partial update to a media item's attachment metadata.
    ///
    /// Scoped by owner and skips soft-deleted media, so another user's item and
    /// a deleted one both report
    /// [`NotFound`](MediaRepositoryError::NotFound).
    ///
    /// # Errors
    /// [`NotFound`](MediaRepositoryError::NotFound) if no live attachment for
    /// that media belongs to `owner`.
    async fn patch_attachment(
        &self,
        owner: UserId,
        media_id: Uuid,
        data: PatchAttachmentData,
    ) -> Result<(), MediaRepositoryError>;

    /// Clears the soft-delete flag, returning the item to listings and to the
    /// public read path.
    ///
    /// Idempotent: restoring an item that was never deleted succeeds. Only a
    /// missing item, or one owned by someone else, is an error.
    async fn restore(&self, owner: UserId, media_id: Uuid) -> Result<(), MediaRepositoryError>;

    /// Removes the media row, its attachments and its variant rows
    /// permanently.
    ///
    /// **Does not delete the stored objects.** Reclaiming those is the bucket's
    /// lifecycle policy, not this call's job — which also means a hard delete
    /// is not a way to make bytes unreachable in a hurry.
    ///
    /// Irreversible, unlike [`soft_delete`](Self::soft_delete).
    async fn hard_delete(&self, owner: UserId, media_id: Uuid) -> Result<(), MediaRepositoryError>;
}
