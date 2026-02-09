#[derive(Debug, Clone)]
pub struct UploadPolicy {
    pub max_file_size_bytes: u64,
    pub max_width_height_px: u32,
    pub max_file_name_len: usize,
    pub allowed_mime_types: &'static [&'static str],
    pub bucket_name: String,
}

impl UploadPolicy {
    pub const DEFAULT_BUCKET_NAME: &'static str = "blogport-cms-upload";
    pub const DEFAULT_ALLOWED_MIME_TYPES: &'static [&'static str] =
        &["image/jpeg", "image/png", "image/webp"];

    /// Load policy with `bucket_name` from env var, fallback to "blogport-cms-upload".
    ///
    /// Env var name suggestion: `MULTIMEDIA_UPLOAD_BUCKET`
    pub fn from_env() -> Self {
        let bucket_name = std::env::var("MULTIMEDIA_UPLOAD_BUCKET")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| Self::DEFAULT_BUCKET_NAME.to_string());

        Self {
            max_file_size_bytes: 5 * 1024 * 1024, // 5MB
            max_width_height_px: 6000,
            max_file_name_len: 255,
            allowed_mime_types: Self::DEFAULT_ALLOWED_MIME_TYPES,
            bucket_name,
        }
    }

    /// Handy for unit tests or custom wiring (no env reads).
    pub fn new(bucket_name: String) -> Self {
        Self {
            bucket_name,
            ..Self::from_env_with_bucket_fallback(Self::DEFAULT_BUCKET_NAME)
        }
    }

    /// Internal helper to keep construction consistent without repeating values.
    fn from_env_with_bucket_fallback(fallback: &str) -> Self {
        Self {
            max_file_size_bytes: 5 * 1024 * 1024,
            max_width_height_px: 6000,
            max_file_name_len: 255,
            allowed_mime_types: Self::DEFAULT_ALLOWED_MIME_TYPES,
            bucket_name: fallback.to_string(),
        }
    }
}
