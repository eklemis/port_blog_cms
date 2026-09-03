//! An application's cover letter and its reflection.

use actix_web::{delete, get, patch, put, web, HttpResponse, Responder};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::error;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    api::schemas::{ErrorResponse, SuccessResponse},
    auth::{
        adapter::incoming::web::extractors::auth::VerifiedUser,
        application::domain::entities::UserId,
    },
    career::application::ports::incoming::use_cases::LetterError,
    career::application::ports::outgoing::{PatchCoverLetterData, ReflectionData},
    career::domain::entities::{CoverLetter, CoverLetterStatus, Reflection},
    shared::api::{ApiResponse, ErrorCode},
    AppState,
};

/// A cover letter as returned by the API.
#[derive(Debug, Serialize, ToSchema)]
pub struct CoverLetterResponse {
    /// The application it belongs to.
    pub application_id: Uuid,
    /// Markdown, like a post body.
    pub content: String,
    /// The letter's own language — not the writer's interface language.
    pub language: String,
    /// Whether it has gone out.
    pub status: CoverLetterStatus,
    /// When it was started.
    pub created_at: DateTime<Utc>,
    /// Last edit.
    pub updated_at: DateTime<Utc>,
}

impl From<CoverLetter> for CoverLetterResponse {
    fn from(l: CoverLetter) -> Self {
        Self {
            application_id: l.application_id,
            content: l.content,
            language: l.language,
            status: l.status,
            created_at: l.created_at,
            updated_at: l.updated_at,
        }
    }
}

/// Body for writing a cover letter. Omitted fields are left alone.
#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct PatchCoverLetterRequest {
    /// New body, Markdown.
    pub content: Option<String>,

    /// The language to write in.
    ///
    /// **Explicit, never inferred.** Guessing from the existing text breaks on
    /// a half-written letter, and this is what tells a generator which language
    /// to produce.
    pub language: Option<String>,

    /// `draft` or `sent`.
    pub status: Option<CoverLetterStatus>,
}

/// A reflection as returned by the API.
#[derive(Debug, Serialize, ToSchema)]
pub struct ReflectionResponse {
    /// The application it belongs to.
    pub application_id: Uuid,
    /// How far it got.
    pub stage_reached: String,
    /// What happened.
    pub what_happened: String,
    /// What they would change.
    pub what_id_change: String,
    /// When it was written.
    pub created_at: DateTime<Utc>,
    /// Last edit.
    pub updated_at: DateTime<Utc>,
}

