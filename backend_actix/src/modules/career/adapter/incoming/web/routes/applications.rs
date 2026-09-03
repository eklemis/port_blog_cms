//! `/api/applications` — one row per application.

use actix_web::{delete, get, patch, post, web, HttpResponse, Responder};
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
    career::application::ports::incoming::use_cases::{ApplicationError, UpdateApplicationInput},
    career::application::ports::outgoing::CreateApplicationData,
    career::domain::entities::{Application, ApplicationStatus},
    shared::api::{ApiResponse, ErrorCode},
    AppState,
};

/// An application as returned by the API.
#[derive(Debug, Serialize, ToSchema)]
pub struct ApplicationResponse {
    /// Identifier.
    pub id: Uuid,
    /// The posting applied to.
    pub job_id: Uuid,
    /// The frozen CV that was sent. `null` only while this is a draft.
    pub cv_snapshot_id: Option<Uuid>,
    /// Where it has got to.
    pub status: ApplicationStatus,
    /// When it was sent. `null` while still a draft.
    pub applied_at: Option<DateTime<Utc>>,
    /// What you owe it next, in your own words. Empty when nothing is due.
    pub next_action: String,
    /// When that is due.
    pub next_action_at: Option<DateTime<Utc>>,
    /// When the row was created.
    pub created_at: DateTime<Utc>,
    /// Last edit.
    pub updated_at: DateTime<Utc>,
}

impl From<Application> for ApplicationResponse {
    fn from(a: Application) -> Self {
        Self {
            id: a.id,
            job_id: a.job_id,
            cv_snapshot_id: a.cv_snapshot_id,
            status: a.status,
            applied_at: a.applied_at,
            next_action: a.next_action,
            next_action_at: a.next_action_at,
            created_at: a.created_at,
            updated_at: a.updated_at,
        }
    }
}

/// Body for starting an application.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateApplicationRequest {
    /// The posting being applied to. Must be one of yours.
    pub job_id: Uuid,
    /// What you owe it next.
    #[serde(default)]
    pub next_action: String,
    /// When that is due.
    #[serde(default)]
    pub next_action_at: Option<DateTime<Utc>>,
}

/// Body for editing an application.
#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct PatchApplicationRequest {
    /// New status.
    pub status: Option<ApplicationStatus>,

    /// A CV to freeze and attach as part of this edit.
    ///
    /// This is how the snapshot gets taken: name the CV you are applying with
    /// and the backend stores an immutable copy. **Moving off `draft` requires
    /// one** — either sent here, or already attached by an earlier edit.
    pub cv_id: Option<Uuid>,

    /// New next action. Send `""` to clear it.
    pub next_action: Option<String>,

    /// New due date. Send `null` to clear it.
    #[serde(default, deserialize_with = "double_option")]
    pub next_action_at: Option<Option<DateTime<Utc>>>,
}

/// Distinguishes "key absent" from "key present and null".
fn double_option<'de, D, T>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    serde::Deserialize::deserialize(de).map(Some)
}

fn map_error(e: ApplicationError) -> HttpResponse {
    match e {
        ApplicationError::NotFound => {
            ApiResponse::not_found(ErrorCode::ApplicationNotFound, "Application not found")
        }
        ApplicationError::JobNotFound => {
            ApiResponse::not_found(ErrorCode::JobNotFound, "Job not found")
        }
        ApplicationError::CvNotFound => {
            ApiResponse::not_found(ErrorCode::CvNotFound, "CV not found")
        }
        ApplicationError::SnapshotRequired => {
            ApiResponse::bad_request(ErrorCode::SnapshotRequired, &e.to_string())
        }
        ApplicationError::RepositoryError(e) => {
            error!("Repository error on an application: {}", e);
            ApiResponse::internal_error()
        }
    }
}

