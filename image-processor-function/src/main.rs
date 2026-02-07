#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
use chrono::Utc;
use fast_image_resize::{images::Image, FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer};
use futures::future::join_all;
use google_cloud_storage::{
    client::{Client as GcsClient, ClientConfig},
    http::objects::{
        download::Range,
        get::GetObjectRequest,
        upload::{Media, UploadObjectRequest, UploadType},
    },
};
use image::{DynamicImage, GenericImageView, ImageReader};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info, warn};
use webp::Encoder;

// =============================================================================
// Configuration
// =============================================================================

const PIPELINE_VERSION: &str = "v1";

fn output_bucket() -> String {
    std::env::var("OUTPUT_BUCKET").unwrap_or_else(|_| "blogport-cms-ready".to_string())
}

fn manifest_bucket() -> String {
    std::env::var("MANIFEST_BUCKET").unwrap_or_else(|_| "blogport-cms-manifests".to_string())
}

/// Widths to generate (in addition to 150x150 thumbnail)
const RESIZE_WIDTHS: [u32; 3] = [320, 768, 1200];

/// WebP quality (0-100)
const WEBP_QUALITY: f32 = 80.0;

// =============================================================================
// Business rules
// =============================================================================

const MAX_FILE_BYTES: usize = 5 * 1024 * 1024; // 5MB
const MAX_TOTAL_PIXELS: u64 = 20_000_000; // 20MP
const MAX_DIMENSION: u32 = 6000; // max width/height

#[derive(Clone, Copy, Debug)]
enum AllowedFormat {
    Jpeg,
    Png,
    Webp,
}

fn detect_format(bytes: &[u8]) -> Option<AllowedFormat> {
    // JPEG: FF D8 FF
    if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        return Some(AllowedFormat::Jpeg);
    }
    // PNG: 89 50 4E 47 0D 0A 1A 0A
    if bytes.len() >= 8 && &bytes[..8] == b"\x89PNG\r\n\x1a\n" {
        return Some(AllowedFormat::Png);
    }
    // WEBP: RIFF .... WEBP
    if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some(AllowedFormat::Webp);
    }
    None
}

#[derive(Debug)]
enum RuleCode {
    InvalidType,
    TooLargeBytes,
    TooLargePixels,
    TooLargeDimensions,
    DecodeFailed,
}

impl RuleCode {
    fn as_str(&self) -> &'static str {
        match self {
            RuleCode::InvalidType => "INVALID_TYPE",
            RuleCode::TooLargeBytes => "MAX_SIZE_EXCEEDED",
            RuleCode::TooLargePixels => "MAX_PIXELS_EXCEEDED",
            RuleCode::TooLargeDimensions => "MAX_DIM_EXCEEDED",
            RuleCode::DecodeFailed => "DECODE_FAILED",
        }
    }
}

#[derive(Debug)]
struct RuleError {
    code: RuleCode,
    message: String,
    stage: &'static str, // "validation" | "processing"
}

fn validate_and_decode(bytes: &[u8]) -> Result<(DynamicImage, AllowedFormat), RuleError> {
    if bytes.len() > MAX_FILE_BYTES {
        return Err(RuleError {
            code: RuleCode::TooLargeBytes,
            message: format!(
                "File too large: {} bytes (max {} bytes)",
                bytes.len(),
                MAX_FILE_BYTES
            ),
            stage: "validation",
        });
    }

    let fmt = detect_format(bytes).ok_or_else(|| RuleError {
        code: RuleCode::InvalidType,
        message: "Only JPEG, PNG, and WEBP are allowed".to_string(),
        stage: "validation",
    })?;

    // Decode (CPU-bound)
    let img = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| RuleError {
            code: RuleCode::DecodeFailed,
            message: format!("Failed to guess format: {e}"),
            stage: "validation",
        })?
        .decode()
        .map_err(|e| RuleError {
            code: RuleCode::DecodeFailed,
            message: format!("Failed to decode image: {e}"),
            stage: "validation",
        })?;

    let (w, h) = img.dimensions();

    if w > MAX_DIMENSION || h > MAX_DIMENSION {
        return Err(RuleError {
            code: RuleCode::TooLargeDimensions,
            message: format!(
                "Image too large: {}x{} (max {}x{})",
                w, h, MAX_DIMENSION, MAX_DIMENSION
            ),
            stage: "validation",
        });
    }

    let pixels = (w as u64) * (h as u64);
    if pixels > MAX_TOTAL_PIXELS {
        return Err(RuleError {
            code: RuleCode::TooLargePixels,
            message: format!(
                "Image has too many pixels: {} (max {})",
                pixels, MAX_TOTAL_PIXELS
            ),
            stage: "validation",
        });
    }

    Ok((img, fmt))
}

