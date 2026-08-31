use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

use crate::{
    auth::application::domain::entities::UserId,
    multimedia::application::{
        domain::{
            entities::{AttachmentTarget, MediaRole, MediaState},
            policies::upload_policy::UploadPolicy,
        },
        ports::outgoing::{
            cloud_storage::SignUrlError,
            db::{NewMedia, NewMediaAttachment},
        },
    },
};

#[derive(Debug, Clone, thiserror::Error)]
pub enum UploadUrlCommandError {
    #[error("Missing required field: {0}")]
    MissingField(&'static str),

    #[error("Invalid file name")]
    InvalidFileName,

    #[error("File too large (max {max_bytes} bytes, got {actual_bytes} bytes)")]
    FileTooLarge { max_bytes: u64, actual_bytes: u64 },

    #[error("Invalid image dimensions (max {max_px}px, got {width_px}x{height_px})")]
    InvalidDimensions {
        max_px: u32,
        width_px: u32,
        height_px: u32,
    },

    #[error("Invalid mime type: {0}")]
    InvalidMimeType(String),

    #[error("Invalid file extension: {0}")]
    InvalidExtension(String),

    #[error("Mime type does not match file extension (mime={mime_type}, ext={ext})")]
    MimeExtensionMismatch { mime_type: String, ext: String },
}

fn sanitize_basename(file_name: &str, max_len: usize) -> Result<String, UploadUrlCommandError> {
    let p = Path::new(file_name);

    let base = p
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or(UploadUrlCommandError::InvalidFileName)?;

    if base.is_empty() || base.len() > max_len {
        return Err(UploadUrlCommandError::InvalidFileName);
    }

    // Reject path-like input (basename differs)
    if base != file_name {
        return Err(UploadUrlCommandError::InvalidFileName);
    }

    // Basic hardening: reject control characters
    if base.chars().any(|c| c.is_control()) {
        return Err(UploadUrlCommandError::InvalidFileName);
    }

    Ok(base.to_string())
}

fn ext_lower(file_name: &str) -> Result<String, UploadUrlCommandError> {
    let ext = Path::new(file_name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .trim();

    if ext.is_empty() {
        return Err(UploadUrlCommandError::InvalidExtension("".to_string()));
    }

    Ok(ext.to_ascii_lowercase())
}

fn validate_mime(mime_type: &str, allowed: &[&str]) -> Result<(), UploadUrlCommandError> {
    if !allowed.contains(&mime_type) {
        return Err(UploadUrlCommandError::InvalidMimeType(
            mime_type.to_string(),
        ));
    }
    Ok(())
}

fn validate_ext(ext: &str) -> Result<(), UploadUrlCommandError> {
    match ext {
        "jpg" | "jpeg" | "png" | "webp" => Ok(()),
        other => Err(UploadUrlCommandError::InvalidExtension(other.to_string())),
    }
}

fn validate_mime_ext_match(mime: &str, ext: &str) -> Result<(), UploadUrlCommandError> {
    // Cheap defense. Real verification should happen post-upload before leaving Pending.
    let ok = match mime {
        "image/jpeg" => matches!(ext, "jpg" | "jpeg"),
        "image/png" => ext == "png",
        "image/webp" => ext == "webp",
        _ => false,
    };

    if !ok {
        return Err(UploadUrlCommandError::MimeExtensionMismatch {
            mime_type: mime.to_string(),
            ext: ext.to_string(),
        });
    }
    Ok(())
}

/// Builds the storage object key for a media item.
///
/// Produces `<media_id>/<original_name>`.
///
/// Validates the extension only. The name is interpolated as given, so this
/// does NOT by itself prevent a path segment: `"../escape.png"` yields
/// `<media_id>/../escape.png`. It is safe in practice because its only caller
/// passes a name already through `sanitize_basename`, which rejects anything
/// path-like — a second caller would have to do the same.
///
/// The previous comment claimed `<media_id>.<ext>` and "no user-controlled path
/// segments". Neither matched the implementation.
pub fn make_object_key(
    media_id: Uuid,
    original_name: &str,
) -> Result<String, UploadUrlCommandError> {
    let ext = ext_lower(original_name)?;
    validate_ext(&ext)?;
    Ok(format!("{}/{}", media_id, original_name))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMediaCommand {
    owner: UserId,
    state: MediaState,
    bucket_name: String,
    original_name: String,
    mime_type: String,
    file_size_bytes: u64,
    width_px: Option<u32>,
    height_px: Option<u32>,
    duration_seconds: Option<u64>,
}

impl CreateMediaCommand {
    pub fn builder() -> CreateMediaCommandBuilder {
        CreateMediaCommandBuilder::default()
    }

    pub fn owner(&self) -> &UserId {
        &self.owner
    }
    pub fn original_name(&self) -> &str {
        &self.original_name
    }
    pub fn mime_type(&self) -> &str {
        &self.mime_type
    }
    pub fn file_size_bytes(&self) -> u64 {
        self.file_size_bytes
    }
    pub fn width_px(&self) -> Option<u32> {
        self.width_px
    }
    pub fn height_px(&self) -> Option<u32> {
        self.height_px
    }
    pub fn duration_seconds(&self) -> Option<u64> {
        self.duration_seconds
    }

    pub fn to_new_media(&self) -> NewMedia {
        NewMedia {
            owner: self.owner,
            state: self.state.clone(),
            bucket_name: self.bucket_name.clone(),
            original_name: self.original_name.clone(),
            mime_type: self.mime_type.clone(),
            file_size_bytes: self.file_size_bytes,
            width_px: self.width_px,
            height_px: self.height_px,
            duration_seconds: self.duration_seconds,
        }
    }
}

#[derive(Default)]
pub struct CreateMediaCommandBuilder {
    owner: Option<UserId>,
    file_name: Option<String>,
    mime_type: Option<String>,
    file_size_bytes: Option<u64>,
    width_px: Option<u32>,
    height_px: Option<u32>,
    duration_seconds: Option<u64>,
}

impl CreateMediaCommandBuilder {
    pub fn owner(mut self, owner: UserId) -> Self {
        self.owner = Some(owner);
        self
    }

    pub fn file_name(mut self, file_name: String) -> Self {
        self.file_name = Some(file_name);
        self
    }

    pub fn mime_type(mut self, mime_type: String) -> Self {
        self.mime_type = Some(mime_type);
        self
    }

    pub fn file_size_bytes(mut self, size: u64) -> Self {
        self.file_size_bytes = Some(size);
        self
    }

    pub fn width_px(mut self, width_px: Option<u32>) -> Self {
        self.width_px = width_px;
        self
    }

    pub fn height_px(mut self, height_px: Option<u32>) -> Self {
        self.height_px = height_px;
        self
    }

    pub fn duration_seconds(mut self, duration_seconds: Option<u64>) -> Self {
        self.duration_seconds = duration_seconds;
        self
    }

    /// Build a validated command using injected policy (no hardcoded constants).
    pub fn build(self, policy: &UploadPolicy) -> Result<CreateMediaCommand, UploadUrlCommandError> {
        let owner = self
            .owner
            .ok_or(UploadUrlCommandError::MissingField("owner"))?;
        let file_name = self
            .file_name
            .ok_or(UploadUrlCommandError::MissingField("file_name"))?;
        let mime_type = self
            .mime_type
            .ok_or(UploadUrlCommandError::MissingField("mime_type"))?;
        let file_size_bytes = self
            .file_size_bytes
            .ok_or(UploadUrlCommandError::MissingField("file_size_bytes"))?;

        // 1) Filename hardening + extension rules
        let safe_name = sanitize_basename(&file_name, policy.max_file_name_len)?;
        let ext = ext_lower(&safe_name)?;
        validate_ext(&ext)?;

        // 2) Mime allowlist + mime/ext consistency
        validate_mime(&mime_type, policy.allowed_mime_types)?;
        validate_mime_ext_match(&mime_type, &ext)?;

        // 3) File size rule
        if file_size_bytes > policy.max_file_size_bytes {
            return Err(UploadUrlCommandError::FileTooLarge {
                max_bytes: policy.max_file_size_bytes,
                actual_bytes: file_size_bytes,
            });
        }

        // 4) Dimensions rule:
        // - either both present or both absent
        // - if present: non-zero and <= max
        if let (Some(w), Some(h)) = (self.width_px, self.height_px) {
            if w == 0 || h == 0 || w > policy.max_width_height_px || h > policy.max_width_height_px
            {
                return Err(UploadUrlCommandError::InvalidDimensions {
                    max_px: policy.max_width_height_px,
                    width_px: w,
                    height_px: h,
                });
            }
        } else if self.width_px.is_some() ^ self.height_px.is_some() {
            return Err(UploadUrlCommandError::MissingField("width_px/height_px"));
        }

        Ok(CreateMediaCommand {
            owner,
            state: MediaState::Pending,
            bucket_name: policy.bucket_name.clone(),
            original_name: safe_name,
            mime_type,
            file_size_bytes,
            width_px: self.width_px,
            height_px: self.height_px,
            duration_seconds: self.duration_seconds,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAttachmentCommand {
    owner: UserId,
    attachment_target: AttachmentTarget,
    attachment_target_id: Uuid,
    role: MediaRole,
    position: u8,
    alt_text: Option<String>,
    caption: Option<String>,
}

impl CreateAttachmentCommand {
    pub fn builder() -> CreateAttachmentCommandBuilder {
        CreateAttachmentCommandBuilder::default()
    }

    pub fn owner(&self) -> &UserId {
        &self.owner
    }
    pub fn attachment_target(&self) -> &AttachmentTarget {
        &self.attachment_target
    }
    pub fn attachment_target_id(&self) -> Uuid {
        self.attachment_target_id
    }
    pub fn role(&self) -> &MediaRole {
        &self.role
    }
    pub fn position(&self) -> u8 {
        self.position
    }
    pub fn alt_text(&self) -> Option<&str> {
        self.alt_text.as_deref()
    }
    pub fn caption(&self) -> Option<&str> {
        self.caption.as_deref()
    }

    pub fn to_new_attachment(&self) -> NewMediaAttachment {
        NewMediaAttachment {
            owner: self.owner.clone(),
            attachment_target: self.attachment_target.clone(),
            attachment_target_id: self.attachment_target_id,
            role: self.role.clone(),
            position: self.position,
            alt_text: self.alt_text.clone(),
            caption: self.caption.clone(),
        }
    }
}

#[derive(Default)]
pub struct CreateAttachmentCommandBuilder {
    owner: Option<UserId>,
    attachment_target: Option<AttachmentTarget>,
    attachment_target_id: Option<Uuid>,
    role: Option<MediaRole>,
    position: Option<u8>,
    alt_text: Option<String>,
    caption: Option<String>,
}

impl CreateAttachmentCommandBuilder {
    pub fn owner(mut self, owner: UserId) -> Self {
        self.owner = Some(owner);
        self
    }
    pub fn attachment_target(mut self, target: AttachmentTarget) -> Self {
        self.attachment_target = Some(target);
        self
    }
    pub fn attachment_target_id(mut self, target_id: Uuid) -> Self {
        self.attachment_target_id = Some(target_id);
        self
    }
    pub fn role(mut self, role: MediaRole) -> Self {
        self.role = Some(role);
        self
    }
    pub fn position(mut self, position: u8) -> Self {
        self.position = Some(position);
        self
    }
    pub fn alt_text(mut self, alt_text: String) -> Self {
        self.alt_text = Some(alt_text);
        self
    }
    pub fn caption(mut self, caption: String) -> Self {
        self.caption = Some(caption);
        self
    }

    pub fn build(self) -> Result<CreateAttachmentCommand, UploadUrlCommandError> {
        let owner = self
            .owner
            .ok_or(UploadUrlCommandError::MissingField("owner"))?;
        let attachment_target = self
            .attachment_target
            .ok_or(UploadUrlCommandError::MissingField("attachment_target"))?;
        let attachment_target_id = self
            .attachment_target_id
            .ok_or(UploadUrlCommandError::MissingField("attachment_target_id"))?;
        let role = self
            .role
            .ok_or(UploadUrlCommandError::MissingField("role"))?;
        let position = self
            .position
            .ok_or(UploadUrlCommandError::MissingField("position"))?;

        Ok(CreateAttachmentCommand {
            owner,
            attachment_target,
            attachment_target_id,
            role,
            position,
            alt_text: self.alt_text,
            caption: self.caption,
        })
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum CreateUrlError {
    #[error("Repository error: {0}")]
    RepositoryError(String),

    #[error("Storage service error: {0}")]
    StorageError(String),
}

impl From<SignUrlError> for CreateUrlError {
    fn from(error: SignUrlError) -> Self {
        CreateUrlError::StorageError(error.to_string())
    }
}
#[derive(Debug, Clone)]
pub struct CreateMediaResult {
    pub url: String,
    pub media_id: Uuid,
}
#[async_trait]
pub trait CreateUploadMediaUrlUseCase: Send + Sync {
    async fn execute(
        &self,
        media_command: CreateMediaCommand,
        attachment_command: CreateAttachmentCommand,
    ) -> Result<CreateMediaResult, CreateUrlError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> UploadPolicy {
        UploadPolicy::new("test-bucket".to_string())
    }

    fn valid_builder() -> CreateMediaCommandBuilder {
        CreateMediaCommand::builder()
            .owner(UserId::from(Uuid::new_v4()))
            .file_name("photo.png".to_string())
            .mime_type("image/png".to_string())
            .file_size_bytes(1024)
    }

    // ------------------------------------------------------------------
    // Filename hardening
    // ------------------------------------------------------------------

    /// The filename becomes part of the storage object key, so anything
    /// path-like has to be refused here. `sanitize_basename` compares the
    /// basename against the whole input, which is what rejects these.
    #[test]
    fn rejects_path_like_file_names() {
        for bad in [
            "../evil.png",
            "../../etc/passwd.png",
            "/absolute/photo.png",
            "dir/photo.png",
            "./photo.png",
        ] {
            let err = valid_builder()
                .file_name(bad.to_string())
                .build(&policy())
                .unwrap_err();
            assert!(
                matches!(err, UploadUrlCommandError::InvalidFileName),
                "{bad} should be rejected as a path, got {err:?}"
            );
        }
    }

    #[test]
    fn rejects_a_file_name_with_control_characters() {
        let err = valid_builder()
            .file_name("pho\nto.png".to_string())
            .build(&policy())
            .unwrap_err();
        assert!(matches!(err, UploadUrlCommandError::InvalidFileName));
    }

    #[test]
    fn rejects_a_file_name_longer_than_the_policy_allows() {
        let long = format!("{}.png", "a".repeat(policy().max_file_name_len));
        let err = valid_builder()
            .file_name(long)
            .build(&policy())
            .unwrap_err();
        assert!(matches!(err, UploadUrlCommandError::InvalidFileName));
    }

    // ------------------------------------------------------------------
    // Extension and mime rules
    // ------------------------------------------------------------------

    #[test]
    fn rejects_a_file_name_with_no_extension() {
        let err = valid_builder()
            .file_name("photo".to_string())
            .build(&policy())
            .unwrap_err();
        assert!(matches!(err, UploadUrlCommandError::InvalidExtension(e) if e.is_empty()));
    }

    #[test]
    fn rejects_an_extension_outside_the_allowlist() {
        for bad in ["photo.exe", "photo.svg", "photo.gif", "photo.php"] {
            let err = valid_builder()
                .file_name(bad.to_string())
                .build(&policy())
                .unwrap_err();
            assert!(
                matches!(err, UploadUrlCommandError::InvalidExtension(_)),
                "{bad} should be rejected"
            );
        }
    }

    /// Extensions are compared lowercased, so a capitalised one is accepted
    /// rather than being read as an unknown type.
    #[test]
    fn accepts_an_uppercase_extension() {
        let cmd = valid_builder()
            .file_name("PHOTO.PNG".to_string())
            .build(&policy())
            .unwrap();
        assert_eq!(cmd.original_name(), "PHOTO.PNG");
    }

    #[test]
    fn rejects_a_mime_type_outside_the_allowlist() {
        let err = valid_builder()
            .mime_type("application/pdf".to_string())
            .build(&policy())
            .unwrap_err();
        assert!(matches!(err, UploadUrlCommandError::InvalidMimeType(m) if m == "application/pdf"));
    }

    /// A .png named file claiming to be a jpeg is refused. This is the cheap
    /// half of the check; the comment in the source is explicit that real
    /// content verification belongs after upload, before leaving Pending.
    #[test]
    fn rejects_a_mime_type_that_disagrees_with_the_extension() {
        let err = valid_builder()
            .file_name("photo.png".to_string())
            .mime_type("image/jpeg".to_string())
            .build(&policy())
            .unwrap_err();

        assert!(matches!(
            err,
            UploadUrlCommandError::MimeExtensionMismatch { mime_type, ext }
                if mime_type == "image/jpeg" && ext == "png"
        ));
    }

    #[test]
    fn accepts_both_jpeg_spellings() {
        for name in ["photo.jpg", "photo.jpeg"] {
            assert!(valid_builder()
                .file_name(name.to_string())
                .mime_type("image/jpeg".to_string())
                .build(&policy())
                .is_ok());
        }
    }

    // ------------------------------------------------------------------
    // Size and dimensions
    // ------------------------------------------------------------------

    #[test]
    fn rejects_a_file_over_the_size_limit() {
        let p = policy();
        let err = valid_builder()
            .file_size_bytes(p.max_file_size_bytes + 1)
            .build(&p)
            .unwrap_err();

        assert!(matches!(
            err,
            UploadUrlCommandError::FileTooLarge { max_bytes, actual_bytes }
                if max_bytes == p.max_file_size_bytes && actual_bytes == p.max_file_size_bytes + 1
        ));
    }

    #[test]
    fn accepts_a_file_exactly_at_the_size_limit() {
        let p = policy();
        assert!(valid_builder()
            .file_size_bytes(p.max_file_size_bytes)
            .build(&p)
            .is_ok());
    }

    #[test]
    fn rejects_zero_dimensions() {
        let err = valid_builder()
            .width_px(Some(0))
            .height_px(Some(10))
            .build(&policy())
            .unwrap_err();
        assert!(matches!(
            err,
            UploadUrlCommandError::InvalidDimensions { .. }
        ));
    }

    #[test]
    fn rejects_dimensions_over_the_limit() {
        let p = policy();
        let err = valid_builder()
            .width_px(Some(p.max_width_height_px + 1))
            .height_px(Some(10))
            .build(&p)
            .unwrap_err();
        assert!(matches!(
            err,
            UploadUrlCommandError::InvalidDimensions { .. }
        ));
    }

    /// Width and height must arrive together. One without the other would let a
    /// caller sidestep the bounds check on the missing axis.
    #[test]
    fn rejects_one_dimension_without_the_other() {
        for (w, h) in [(Some(100), None), (None, Some(100))] {
            let err = valid_builder()
                .width_px(w)
                .height_px(h)
                .build(&policy())
                .unwrap_err();
            assert!(
                matches!(err, UploadUrlCommandError::MissingField("width_px/height_px")),
                "({w:?}, {h:?}) should be rejected"
            );
        }
    }

    #[test]
    fn accepts_both_dimensions_absent() {
        assert!(valid_builder().build(&policy()).is_ok());
    }

    // ------------------------------------------------------------------
    // Required fields
    // ------------------------------------------------------------------

    #[test]
    fn reports_each_missing_required_field() {
        let cases: Vec<(&str, CreateMediaCommandBuilder)> = vec![
            (
                "owner",
                CreateMediaCommand::builder()
                    .file_name("p.png".into())
                    .mime_type("image/png".into())
                    .file_size_bytes(1),
            ),
            (
                "file_name",
                CreateMediaCommand::builder()
                    .owner(UserId::from(Uuid::new_v4()))
                    .mime_type("image/png".into())
                    .file_size_bytes(1),
            ),
            (
                "mime_type",
                CreateMediaCommand::builder()
                    .owner(UserId::from(Uuid::new_v4()))
                    .file_name("p.png".into())
                    .file_size_bytes(1),
            ),
            (
                "file_size_bytes",
                CreateMediaCommand::builder()
                    .owner(UserId::from(Uuid::new_v4()))
                    .file_name("p.png".into())
                    .mime_type("image/png".into()),
            ),
        ];

        for (field, builder) in cases {
            let err = builder.build(&policy()).unwrap_err();
            assert!(
                matches!(err, UploadUrlCommandError::MissingField(f) if f == field),
                "expected MissingField({field}), got {err:?}"
            );
        }
    }

    // ------------------------------------------------------------------
    // Built command and object key
    // ------------------------------------------------------------------

    #[test]
    fn a_built_command_starts_pending_and_carries_the_policy_bucket() {
        let cmd = valid_builder().duration_seconds(Some(12)).build(&policy()).unwrap();

        assert_eq!(cmd.original_name(), "photo.png");
        assert_eq!(cmd.mime_type(), "image/png");
        assert_eq!(cmd.file_size_bytes(), 1024);
        assert_eq!(cmd.duration_seconds(), Some(12));
        assert_eq!(cmd.width_px(), None);
        assert_eq!(cmd.height_px(), None);

        let new_media = cmd.to_new_media();
        // Pending, not Ready: the processing pipeline owns that transition.
        assert_eq!(new_media.state, MediaState::Pending);
        assert_eq!(new_media.bucket_name, "test-bucket");
    }

    #[test]
    fn make_object_key_namespaces_by_media_id() {
        let id = Uuid::new_v4();
        assert_eq!(
            make_object_key(id, "photo.png").unwrap(),
            format!("{id}/photo.png")
        );
    }

    #[test]
    fn make_object_key_rejects_a_disallowed_extension() {
        assert!(matches!(
            make_object_key(Uuid::new_v4(), "payload.exe"),
            Err(UploadUrlCommandError::InvalidExtension(_))
        ));
    }

    /// `make_object_key` validates only the extension, not the basename, so it
    /// will happily embed a traversal sequence. It is safe in practice because
    /// its only caller passes a name already through `sanitize_basename` — but
    /// the function's own doc claims "no user-controlled path segments", which
    /// overstates what it enforces. Pinned so the dependency is visible if it
    /// ever gains another caller.
    #[test]
    fn make_object_key_does_not_itself_reject_a_traversal_name() {
        let id = Uuid::new_v4();
        let key = make_object_key(id, "../escape.png").unwrap();
        assert_eq!(key, format!("{id}/../escape.png"));
    }

    // ------------------------------------------------------------------
    // Accessors and remaining branches
    // ------------------------------------------------------------------

    /// The command's fields are private, so these accessors are the only way a
    /// service reads it — `to_new_media` and the object-key builder both go
    /// through them.
    #[test]
    fn media_command_accessors_expose_every_field() {
        let owner = UserId::from(Uuid::new_v4());
        let cmd = CreateMediaCommand::builder()
            .owner(owner)
            .file_name("photo.webp".to_string())
            .mime_type("image/webp".to_string())
            .file_size_bytes(2048)
            .width_px(Some(640))
            .height_px(Some(480))
            .duration_seconds(Some(7))
            .build(&policy())
            .unwrap();

        assert_eq!(cmd.owner(), &owner);
        assert_eq!(cmd.original_name(), "photo.webp");
        assert_eq!(cmd.mime_type(), "image/webp");
        assert_eq!(cmd.file_size_bytes(), 2048);
        assert_eq!(cmd.width_px(), Some(640));
        assert_eq!(cmd.height_px(), Some(480));
        assert_eq!(cmd.duration_seconds(), Some(7));
    }

    /// webp is the format the processing pipeline emits, so its mime/extension
    /// pairing is the one most likely to be exercised in production.
    #[test]
    fn webp_is_accepted_and_mismatches_against_it_are_caught() {
        assert!(valid_builder()
            .file_name("photo.webp".to_string())
            .mime_type("image/webp".to_string())
            .build(&policy())
            .is_ok());

        // png bytes claimed as webp
        assert!(matches!(
            valid_builder()
                .file_name("photo.png".to_string())
                .mime_type("image/webp".to_string())
                .build(&policy())
                .unwrap_err(),
            UploadUrlCommandError::MimeExtensionMismatch { .. }
        ));
    }

    /// A mime outside the allowlist is rejected before the pairing check, so
    /// the `_ => false` arm is only reachable if the allowlist and the pairing
    /// table ever disagree. Covering it keeps that arm honest.
    #[test]
    fn an_unpaired_mime_type_is_rejected() {
        let mut policy = policy();
        // Widen the allowlist without teaching the pairing table about it.
        policy.allowed_mime_types = &["image/jpeg", "image/png", "image/webp", "image/gif"];

        let err = valid_builder()
            .file_name("photo.png".to_string())
            .mime_type("image/gif".to_string())
            .build(&policy)
            .unwrap_err();

        assert!(matches!(
            err,
            UploadUrlCommandError::MimeExtensionMismatch { .. }
        ));
    }

    // ------------------------------------------------------------------
    // Attachment command
    // ------------------------------------------------------------------

    fn valid_attachment() -> CreateAttachmentCommand {
        CreateAttachmentCommand::builder()
            .owner(UserId::from(Uuid::new_v4()))
            .attachment_target(AttachmentTarget::BlogPost)
            .attachment_target_id(Uuid::new_v4())
            .role(MediaRole::Cover)
            .position(2)
            .alt_text("alt".to_string())
            .caption("cap".to_string())
            .build()
            .unwrap()
    }

    #[test]
    fn attachment_command_accessors_expose_every_field() {
        let cmd = valid_attachment();
        assert_eq!(cmd.attachment_target(), &AttachmentTarget::BlogPost);
        assert_eq!(cmd.role(), &MediaRole::Cover);
        assert_eq!(cmd.position(), 2);
        assert_eq!(cmd.alt_text(), Some("alt"));
        assert_eq!(cmd.caption(), Some("cap"));
        assert!(!cmd.attachment_target_id().is_nil());
    }

    #[test]
    fn attachment_command_reports_its_missing_fields() {
        assert!(matches!(
            CreateAttachmentCommand::builder().build().unwrap_err(),
            UploadUrlCommandError::MissingField(_)
        ));
    }
}
