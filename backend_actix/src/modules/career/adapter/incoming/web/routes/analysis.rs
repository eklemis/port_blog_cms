//! `POST /api/applications/{id}/analysis` — how well a CV reads, and how well
//! it matches.

use actix_web::{post, web, HttpResponse, Responder};
use serde::Deserialize;
use tracing::error;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    api::schemas::{ErrorResponse, SuccessResponse},
    auth::{
        adapter::incoming::web::extractors::auth::VerifiedUser,
        application::domain::entities::UserId,
    },
    career::application::ports::incoming::use_cases::{
        AnalyseApplicationInput, AnalysisError, MatchAnalysis,
    },
    shared::api::{ApiResponse, ErrorCode},
    AppState,
};

/// Which CV to analyse.
#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct AnalyseApplicationRequest {
    /// A living CV — this is how tailoring works, before anything is sent.
    ///
    /// Omit it once the application has been sent and the analysis will run
    /// against the snapshot that actually went out. A draft with neither
    /// cannot be analysed.
    pub cv_id: Option<Uuid>,
}

fn map_error(e: AnalysisError) -> HttpResponse {
    match e {
        AnalysisError::ApplicationNotFound => {
            ApiResponse::not_found(ErrorCode::ApplicationNotFound, "Application not found")
        }
        AnalysisError::CvNotFound => ApiResponse::not_found(ErrorCode::CvNotFound, "CV not found"),
        AnalysisError::NoCvToAnalyse => {
            ApiResponse::bad_request(ErrorCode::ValidationError, &e.to_string())
        }
        AnalysisError::RepositoryError(e) => {
            error!("Repository error on an analysis: {}", e);
            ApiResponse::internal_error()
        }
    }
}

/// Analyse a CV against the job an application is for
///
/// Two halves, reported separately and **never averaged**. A single blended
/// number would hide which half a person should trust.
///
/// `readability` is computed here, deterministically — no model is consulted,
/// and the score is arithmetic over the checks shown, so a reader can
/// reconstruct it. Passing checks are included as well as failing ones: a list
/// of only problems leaves someone unable to tell "nothing wrong" from
/// "nothing looked at".
///
/// **`relevance` is `null` until the AI proxy exists.** `null` means *not
/// computed*, never *scored zero* — render one bar rather than two with one at
/// the floor.
///
/// One check the frontend asked for is deliberately absent: a CV here is
/// structured data, so whether it renders in one column or two is decided by
/// the template that draws it. A `single_column` result from this endpoint
/// would be the backend guessing about the frontend's rendering.
#[utoipa::path(
    post,
    path = "/api/applications/{application_id}/analysis",
    tag = "career",
    params(("application_id" = Uuid, Path, description = "The application to analyse")),
    request_body = AnalyseApplicationRequest,
    responses(
        (
            status = 200,
            description = "The analysis",
            body = inline(SuccessResponse<MatchAnalysis>),
            example = json!({
                "success": true,
                "data": {
                    "readability": {
                        "score": 88,
                        "checks": [
                            { "id": "has_experience", "ok": true, "detail": null },
                            {
                                "id": "dates_parse",
                                "ok": false,
                                "detail": "Start date not recognised on: Engineer at Acme. Use a year, or YYYY-MM."
                            }
                        ]
                    },
                    "relevance": null
                }
            })
        ),
        (status = 400, description = "A draft with no CV named and no snapshot", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
        (status = 404, description = "No such application or CV, or it is not yours", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[post("/api/applications/{application_id}/analysis")]
pub async fn analyse_application_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    body: Option<web::Json<AnalyseApplicationRequest>>,
    data: web::Data<AppState>,
) -> impl Responder {
    // The body is optional: analysing a sent application needs nothing in it,
    // and requiring `{}` for that would be a pointless refusal.
    let cv_id = body.and_then(|b| b.into_inner().cv_id);

    match data
        .career
        .analyse
        .execute(
            UserId::from(user.user_id),
            path.into_inner(),
            AnalyseApplicationInput { cv_id },
        )
        .await
    {
        Ok(analysis) => ApiResponse::success(analysis),
        Err(e) => map_error(e),
    }
}