// =============================================================================
// CloudEvent structures
// =============================================================================

#[derive(Debug)]
struct CloudEventHeaders {
    id: String,
    source: String,
    event_type: String,
    subject: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GcsObjectData {
    bucket: String,
    name: String,
    content_type: Option<String>,
    size: Option<String>,
}

#[derive(Serialize)]
struct FunctionResponse {
    status: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    variants_created: Option<Vec<String>>,
}

// =============================================================================
// Manifest structures
// =============================================================================

#[derive(Serialize)]
struct ManifestOriginal {
    bucket: String,
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    height: Option<u32>,
}

#[derive(Serialize)]
struct ManifestVariant {
    size: u32,
    path: String,
    width: u32,
    height: u32,
}

#[derive(Serialize)]
struct ManifestMetrics {
    /// Total wall-clock time (download + processing + uploads + manifest)
    total_ms: u64,
    /// Download time from GCS
    download_ms: u64,
    /// CPU processing time (decode + resize + encode)
    processing_ms: u64,
    /// Upload time for variants + manifest
    upload_ms: u64,
    encoder: String,
    quality: u32,
}

#[derive(Serialize)]
struct ManifestError {
    code: String,
    message: String,
    stage: String,
}

#[derive(Serialize)]
#[serde(tag = "state")]
enum Manifest {
    #[serde(rename = "ready")]
    Ready {
        media_id: String,
        pipeline_version: String,
        updated_at: String,
        original: ManifestOriginal,
        variants: Vec<ManifestVariant>,
        metrics: ManifestMetrics,
    },
    #[serde(rename = "failed")]
    Failed {
        media_id: String,
        pipeline_version: String,
        updated_at: String,
        error: ManifestError,
    },
}

fn now_iso8601() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn failed_manifest(media_id: String, code: &str, message: String, stage: &str) -> Manifest {
    Manifest::Failed {
        media_id,
        pipeline_version: PIPELINE_VERSION.to_string(),
        updated_at: now_iso8601(),
        error: ManifestError {
            code: code.to_string(),
            message,
            stage: stage.to_string(),
        },
    }
}

fn failed_manifest_from_rule(media_id: String, err: RuleError) -> Manifest {
    failed_manifest(
        media_id,
        err.code.as_str(),
        err.message,
        err.stage, // "validation"
    )
}

// =============================================================================
// Image processing (keeps your algorithm, removes extra copies)
// =============================================================================

struct ResizeTarget {
    width: u32,
    height: u32,
    suffix: String,
    crop_square: Option<u32>,
}

struct ImageVariant {
    suffix: String,
    width: u32,
    height: u32,
    data: Vec<u8>,
}

struct ProcessedImage {
    original_width: u32,
    original_height: u32,
    variants: Vec<ImageVariant>,
}

fn resize_to_webp(
    src: &Image,
    target: &ResizeTarget,
    resizer: &mut Resizer,
) -> Result<(Vec<u8>, u32, u32), String> {
    let mut dst_image = Image::new(target.width, target.height, src.pixel_type());

    resizer
        .resize(
            src,
            &mut dst_image,
            &ResizeOptions::new().resize_alg(ResizeAlg::Convolution(FilterType::Bilinear)),
        )
        .map_err(|e| format!("Resize failed: {}", e))?;

    // Non-crop variants borrow dst buffer (no copy).
    // Thumbnail crop needs an owned cropped buffer.
    let (final_width, final_height, final_pixels): (u32, u32, Cow<[u8]>) =
        if let Some(crop_size) = target.crop_square {
            let x = (target.width.saturating_sub(crop_size)) / 2;
            let y = (target.height.saturating_sub(crop_size)) / 2;

            let src_buf = dst_image.buffer();
            let bpp = match src.pixel_type() {
                PixelType::U8x3 => 3,
                PixelType::U8x4 => 4,
                _ => 3,
            };

            let src_stride = target.width as usize * bpp;
            let crop_stride = crop_size as usize * bpp;

            let mut cropped = Vec::with_capacity(crop_size as usize * crop_size as usize * bpp);
            for row in 0..crop_size {
                let off = ((y + row) as usize * src_stride) + (x as usize * bpp);
                cropped.extend_from_slice(&src_buf[off..off + crop_stride]);
            }

            (crop_size, crop_size, Cow::Owned(cropped))
        } else {
            (
                target.width,
                target.height,
                Cow::Borrowed(dst_image.buffer()),
            )
        };

    // EXIF is effectively stripped because we re-encode from raw pixels to WebP (no metadata carryover).
    let webp_data = match src.pixel_type() {
        PixelType::U8x4 => {
            Encoder::from_rgba(&final_pixels, final_width, final_height).encode(WEBP_QUALITY)
        }
        _ => Encoder::from_rgb(&final_pixels, final_width, final_height).encode(WEBP_QUALITY),
    };

    Ok((webp_data.to_vec(), final_width, final_height))
}

fn process_dynamic_image(img: DynamicImage) -> Result<ProcessedImage, String> {
    let (w, h) = img.dimensions();

    // Convert to fast_image_resize::Image without cloning entire buffers
    let src_image = match img {
        DynamicImage::ImageRgb8(rgb) => Image::from_vec_u8(w, h, rgb.into_raw(), PixelType::U8x3)
            .map_err(|e| format!("Failed to create image: {}", e))?,
        DynamicImage::ImageRgba8(rgba) => {
            Image::from_vec_u8(w, h, rgba.into_raw(), PixelType::U8x4)
                .map_err(|e| format!("Failed to create image: {}", e))?
        }
        other => {
            let rgb = other.to_rgb8();
            Image::from_vec_u8(w, h, rgb.into_raw(), PixelType::U8x3)
                .map_err(|e| format!("Failed to create image: {}", e))?
        }
    };

    // Build resize targets
    let mut targets: Vec<ResizeTarget> = Vec::with_capacity(4);

    // 150x150 thumbnail (scale then crop)
    let thumb_scale = 150.0 / (w.min(h) as f32);
    let thumb_w = ((w as f32 * thumb_scale).round() as u32).max(150);
    let thumb_h = ((h as f32 * thumb_scale).round() as u32).max(150);

    targets.push(ResizeTarget {
        width: thumb_w,
        height: thumb_h,
        suffix: "150".to_string(),
        crop_square: Some(150),
    });

    // Responsive widths
    for tw in RESIZE_WIDTHS {
        let nh = ((h as f32) * (tw as f32 / w as f32)).round() as u32;
        targets.push(ResizeTarget {
            width: tw,
            height: nh.max(1),
            suffix: tw.to_string(),
            crop_square: None,
        });
    }

    // Parallel variants (kept)
    let variants: Result<Vec<ImageVariant>, String> = targets
        .par_iter()
        .map(|target| {
            let mut resizer = Resizer::new();
            let (webp_data, final_w, final_h) = resize_to_webp(&src_image, target, &mut resizer)?;
            Ok(ImageVariant {
                suffix: target.suffix.clone(),
                width: final_w,
                height: final_h,
                data: webp_data,
            })
        })
        .collect();

    Ok(ProcessedImage {
        original_width: w,
        original_height: h,
        variants: variants?,
    })
}

// =============================================================================
// GCS operations
// =============================================================================

async fn download_from_gcs(
    client: &GcsClient,
    bucket: &str,
    name: &str,
) -> Result<Vec<u8>, String> {
    let request = GetObjectRequest {
        bucket: bucket.to_string(),
        object: name.to_string(),
        ..Default::default()
    };

    client
        .download_object(&request, &Range::default())
        .await
        .map_err(|e| format!("Failed to download from GCS: {}", e))
}

async fn upload_to_gcs(
    client: &GcsClient,
    bucket: &str,
    name: &str,
    data: Vec<u8>,
    content_type: &str,
) -> Result<(), String> {
    let upload_type = UploadType::Simple(Media {
        name: name.to_string().into(),
        content_type: content_type.to_string().into(),
        content_length: Some(data.len() as u64),
    });

    client
        .upload_object(
            &UploadObjectRequest {
                bucket: bucket.to_string(),
                ..Default::default()
            },
            data,
            &upload_type,
        )
        .await
        .map_err(|e| format!("Failed to upload to GCS: {}", e))?;

    Ok(())
}

async fn upload_manifest(
    client: &GcsClient,
    media_id: &str,
    manifest: &Manifest,
) -> Result<(), String> {
    let json = serde_json::to_string_pretty(manifest)
        .map_err(|e| format!("Failed to serialize manifest: {}", e))?;

    let manifest_path = format!("{}/manifest.json", media_id);

    upload_to_gcs(
        client,
        &manifest_bucket(),
        &manifest_path,
        json.into_bytes(),
        "application/json",
    )
    .await
}

// =============================================================================
// Request handler
// =============================================================================

fn extract_cloud_event_headers(req: &HttpRequest) -> Option<CloudEventHeaders> {
    let get_header = |name: &str| -> Option<String> {
        req.headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(String::from)
    };

    Some(CloudEventHeaders {
        id: get_header("ce-id")?,
        source: get_header("ce-source")?,
        event_type: get_header("ce-type")?,
        subject: get_header("ce-subject"),
    })
}

/// Fast skip using metadata only (real enforcement happens after download via magic-bytes + decode)
fn is_processable_image_by_metadata(name: &str, content_type: Option<&str>) -> bool {
    let lower = name.to_lowercase();
    let by_extension = lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".png")
        || lower.ends_with(".webp");

