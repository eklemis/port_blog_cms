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

/// Helper for service: safe object key generation.
/// Strategy: `<media_id>.<ext>` (no user-controlled path segments)
pub fn make_object_key(
    media_id: Uuid,
    original_name: &str,
) -> Result<String, UploadUrlCommandError> {
    // let ext = ext_lower(original_name)?;
    // validate_ext(&ext)?;
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