/// Start an application
///
/// Always starts as a draft. Sending it is an edit — which is where the
/// snapshot rule applies, so there is exactly one path that can produce a sent
/// application rather than two to keep in step.
#[utoipa::path(
    post,
    path = "/api/applications",
    tag = "career",
    request_body = CreateApplicationRequest,
    responses(
        (status = 201, description = "Draft application started", body = inline(SuccessResponse<ApplicationResponse>)),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
        (status = 404, description = "No such posting, or it is not yours", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[post("/api/applications")]
pub async fn create_application_handler(
    user: VerifiedUser,
    body: web::Json<CreateApplicationRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let b = body.into_inner();

    match data
        .career
        .create_application
        .execute(
            UserId::from(user.user_id),
            CreateApplicationData {
                job_id: b.job_id,
                next_action: b.next_action,
                next_action_at: b.next_action_at,
            },
        )
        .await
    {
        Ok(app) => ApiResponse::created(ApplicationResponse::from(app)),
        Err(e) => map_error(e),
    }
}

/// List applications
#[utoipa::path(
    get,
    path = "/api/applications",
    tag = "career",
    responses(
        (status = 200, description = "Applications, newest first", body = inline(SuccessResponse<Vec<ApplicationResponse>>)),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[get("/api/applications")]
pub async fn get_applications_handler(
    user: VerifiedUser,
    data: web::Data<AppState>,
) -> impl Responder {
    match data
        .career
        .list_applications
        .execute(UserId::from(user.user_id))
        .await
    {
        Ok(apps) => ApiResponse::success(
            apps.into_iter()
                .map(ApplicationResponse::from)
                .collect::<Vec<_>>(),
        ),
        Err(e) => map_error(e),
    }
}

/// Read one application
#[utoipa::path(
    get,
    path = "/api/applications/{application_id}",
    tag = "career",
    params(("application_id" = Uuid, Path, description = "Identifier of the application")),
    responses(
        (status = 200, description = "The application", body = inline(SuccessResponse<ApplicationResponse>)),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
        (status = 404, description = "No such application, or it is not yours", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[get("/api/applications/{application_id}")]
pub async fn get_application_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> impl Responder {
    match data
        .career
        .get_application
        .execute(UserId::from(user.user_id), path.into_inner())
        .await
    {
        Ok(app) => ApiResponse::success(ApplicationResponse::from(app)),
        Err(e) => map_error(e),
    }
}

/// Edit an application
///
/// **Moving off `draft` requires a CV snapshot.** Send `cv_id` and one is
/// taken as part of this call; if the application already carries a snapshot
/// from an earlier edit, that is enough. Without either, this refuses with
/// `SNAPSHOT_REQUIRED` rather than storing a row that will misreport what was
/// sent once the CV is next edited.
///
/// `applied_at` is stamped automatically the first time the application leaves
/// draft. A reopened application that is sent again keeps its original date —
/// that is the date the employer saw.
#[utoipa::path(
    patch,
    path = "/api/applications/{application_id}",
    tag = "career",
    params(("application_id" = Uuid, Path, description = "Identifier of the application")),
    request_body = PatchApplicationRequest,
    responses(
        (status = 200, description = "The stored application", body = inline(SuccessResponse<ApplicationResponse>)),
        (
            status = 400,
            description = "Leaving draft with no CV to point at",
            body = ErrorResponse,
            example = json!({
                "success": false,
                "error": {
                    "code": "SNAPSHOT_REQUIRED",
                    "message": "Leaving draft requires a CV: send cv_id, or attach a snapshot first"
                }
            })
        ),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
        (status = 404, description = "No such application or CV, or it is not yours", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[patch("/api/applications/{application_id}")]
pub async fn patch_application_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    body: web::Json<PatchApplicationRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let b = body.into_inner();

    match data
        .career
        .patch_application
        .execute(
            UserId::from(user.user_id),
            path.into_inner(),
            UpdateApplicationInput {
                status: b.status,
                cv_id: b.cv_id,
                next_action: b.next_action,
                next_action_at: b.next_action_at,
            },
        )
        .await
    {
        Ok(app) => ApiResponse::success(ApplicationResponse::from(app)),
        Err(e) => map_error(e),
    }
}

/// Archive an application
#[utoipa::path(
    delete,
    path = "/api/applications/{application_id}",
    tag = "career",
    params(("application_id" = Uuid, Path, description = "Identifier of the application")),
    responses(
        (status = 204, description = "Archived"),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
        (status = 404, description = "No such application, or it is not yours", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[delete("/api/applications/{application_id}")]
pub async fn archive_application_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> impl Responder {
    match data
        .career
        .archive_application
        .execute(UserId::from(user.user_id), path.into_inner())
        .await
    {
        Ok(()) => ApiResponse::no_content(),
        Err(e) => map_error(e),
    }
}