impl From<Reflection> for ReflectionResponse {
    fn from(r: Reflection) -> Self {
        Self {
            application_id: r.application_id,
            stage_reached: r.stage_reached,
            what_happened: r.what_happened,
            what_id_change: r.what_id_change,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

/// Body for writing a reflection. All three questions are optional to answer.
#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct PutReflectionRequest {
    /// How far it got, in your own words.
    #[serde(default)]
    pub stage_reached: String,
    /// What happened.
    #[serde(default)]
    pub what_happened: String,
    /// What you would do differently.
    #[serde(default)]
    pub what_id_change: String,
}

fn map_error(e: LetterError) -> HttpResponse {
    match e {
        LetterError::NotFound => {
            ApiResponse::not_found(ErrorCode::ApplicationNotFound, "Not found")
        }
        LetterError::RepositoryError(e) => {
            error!("Repository error on a cover letter or reflection: {}", e);
            ApiResponse::internal_error()
        }
    }
}

/// Read an application's cover letter
#[utoipa::path(
    get,
    path = "/api/applications/{application_id}/cover-letter",
    tag = "career",
    params(("application_id" = Uuid, Path, description = "The application")),
    responses(
        (status = 200, description = "The letter", body = inline(SuccessResponse<CoverLetterResponse>)),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
        (status = 404, description = "No letter, or the application is not yours", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[get("/api/applications/{application_id}/cover-letter")]
pub async fn get_cover_letter_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> impl Responder {
    match data
        .career
        .cover_letter
        .get(UserId::from(user.user_id), path.into_inner())
        .await
    {
        Ok(letter) => ApiResponse::success(CoverLetterResponse::from(letter)),
        Err(e) => map_error(e),
    }
}

/// Write an application's cover letter
///
/// Creates it on first write. Omitted fields keep whatever is stored, matching
/// the blog editor's semantics, so a partial save behaves the way the editor
/// already expects.
#[utoipa::path(
    patch,
    path = "/api/applications/{application_id}/cover-letter",
    tag = "career",
    params(("application_id" = Uuid, Path, description = "The application")),
    request_body = PatchCoverLetterRequest,
    responses(
        (status = 200, description = "The stored letter", body = inline(SuccessResponse<CoverLetterResponse>)),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
        (status = 404, description = "The application is not yours", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[patch("/api/applications/{application_id}/cover-letter")]
pub async fn patch_cover_letter_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    body: web::Json<PatchCoverLetterRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let b = body.into_inner();

    match data
        .career
        .cover_letter
        .write(
            UserId::from(user.user_id),
            path.into_inner(),
            PatchCoverLetterData {
                content: b.content,
                language: b.language,
                status: b.status,
            },
        )
        .await
    {
        Ok(letter) => ApiResponse::success(CoverLetterResponse::from(letter)),
        Err(e) => map_error(e),
    }
}

/// Delete an application's cover letter
#[utoipa::path(
    delete,
    path = "/api/applications/{application_id}/cover-letter",
    tag = "career",
    params(("application_id" = Uuid, Path, description = "The application")),
    responses(
        (status = 204, description = "Deleted"),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[delete("/api/applications/{application_id}/cover-letter")]
pub async fn delete_cover_letter_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> impl Responder {
    match data
        .career
        .cover_letter
        .delete(UserId::from(user.user_id), path.into_inner())
        .await
    {
        Ok(()) => ApiResponse::no_content(),
        Err(e) => map_error(e),
    }
}

/// Read an application's reflection
///
/// Private to its author. Nothing else reads this — see
/// `docs/adr/0009-reflections-never-feed-generation.md`.
#[utoipa::path(
    get,
    path = "/api/applications/{application_id}/reflection",
    tag = "career",
    params(("application_id" = Uuid, Path, description = "The application")),
    responses(
        (status = 200, description = "The reflection", body = inline(SuccessResponse<ReflectionResponse>)),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
        (status = 404, description = "No reflection, or the application is not yours", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[get("/api/applications/{application_id}/reflection")]
pub async fn get_reflection_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> impl Responder {
    match data
        .career
        .reflection
        .get(UserId::from(user.user_id), path.into_inner())
        .await
    {
        Ok(reflection) => ApiResponse::success(ReflectionResponse::from(reflection)),
        Err(e) => map_error(e),
    }
}

/// Write an application's reflection
///
/// Written whole rather than patched: the three questions are answered in one
/// sitting, and a partial update would let a half-finished thought overwrite a
/// finished one field by field.
///
/// **This is the most sensitive data the product holds.** It never enters a
/// prompt that produces user-facing content — not a CV bullet, not a cover
/// letter, not a tailoring suggestion. See
/// `docs/adr/0009-reflections-never-feed-generation.md`.
#[utoipa::path(
    put,
    path = "/api/applications/{application_id}/reflection",
    tag = "career",
    params(("application_id" = Uuid, Path, description = "The application")),
    request_body = PutReflectionRequest,
    responses(
        (status = 200, description = "The stored reflection", body = inline(SuccessResponse<ReflectionResponse>)),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
        (status = 404, description = "The application is not yours", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[put("/api/applications/{application_id}/reflection")]
pub async fn put_reflection_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    body: web::Json<PutReflectionRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let b = body.into_inner();

    match data
        .career
        .reflection
        .write(
            UserId::from(user.user_id),
            path.into_inner(),
            ReflectionData {
                stage_reached: b.stage_reached,
                what_happened: b.what_happened,
                what_id_change: b.what_id_change,
            },
        )
        .await
    {
        Ok(reflection) => ApiResponse::success(ReflectionResponse::from(reflection)),
        Err(e) => map_error(e),
    }
}

/// Delete an application's reflection
///
/// Real deletion, not a flag. Someone withdrawing a private note about their
/// own rejection should not later discover it was only hidden.
#[utoipa::path(
    delete,
    path = "/api/applications/{application_id}/reflection",
    tag = "career",
    params(("application_id" = Uuid, Path, description = "The application")),
    responses(
        (status = 204, description = "Deleted permanently"),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[delete("/api/applications/{application_id}/reflection")]
pub async fn delete_reflection_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> impl Responder {
    match data
        .career
        .reflection
        .delete(UserId::from(user.user_id), path.into_inner())
        .await
    {
        Ok(()) => ApiResponse::no_content(),
        Err(e) => map_error(e),
    }
}
