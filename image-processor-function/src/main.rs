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
struct ManifestExpectedVariant {
    size: u32,
    path: String,
}

#[derive(Serialize)]
struct ManifestMetrics {
    processing_ms: u64,
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
    #[serde(rename = "processing")]
    Processing {
        media_id: String,
        pipeline_version: String,
        updated_at: String,
        original: ManifestOriginal,
        expected_variants: Vec<ManifestExpectedVariant>,
    },
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

// =============================================================================
// Image processing with rayon parallelism
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

    // Handle optional center crop for thumbnails
    let final_buffer: Vec<u8>;
    let (final_width, final_height) = if let Some(crop_size) = target.crop_square {
        let x = (target.width.saturating_sub(crop_size)) / 2;
        let y = (target.height.saturating_sub(crop_size)) / 2;

        let src_buf = dst_image.buffer();
        let bytes_per_pixel = match src.pixel_type() {
            PixelType::U8x3 => 3,
            PixelType::U8x4 => 4,
            _ => 3,
        };
        let src_stride = target.width as usize * bytes_per_pixel;
        let crop_stride = crop_size as usize * bytes_per_pixel;

        let mut cropped =
            Vec::with_capacity(crop_size as usize * crop_size as usize * bytes_per_pixel);
        for row in 0..crop_size {
            let src_offset = ((y + row) as usize * src_stride) + (x as usize * bytes_per_pixel);
            cropped.extend_from_slice(&src_buf[src_offset..src_offset + crop_stride]);
        }
        final_buffer = cropped;
        (crop_size, crop_size)
    } else {
        final_buffer = dst_image.buffer().to_vec();
        (target.width, target.height)
    };

    // Encode to WebP
    let webp_data = match src.pixel_type() {
        PixelType::U8x4 => {
            Encoder::from_rgba(&final_buffer, final_width, final_height).encode(WEBP_QUALITY)
        }
        _ => Encoder::from_rgb(&final_buffer, final_width, final_height).encode(WEBP_QUALITY),
    };

    Ok((webp_data.to_vec(), final_width, final_height))
}

fn process_image(bytes: &[u8]) -> Result<ProcessedImage, String> {
    // Decode image
    let img = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| format!("Failed to guess format: {}", e))?
        .decode()
        .map_err(|e| format!("Failed to decode image: {}", e))?;

    let (w, h) = img.dimensions();

    // Convert to fast_image_resize Image format
    let src_image = match &img {
        DynamicImage::ImageRgb8(rgb) => {
            Image::from_vec_u8(w, h, rgb.as_raw().to_vec(), PixelType::U8x3)
                .map_err(|e| format!("Failed to create image: {}", e))?
        }
        DynamicImage::ImageRgba8(rgba) => {
            Image::from_vec_u8(w, h, rgba.as_raw().to_vec(), PixelType::U8x4)
                .map_err(|e| format!("Failed to create image: {}", e))?
        }
        _ => {
            let rgb = img.to_rgb8();
            Image::from_vec_u8(w, h, rgb.into_raw(), PixelType::U8x3)
                .map_err(|e| format!("Failed to create image: {}", e))?
        }
    };

    // Drop original to free memory
    drop(img);

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

    // Process targets in parallel using rayon
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

fn is_processable_image(name: &str, content_type: Option<&str>) -> bool {
    let by_extension = name.to_lowercase().ends_with(".jpg")
        || name.to_lowercase().ends_with(".jpeg")
        || name.to_lowercase().ends_with(".png")
        || name.to_lowercase().ends_with(".webp");

    let by_content_type = content_type
        .map(|ct| ct.starts_with("image/"))
        .unwrap_or(false);

    by_extension || by_content_type
}

/// Check if this is a folder creation event (not a real file)
/// GCS folders are zero-byte objects ending with /
fn is_folder_marker(name: &str, size: Option<&str>) -> bool {
    // Folder markers end with /
    if name.ends_with('/') {
        return true;
    }

    // Also check for zero-byte objects without extension (likely folder placeholders)
    let is_zero_bytes = size.map(|s| s == "0").unwrap_or(false);
    let has_no_extension = !name.contains('.') || name.ends_with('/');

    is_zero_bytes && has_no_extension
}

