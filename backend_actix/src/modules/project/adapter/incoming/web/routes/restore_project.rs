//! `POST /api/projects/{project_id}/restore`.

use actix_web::{post, web, Responder};
use tracing::error;
use uuid::Uuid;

use crate::api::schemas::ErrorResponse;
use crate::auth::adapter::incoming::web::extractors::auth::VerifiedUser;
use crate::auth::application::domain::entities::UserId;
use crate::project::application::ports::incoming::use_cases::RestoreProjectError;
use crate::shared::api::{ApiResponse, ErrorCode};
use crate::AppState;

/// Restore an archived project
///
/// `DELETE /api/projects/{id}` has always been a soft delete, and the archiver
/// has always had a `restore`, but nothing exposed it — so the console had to
/// present project deletion as permanent, contradicting the archive pattern
/// blog and CVs already follow.
///
/// Idempotent: restoring a project that was never archived succeeds.
#[utoipa::path(
    post,
    path = "/api/projects/{project_id}/restore",
    tag = "projects",
    params(("project_id" = Uuid, Path, description = "Project identifier")),
    responses(
        (status = 204, description = "Restored"),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 404, description = "Unknown, or owned by another user", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[post("/api/projects/{project_id}/restore")]
pub async fn restore_project_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> impl Responder {
    let owner = UserId::from(user.user_id);

    match data.project.restore.execute(owner, path.into_inner()).await {
        Ok(()) => ApiResponse::<()>::no_content(),
        Err(RestoreProjectError::ProjectNotFound) => {
            ApiResponse::not_found(ErrorCode::ProjectNotFound, "Project not found")
        }
        Err(RestoreProjectError::RepositoryError(e)) => {
            error!("Failed to restore project: {}", e);
            ApiResponse::internal_error()
        }
    }
}
