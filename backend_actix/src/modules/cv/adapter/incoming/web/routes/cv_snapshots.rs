//! Taking a CV snapshot, and reading one back.

use actix_web::{get, post, web, Responder};
use chrono::{DateTime, Utc};
use serde::Serialize;
use tracing::error;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    api::schemas::{ErrorResponse, SuccessResponse},
    auth::{
        adapter::incoming::web::extractors::auth::VerifiedUser,
        application::domain::entities::UserId,
    },
    cv::adapter::incoming::web::dto::CvResponse,
    cv::application::ports::outgoing::CvSnapshot,
    cv::application::use_cases::cv_snapshots::CvSnapshotError,
    shared::api::{ApiResponse, ErrorCode},
    AppState,
};

/// What a freshly taken snapshot reports.
#[derive(Debug, Serialize, ToSchema)]
pub struct CvSnapshotCreated {
    /// The snapshot's identifier. Store this on the application.
    pub snapshot_id: Uuid,

    /// When it was taken — the "as sent" date the tracker shows.
    pub created_at: DateTime<Utc>,
}

/// A snapshot read back.
#[derive(Debug, Serialize, ToSchema)]
pub struct CvSnapshotResponse {
    /// The snapshot's identifier.
    pub snapshot_id: Uuid,

    /// The CV it was taken from. That CV has probably changed since; this is
    /// here so a client can offer "start a new version from this", not so it
    /// can go and read the current one instead.
    pub cv_id: Uuid,

    /// When it was taken.
    pub created_at: DateTime<Utc>,

    /// The CV exactly as it stood. Read-only, always.
    pub document: CvResponse,
}

fn map_error(e: CvSnapshotError) -> actix_web::HttpResponse {
    match e {
        CvSnapshotError::CvNotFound => {
            ApiResponse::not_found(ErrorCode::CvNotFound, "CV not found")
        }
        CvSnapshotError::SnapshotNotFound => {
            ApiResponse::not_found(ErrorCode::CvNotFound, "Snapshot not found")
        }
        CvSnapshotError::RepositoryError(e) => {
            error!("Repository error on a CV snapshot: {}", e);
            ApiResponse::internal_error()
        }
    }
}

fn to_response(snapshot: CvSnapshot) -> CvSnapshotResponse {
    CvSnapshotResponse {
        snapshot_id: snapshot.id,
        cv_id: snapshot.cv_id,
        created_at: snapshot.created_at,
        document: snapshot.document.into(),
    }
}

/// Freeze a CV as it stands
///
/// Takes an immutable copy for an application to point at. Without one, the
/// tracker links to a living document: keep editing the CV and every past
/// application retroactively claims to have used a version that did not exist
/// when it was sent.
///
/// Deliberately **not** idempotent. Two applications sent a week apart each get
/// their own snapshot, even if the CV did not change in between.
#[utoipa::path(
    post,
    path = "/api/cvs/{cv_id}/snapshot",
    tag = "cvs",
    params(("cv_id" = Uuid, Path, description = "The CV to freeze")),
    responses(
        (
            status = 201,
            description = "Snapshot taken",
            body = inline(SuccessResponse<CvSnapshotCreated>),
            example = json!({
                "success": true,
                "data": {
                    "snapshot_id": "8f1b2c3d-4e5f-6a7b-8c9d-0e1f2a3b4c5d",
                    "created_at": "2026-09-03T09:00:00Z"
                }
            })
        ),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
        (status = 404, description = "No such CV, or it is not yours", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[post("/api/cvs/{cv_id}/snapshot")]
pub async fn create_cv_snapshot_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> impl Responder {
    match data
        .cv_snapshot
        .create
        .execute(UserId::from(user.user_id), path.into_inner())
        .await
    {
        Ok(snapshot) => ApiResponse::created(CvSnapshotCreated {
            snapshot_id: snapshot.id,
            created_at: snapshot.created_at,
        }),
        Err(e) => map_error(e),
    }
}

/// Read a snapshot back
///
/// Returns the CV exactly as it was sent. There is no update path — if the
/// author wants to work from it, the client offers "start a new version from
/// this", which creates an ordinary CV rather than editing history.
///
/// Owner-scoped: a snapshot is a record of what *you* sent. The CV it came
/// from may be public, but by the time anyone asks, the two are different
/// documents.
#[utoipa::path(
    get,
    path = "/api/cv-snapshots/{snapshot_id}",
    tag = "cvs",
    params(("snapshot_id" = Uuid, Path, description = "The snapshot to read")),
    responses(
        (status = 200, description = "The frozen CV", body = inline(SuccessResponse<CvSnapshotResponse>)),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
        (status = 404, description = "No such snapshot, or it is not yours", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[get("/api/cv-snapshots/{snapshot_id}")]
pub async fn get_cv_snapshot_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> impl Responder {
    match data
        .cv_snapshot
        .get
        .execute(UserId::from(user.user_id), path.into_inner())
        .await
    {
        Ok(snapshot) => ApiResponse::success(to_response(snapshot)),
        Err(e) => map_error(e),
    }
}