/// Extract media_id from path like "quarantine/abc123/photo.jpg" -> "abc123"
/// or "quarantine/photo.jpg" -> "photo"
fn extract_media_id(name: &str) -> String {
    let without_quarantine = name.trim_start_matches("quarantine/");

    // Check if there's a subfolder structure
    if let Some((folder, _)) = without_quarantine.split_once('/') {
        folder.to_string()
    } else {
        // No subfolder, use filename without extension as media_id
        without_quarantine
            .rsplit_once('.')
            .map(|(s, _)| s)
            .unwrap_or(without_quarantine)
            .to_string()
    }
}

/// Build expected variant paths for processing manifest
fn build_expected_variants(media_id: &str) -> Vec<ManifestExpectedVariant> {
    let sizes = [150u32, 320, 768, 1200];
    sizes
        .iter()
        .map(|&size| ManifestExpectedVariant {
            size,
            path: format!("variants/{}/{}.webp", media_id, size),
        })
        .collect()
}

async fn handle_gcs_event(
    req: HttpRequest,
    body: web::Bytes,
    gcs_client: web::Data<Arc<GcsClient>>,
) -> HttpResponse {
    let start_time = Instant::now();

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

    // Skip folder creation events entirely (no manifest)
    if is_folder_marker(&gcs_data.name, gcs_data.size.as_deref()) {
        info!(file_name = %gcs_data.name, "Skipping folder marker");
        return HttpResponse::Ok().json(FunctionResponse {
            status: "skipped".to_string(),
            message: "Folder marker, not a file".to_string(),
            variants_created: None,
        });
    }

    // Check if this is an image we should process
    if !is_processable_image(&gcs_data.name, gcs_data.content_type.as_deref()) {
        info!(file_name = %gcs_data.name, "Skipping non-image file");

        // Upload failed manifest
        let manifest = Manifest::Failed {
            media_id: media_id.clone(),
            pipeline_version: PIPELINE_VERSION.to_string(),
            updated_at: now_iso8601(),
            error: ManifestError {
                code: "INVALID_FILE_TYPE".to_string(),
                message: "Not a processable image file".to_string(),
                stage: "validation".to_string(),
            },
        };
        if let Err(e) = upload_manifest(client, &media_id, &manifest).await {
            error!(error = %e, "Failed to upload failed manifest");
        }

        return HttpResponse::Ok().json(FunctionResponse {
            status: "skipped".to_string(),
            message: "Not a processable image".to_string(),
            variants_created: None,
        });
    }

    // Upload processing manifest
    let processing_manifest = Manifest::Processing {
        media_id: media_id.clone(),
        pipeline_version: PIPELINE_VERSION.to_string(),
        updated_at: now_iso8601(),
        original: ManifestOriginal {
            bucket: gcs_data.bucket.clone(),
            path: gcs_data.name.clone(),
            width: None,
            height: None,
        },
        expected_variants: build_expected_variants(&media_id),
    };
    if let Err(e) = upload_manifest(client, &media_id, &processing_manifest).await {
        error!(error = %e, "Failed to upload processing manifest");
    }

    // Download the original image
    let image_bytes = match download_from_gcs(client, &gcs_data.bucket, &gcs_data.name).await {
        Ok(bytes) => bytes,
        Err(e) => {
            error!(error = %e, "Failed to download image");

            // Upload failed manifest
            let manifest = Manifest::Failed {
                media_id: media_id.clone(),
                pipeline_version: PIPELINE_VERSION.to_string(),
                updated_at: now_iso8601(),
                error: ManifestError {
                    code: "DOWNLOAD_ERROR".to_string(),
                    message: e.clone(),
                    stage: "download".to_string(),
                },
            };
            if let Err(me) = upload_manifest(client, &media_id, &manifest).await {
                error!(error = %me, "Failed to upload failed manifest");
            }

            return HttpResponse::InternalServerError().json(FunctionResponse {
                status: "error".to_string(),
                message: e,
                variants_created: None,
            });
        }
    };

    info!(size_bytes = image_bytes.len(), "Downloaded original image");

    // Process image (CPU-intensive, run in blocking thread pool)
    let processed = match tokio::task::spawn_blocking(move || process_image(&image_bytes)).await {
        Ok(Ok(p)) => p,
        Ok(Err(e)) => {
            error!(error = %e, "Image processing failed");

            // Upload failed manifest
            let manifest = Manifest::Failed {
                media_id: media_id.clone(),
                pipeline_version: PIPELINE_VERSION.to_string(),
                updated_at: now_iso8601(),
                error: ManifestError {
                    code: "DECODE_ERROR".to_string(),
                    message: e.clone(),
                    stage: "decode".to_string(),
                },
            };
            if let Err(me) = upload_manifest(client, &media_id, &manifest).await {
                error!(error = %me, "Failed to upload failed manifest");
            }

            return HttpResponse::InternalServerError().json(FunctionResponse {
                status: "error".to_string(),
                message: e,
                variants_created: None,
            });
        }
        Err(e) => {
            error!(error = %e, "Task panicked");

            // Upload failed manifest
            let manifest = Manifest::Failed {
                media_id: media_id.clone(),
                pipeline_version: PIPELINE_VERSION.to_string(),
                updated_at: now_iso8601(),
                error: ManifestError {
                    code: "INTERNAL_ERROR".to_string(),
                    message: "Internal processing error".to_string(),
                    stage: "processing".to_string(),
                },
            };
            if let Err(me) = upload_manifest(client, &media_id, &manifest).await {
                error!(error = %me, "Failed to upload failed manifest");
            }

            return HttpResponse::InternalServerError().json(FunctionResponse {
                status: "error".to_string(),
                message: "Internal processing error".to_string(),
                variants_created: None,
            });
        }
    };

    let output_bucket = output_bucket();

    // Build variant info for manifest before moving data
    let variant_info: Vec<(u32, String, u32, u32)> = processed
        .variants
        .iter()
        .map(|v| {
            let size: u32 = v.suffix.parse().unwrap_or(0);
            let path = format!("variants/{}/{}.webp", media_id, v.suffix);
            (size, path, v.width, v.height)
        })
        .collect();

    // Upload all variants concurrently
    let stem = Path::new(&gcs_data.name)
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or(&gcs_data.name);
    let upload_futures: Vec<_> = processed
        .variants
        .into_iter()
        .map(|variant| {
            let output_name = format!("variants/{}/{}_{}.webp", media_id, &stem, variant.suffix);
            let bucket = output_bucket.clone();
            let client = client.clone();

            async move {
                upload_to_gcs(&client, &bucket, &output_name, variant.data, "image/webp").await?;
                Ok::<String, String>(output_name)
            }
        })
        .collect();

    let results = join_all(upload_futures).await;

    // Collect results
    let mut created = Vec::new();
    let mut errors = Vec::new();

    for result in results {
        match result {
            Ok(name) => created.push(name),
            Err(e) => errors.push(e),
        }
    }

    if !errors.is_empty() {
        error!(?errors, "Some uploads failed");

        // Upload failed manifest
        let manifest = Manifest::Failed {
            media_id: media_id.clone(),
            pipeline_version: PIPELINE_VERSION.to_string(),
            updated_at: now_iso8601(),
            error: ManifestError {
                code: "UPLOAD_ERROR".to_string(),
                message: format!("Some uploads failed: {:?}", errors),
                stage: "upload".to_string(),
            },
        };
        if let Err(me) = upload_manifest(client, &media_id, &manifest).await {
            error!(error = %me, "Failed to upload failed manifest");
        }

        return HttpResponse::InternalServerError().json(FunctionResponse {
            status: "partial_error".to_string(),
            message: format!("Some uploads failed: {:?}", errors),
            variants_created: Some(created),
        });
    }

    let processing_ms = start_time.elapsed().as_millis() as u64;

    // Upload ready manifest
    let ready_manifest = Manifest::Ready {
        media_id: media_id.clone(),
        pipeline_version: PIPELINE_VERSION.to_string(),
        updated_at: now_iso8601(),
        original: ManifestOriginal {
            bucket: gcs_data.bucket.clone(),
            path: gcs_data.name.clone(),
            width: Some(processed.original_width),
            height: Some(processed.original_height),
        },
        variants: variant_info
            .into_iter()
            .map(|(size, path, width, height)| ManifestVariant {
                size,
                path,
                width,
                height,
            })
            .collect(),
        metrics: ManifestMetrics {
            processing_ms,
            encoder: "webp".to_string(),
            quality: WEBP_QUALITY as u32,
        },
    };
    if let Err(e) = upload_manifest(client, &media_id, &ready_manifest).await {
        error!(error = %e, "Failed to upload ready manifest");
    }

    info!(
        variants_created = ?created,
        processing_ms = processing_ms,
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
    // Configure rayon to use available CPUs (2 vCPUs on Cloud Run)
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
