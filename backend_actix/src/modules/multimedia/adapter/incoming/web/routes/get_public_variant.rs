//! `GET /api/public/media/{media_id}/{size}` — the public read path for media.

use actix_web::{http::header, web, HttpResponse, Responder};
use tracing::error;
use uuid::Uuid;

use crate::api::schemas::ErrorResponse;
use crate::multimedia::application::domain::entities::MediaSize;
use crate::multimedia::application::ports::incoming::use_cases::GetPublicVariantUrlError;
use crate::shared::api::{ApiResponse, ErrorCode};
use crate::AppState;

/// How long the browser may reuse the redirect itself.
///
/// Shorter than the signed URL it points at, so a cached redirect never
/// outlives its own target. Long enough that a reader scrolling a gallery does
/// not re-hit the API for every thumbnail.
const REDIRECT_MAX_AGE_SECS: u32 = 300;

/// Redirect a reader to a signed URL for one media variant
///
/// Public: no token required. The bucket is private, so this endpoint is the
/// only way a reader reaches an object — it checks the media is attached to
/// something published, signs a short-lived URL, and redirects.
///
/// Unpublishing the post that carries the media makes this 404 from then on,
/// which is the property a world-readable bucket cannot provide.
#[utoipa::path(
    get,
    path = "/api/public/media/{media_id}/{size}",
    tag = "media",
    params(
        ("media_id" = Uuid, Path, description = "Media identifier"),
        ("size" = String, Path, description = "thumbnail, small, medium or large"),
    ),
    responses(
        (status = 302, description = "Redirect to a short-lived signed URL"),
        (
            status = 404,
            description = "Unknown variant, or not attached to anything published",
            body = ErrorResponse
        ),
        (status = 502, description = "Object store unavailable", body = ErrorResponse),
    )
)]
#[actix_web::get("/api/public/media/{media_id}/{size}")]
pub async fn get_public_variant_handler(
    path: web::Path<(Uuid, String)>,
    data: web::Data<AppState>,
) -> impl Responder {
    let (media_id, size_raw) = path.into_inner();

    let Ok(size) = size_raw.parse::<MediaSize>() else {
        // An unparseable size is reported the same way as a missing one, so the
        // endpoint says nothing about which sizes exist.
        return ApiResponse::not_found(ErrorCode::VariantNotFound, "Invalid media size");
    };

    match data
        .multimedia
        .get_public_variant_url
        .execute(media_id, size)
        .await
    {
        Ok(url) => HttpResponse::Found()
            .insert_header((header::LOCATION, url))
            .insert_header((
                header::CACHE_CONTROL,
                format!("public, max-age={REDIRECT_MAX_AGE_SECS}"),
            ))
            .finish(),
        Err(GetPublicVariantUrlError::NotFound) => {
            ApiResponse::not_found(ErrorCode::MediaNotFound, "Media not found")
        }
        Err(GetPublicVariantUrlError::StorageError(e)) => {
            error!("Failed to sign a public media URL: {}", e);
            ApiResponse::error(
                actix_web::http::StatusCode::BAD_GATEWAY,
                ErrorCode::StorageError,
                "Storage unavailable",
            )
        }
        Err(GetPublicVariantUrlError::QueryError(e)) => {
            error!("Failed to look up a public media variant: {}", e);
            ApiResponse::internal_error()
        }
    }
}
