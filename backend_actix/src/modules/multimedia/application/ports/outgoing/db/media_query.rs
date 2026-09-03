//! Read-side port for media rows, their variants and their attachments.

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    auth::application::domain::entities::UserId,
    multimedia::application::domain::entities::{
        AttachmentTarget, MediaRole, MediaSize, MediaState, MediaStateInfo,
    },
};

/// Information about a media variant from storage
#[derive(Debug, Clone)]
pub struct StoredVariant {
    /// Which generated size this row is.
    pub size: MediaSize,
    /// Bucket the variant was written to.
    pub bucket_name: String,
    /// Object key of the variant.
    pub object_name: String,
    /// Width in pixels, as generated.
    pub width: u32,
    /// Height in pixels, as generated.
    pub height: u32,
    /// Size of the generated file.
    pub file_size_bytes: u64,
    /// MIME type of the generated file, which may differ from the original's.
    pub mime_type: String,
}

/// Complete media attachment information from database
#[derive(Debug, Clone)]
pub struct MediaAttachment {
    /// The media item.
    pub media_id: Uuid,
    /// The user who uploaded it.
    pub owner: UserId,
    /// What kind of thing it is attached to — a CV, a project, a post.
    pub attachment_target: AttachmentTarget,
    /// The id of that thing.
    pub attachment_target_id: Uuid,
    /// Where the item is in processing. Rows exist before their bytes do, so
    /// this is what says whether the file is usable.
    pub status: MediaState,
    /// What the media is for on its target — a cover image, a gallery entry.
    pub role: MediaRole,
    /// Display order within its role, starting at 0.
    pub position: u8,
    /// Alternative text. Empty rather than absent when unset.
    pub alt_text: String,
    /// Caption. Empty rather than absent when unset.
    pub caption: String,
    /// The name the file was uploaded under.
    pub original_filename: String,
    /// Generated sizes. Empty while the item is still processing.
    pub variants: Vec<StoredVariant>,
}

/// One attachment of a media item, with the visibility of what it is attached
/// to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaUsageRow {
    /// What kind of thing it is attached to, as stored.
    pub attachable_type: String,
    /// The id of that thing.
    pub attachable_id: Uuid,
    /// What the media is for on that target.
    pub role: String,
    /// Whether readers can see that target right now.
    pub is_published: bool,
}

/// Why a media read failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum MediaQueryError {
    /// No media matched the id, or it belongs to another user.
    #[error("Media not found")]
    MediaNotFound,

    /// The store could not be reached.
    #[error("Database error: {0}")]
    DatabaseError(String),
}

/// Reads media rows.
#[async_trait]
pub trait MediaQuery: Send + Sync {
    /// The processing state of one upload.
    ///
    /// Callers poll this to learn whether variants exist yet: a row is created
    /// before the bytes arrive, so existence does not imply availability.
    async fn get_state(&self, media_id: Uuid) -> Result<MediaStateInfo, MediaQueryError>;

    /// Every media item attached to one target — a CV, a project, a post.
    async fn list_by_target(
        &self,
        owner: UserId,
        target: AttachmentTarget,
    ) -> Result<Vec<MediaAttachment>, MediaQueryError>;

    /// What a media item is attached to, and in what role.
    /// The storage coordinates of one variant, **only if it is publicly
    /// visible**.
    ///
    /// Publicly visible means the media is attached to something a reader can
    /// already see — today, a blog post that is published, not scheduled for
    /// the future, and not deleted.
    ///
    /// This is what makes public media revocable. Unpublishing a post makes
    /// this return `Ok(None)`, and the redirect route 404s from then on. A
    /// world-readable bucket could not do that: once a URL escaped, the object
    /// was reachable forever.
    ///
    /// Returns `Ok(None)` for "no such variant" and for "not publicly
    /// visible", deliberately collapsed: telling them apart would let a caller
    /// discover which drafts exist.
    async fn find_public_variant(
        &self,
        media_id: Uuid,
        size: MediaSize,
    ) -> Result<Option<StoredVariant>, MediaQueryError>;

    /// A variant of media attached to one specific thing, **without** the
    /// publication check [`find_public_variant`](Self::find_public_variant)
    /// applies.
    ///
    /// This exists for the draft preview, where the reader holds a capability
    /// for one unpublished post and the ordinary public rule would — correctly
    /// — refuse every image on it. Authorisation has already happened by the
    /// time this is called; the attachment match is what stops that capability
    /// reaching any media except the post's own.
    ///
    /// Callers must therefore never reach this from a path that has not
    /// already established the caller may read `attachable_id`.
    async fn find_variant_attached_to(
        &self,
        media_id: Uuid,
        size: MediaSize,
        attachable_type: AttachmentTarget,
        attachable_id: Uuid,
    ) -> Result<Option<StoredVariant>, MediaQueryError>;

    /// The processing state of several items at once.
    ///
    /// Exists because a grid with twelve uploads in flight otherwise polls
    /// twelve times every two seconds, per client. One call collapses that.
    ///
    /// Scoped by owner, and **ids that do not resolve are simply absent from
    /// the result** rather than erroring — a caller polling a set does not want
    /// the whole batch to fail because one item was deleted mid-poll.
    async fn get_states(
        &self,
        owner: UserId,
        media_ids: &[Uuid],
    ) -> Result<Vec<MediaStateInfo>, MediaQueryError>;

    /// Every place a media item is attached, with each target's visibility.
    ///
    /// The visibility flag is the point: a delete confirmation that can say
    /// "this is on a post that is live right now" is a warning, where a bare
    /// count is a shrug.
    ///
    /// Scoped by owner. An unused item is an empty vec, not an error.
    async fn find_media_usage(
        &self,
        owner: UserId,
        media_id: Uuid,
    ) -> Result<Vec<MediaUsageRow>, MediaQueryError>;

    /// What a media item is attached to, and in what role.
    async fn get_attachment_info(&self, media_id: Uuid)
        -> Result<MediaAttachment, MediaQueryError>;
}