    let by_content_type = content_type
        .map(|ct| ct.starts_with("image/"))
        .unwrap_or(false);

    by_extension || by_content_type
}

/// Check if this is a folder creation event (not a real file)
fn is_folder_marker(name: &str, size: Option<&str>) -> bool {
    if name.ends_with('/') {
        return true;
    }

    let is_zero_bytes = size.map(|s| s == "0").unwrap_or(false);
    let has_no_extension = !name.contains('.') || name.ends_with('/');

    is_zero_bytes && has_no_extension
}

/// Extract media_id from path like "fold-097/photo.jpg" -> "fold-097"
fn extract_media_id(name: &str) -> String {
    if let Some((folder, _)) = name.split_once('/') {
        folder.to_string()
    } else {
        name.rsplit_once('.')
            .map(|(s, _)| s)
            .unwrap_or(name)
            .to_string()
    }
}

async fn handle_gcs_event(
    req: HttpRequest,
    body: web::Bytes,
    gcs_client: web::Data<Arc<GcsClient>>,
) -> HttpResponse {
    let total_start = Instant::now();

    // Extract CloudEvent metadata
    let headers = match extract_cloud_event_headers(&req) {
        Some(h) => h,
        None => {
            warn!("Missing required CloudEvent headers");
            return HttpResponse::BadRequest().json(FunctionResponse {
                status: "error".to_string(),
                message: "Missing required CloudEvent headers".to_string(),
                variants_created: None,
            });
        }
    };

    info!(
        event_id = %headers.id,
        event_type = %headers.event_type,
        "Received CloudEvent"
    );

    // Only process finalize events
    if headers.event_type != "google.cloud.storage.object.v1.finalized" {
        return HttpResponse::Ok().json(FunctionResponse {
            status: "ignored".to_string(),
            message: format!("Event type {} not handled", headers.event_type),
            variants_created: None,
        });
    }

    // Parse GCS object data
    let gcs_data: GcsObjectData = match serde_json::from_slice(&body) {
        Ok(data) => data,
        Err(e) => {
            error!(error = %e, "Failed to parse CloudEvent data");
            return HttpResponse::BadRequest().json(FunctionResponse {
                status: "error".to_string(),
                message: format!("Failed to parse event data: {}", e),
                variants_created: None,
            });
        }
    };

    info!(
        bucket = %gcs_data.bucket,
        file_name = %gcs_data.name,
        content_type = ?gcs_data.content_type,
        "Processing upload event"
    );

    let media_id = extract_media_id(&gcs_data.name);
    let client = gcs_client.get_ref();

    // Skip folder creation events
    if is_folder_marker(&gcs_data.name, gcs_data.size.as_deref()) {
        info!(file_name = %gcs_data.name, "Skipping folder marker");
        return HttpResponse::Ok().json(FunctionResponse {
            status: "skipped".to_string(),
            message: "Folder marker, not a file".to_string(),
            variants_created: None,
        });
    }

    // Fast skip non-image by metadata (enforcement happens after download)
    if !is_processable_image_by_metadata(&gcs_data.name, gcs_data.content_type.as_deref()) {
        info!(file_name = %gcs_data.name, "Skipping non-image file");

        let manifest = failed_manifest(
            media_id.clone(),
            "INVALID_FILE_TYPE",
            "Not a processable image".to_string(),
            "validation",
        );
        let _ = upload_manifest(client, &media_id, &manifest).await;

        return HttpResponse::Ok().json(FunctionResponse {
            status: "skipped".to_string(),
            message: "Not a processable image".to_string(),
            variants_created: None,
        });
    }

    // Download original
    let download_start = Instant::now();
    let image_bytes = match download_from_gcs(client, &gcs_data.bucket, &gcs_data.name).await {
        Ok(bytes) => bytes,
        Err(e) => {
            error!(error = %e, "Failed to download image");

            let manifest =
                failed_manifest(media_id.clone(), "DOWNLOAD_ERROR", e.clone(), "download");
            let _ = upload_manifest(client, &media_id, &manifest).await;

            return HttpResponse::InternalServerError().json(FunctionResponse {
                status: "error".to_string(),
                message: e,
                variants_created: None,
            });
        }
    };
    let download_ms = download_start.elapsed().as_millis() as u64;

    // Validate + decode + process (CPU)
    let processing_start = Instant::now();
    let processed = match tokio::task::spawn_blocking(move || {
        let (img, _fmt) = validate_and_decode(&image_bytes)?;
        process_dynamic_image(img).map_err(|msg| RuleError {
            code: RuleCode::DecodeFailed,
            message: msg,
            stage: "processing",
        })
    })
    .await
    {
        Ok(Ok(p)) => p,
        Ok(Err(rule_err)) => {
            error!(error = ?rule_err, "Validation/processing failed");
            let manifest = failed_manifest_from_rule(media_id.clone(), rule_err);
            let _ = upload_manifest(client, &media_id, &manifest).await;

            return HttpResponse::Ok().json(FunctionResponse {
                status: "skipped".to_string(),
                message: "Business rule validation failed".to_string(),
                variants_created: None,
            });
        }
        Err(e) => {
            error!(error = %e, "Task panicked");
            let manifest = failed_manifest(
                media_id.clone(),
                "INTERNAL_ERROR",
                "Internal processing error".to_string(),
                "processing",
            );
            let _ = upload_manifest(client, &media_id, &manifest).await;

            return HttpResponse::InternalServerError().json(FunctionResponse {
                status: "error".to_string(),
                message: "Internal processing error".to_string(),
                variants_created: None,
            });
        }
    };
    let processing_ms = processing_start.elapsed().as_millis() as u64;

    // Output naming
    let stem = Path::new(&gcs_data.name)
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or(&gcs_data.name);

    let out_bucket = output_bucket();

    // Manifest variant info
    let variant_info: Vec<ManifestVariant> = processed
        .variants
        .iter()
        .map(|v| {
            let size: u32 = v.suffix.parse().unwrap_or(0);
            ManifestVariant {
                size,
                path: format!("variants/{}/{}_{}.webp", media_id, stem, v.suffix),
                width: v.width,
                height: v.height,
            }
        })
        .collect();

    // Build upload futures for variants
    let upload_futures: Vec<_> = processed
        .variants
        .into_iter()
        .map(|variant| {
            let output_name = format!("variants/{}/{}_{}.webp", media_id, stem, variant.suffix);
            let bucket = out_bucket.clone();
            let client = client.clone();

            async move {
                upload_to_gcs(&client, &bucket, &output_name, variant.data, "image/webp").await?;
                Ok::<String, String>(output_name)
            }
        })
        .collect();

    // Upload timing (variants + manifest)
    let upload_start = Instant::now();

    let variant_results = join_all(upload_futures).await;

    let mut created = Vec::new();
    let mut errors = Vec::new();
    for r in variant_results {
        match r {
            Ok(name) => created.push(name),
            Err(e) => errors.push(e),
        }
    }

    if !errors.is_empty() {
        error!(?errors, "Some uploads failed");

        let manifest = failed_manifest(
            media_id.clone(),
            "UPLOAD_ERROR",
            format!("Some variant uploads failed: {:?}", errors),
            "upload",
        );
        let _ = upload_manifest(client, &media_id, &manifest).await;

        return HttpResponse::InternalServerError().json(FunctionResponse {
            status: "partial_error".to_string(),
            message: format!("Some uploads failed: {:?}", errors),
            variants_created: Some(created),
        });
    }

    // Build final manifest with correct metrics (upload_ms + total_ms are computed after manifest upload)
    let mut ready_manifest = Manifest::Ready {
        media_id: media_id.clone(),
        pipeline_version: PIPELINE_VERSION.to_string(),
        updated_at: now_iso8601(),
        original: ManifestOriginal {
            bucket: gcs_data.bucket.clone(),
            path: gcs_data.name.clone(),
            width: Some(processed.original_width),
            height: Some(processed.original_height),
        },
        variants: variant_info,
        metrics: ManifestMetrics {
            total_ms: 0,
            download_ms,
            processing_ms,
            upload_ms: 0,
            encoder: "webp".to_string(),
            quality: WEBP_QUALITY as u32,
        },
    };

    // Upload manifest
    if let Err(e) = upload_manifest(client, &media_id, &ready_manifest).await {
        error!(error = %e, "Failed to upload manifest");
        let manifest = failed_manifest(
            media_id.clone(),
            "UPLOAD_ERROR",
            format!("Manifest upload failed: {}", e),
            "upload",
        );
        let _ = upload_manifest(client, &media_id, &manifest).await;

        return HttpResponse::InternalServerError().json(FunctionResponse {
            status: "error".to_string(),
            message: format!("Manifest upload failed: {}", e),
            variants_created: Some(created),
        });
    }

    // Now we can finalize upload_ms/total_ms accurately (includes manifest upload above)
    let upload_ms = upload_start.elapsed().as_millis() as u64;
    let total_ms = total_start.elapsed().as_millis() as u64;

    // Overwrite manifest once with final metrics (cheap + guarantees correctness)
    if let Manifest::Ready { metrics, .. } = &mut ready_manifest {
        metrics.upload_ms = upload_ms;
        metrics.total_ms = total_ms;
    }
    if let Err(e) = upload_manifest(client, &media_id, &ready_manifest).await {
        error!(error = %e, "Failed to upload final manifest with metrics");
        return HttpResponse::InternalServerError().json(FunctionResponse {
            status: "error".to_string(),
            message: format!("Failed to upload final manifest with metrics: {}", e),
            variants_created: Some(created),
        });
    }

    info!(
        variants_created = ?created,
        download_ms,
        processing_ms,
        upload_ms,
        total_ms,
        "Successfully processed image"
    );

    HttpResponse::Ok().json(FunctionResponse {
        status: "success".to_string(),
        message: format!("Created {} variants", created.len()),
        variants_created: Some(created),
    })
}

async fn health() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({ "status": "healthy" }))
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Configure rayon thread pool
    let num_threads = std::env::var("RAYON_NUM_THREADS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(2)
        });

    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()
        .expect("Failed to build rayon thread pool");

    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_target(false)
        .with_current_span(false)
        .flatten_event(true)
        .init();

    let gcs_config = ClientConfig::default()
        .with_auth()
        .await
        .expect("Failed to create GCS client config");
    let gcs_client = Arc::new(GcsClient::new(gcs_config));

    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .expect("PORT must be a valid u16");

    info!(
        port = port,
        output_bucket = %output_bucket(),
        manifest_bucket = %manifest_bucket(),
        rayon_threads = num_threads,
        "Starting image processing function"
    );

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(gcs_client.clone()))
            .route("/", web::post().to(handle_gcs_event))
            .route("/health", web::get().to(health))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
