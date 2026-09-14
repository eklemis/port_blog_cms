//! `/api/jobs` — captured postings.

use actix_web::{delete, get, patch, post, web, HttpResponse, Responder};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::error;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::career::adapter::incoming::web::routes::ListPageQuery;
use crate::career::application::ports::outgoing::{CareerPageResult, JobFilter};
use crate::{
    api::schemas::{ErrorResponse, SuccessResponse},
    auth::{
        adapter::incoming::web::extractors::auth::VerifiedUser,
        application::domain::entities::UserId,
    },
    career::application::ports::incoming::use_cases::JobError,
    career::application::ports::outgoing::{CreateJobData, PatchJobData},
    career::domain::entities::Job,
    shared::api::{ApiResponse, ErrorCode},
    AppState,
};

/// A posting as returned by the API.
#[derive(Debug, Serialize, ToSchema)]
pub struct JobResponse {
    /// Identifier.
    pub id: Uuid,
    /// Role title.
    pub title: String,
    /// Hiring company.
    pub company: String,
    /// Where the role is. Empty when unstated.
    pub location: String,
    /// Seniority as advertised. Empty when unstated.
    pub seniority: String,
    /// Extracted must-haves.
    pub required_skills: Vec<String>,
    /// Extracted nice-to-haves.
    pub nice_to_have: Vec<String>,
    /// Where it was found. Empty when pasted rather than linked.
    pub source_url: String,
    /// The posting verbatim. Kept because postings get taken down.
    pub source_text: String,
    /// When it was captured.
    pub created_at: DateTime<Utc>,
    /// Last edit.
    pub updated_at: DateTime<Utc>,
}

impl From<Job> for JobResponse {
    fn from(j: Job) -> Self {
        Self {
            id: j.id,
            title: j.title,
            company: j.company,
            location: j.location,
            seniority: j.seniority,
            required_skills: j.required_skills,
            nice_to_have: j.nice_to_have,
            source_url: j.source_url,
            source_text: j.source_text,
            created_at: j.created_at,
            updated_at: j.updated_at,
        }
    }
}

/// Body for capturing a posting.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateJobRequest {
    /// Role title. Required.
    pub title: String,
    /// Hiring company. Required.
    pub company: String,
    /// Where the role is.
    #[serde(default)]
    pub location: String,
    /// Seniority as advertised.
    #[serde(default)]
    pub seniority: String,
    /// Extracted must-haves.
    #[serde(default)]
    pub required_skills: Vec<String>,
    /// Extracted nice-to-haves.
    #[serde(default)]
    pub nice_to_have: Vec<String>,
    /// Where it was found.
    #[serde(default)]
    pub source_url: String,
    /// The posting verbatim. Stored exactly as sent, whitespace included.
    #[serde(default)]
    pub source_text: String,
}

/// Body for editing a posting. Every field is optional; omitted means unchanged.
#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct PatchJobRequest {
    /// New title.
    pub title: Option<String>,
    /// New company.
    pub company: Option<String>,
    /// New location.
    pub location: Option<String>,
    /// New seniority.
    pub seniority: Option<String>,
    /// Replacement must-haves. Sending `[]` empties the list.
    pub required_skills: Option<Vec<String>>,
    /// Replacement nice-to-haves.
    pub nice_to_have: Option<Vec<String>>,
    /// New source URL.
    pub source_url: Option<String>,
    /// Replacement source text. Rarely wanted — this is the record of what was
    /// published, not a field to tidy.
    pub source_text: Option<String>,
}

fn map_error(e: JobError) -> HttpResponse {
    match e {
        JobError::NotFound => ApiResponse::not_found(ErrorCode::JobNotFound, "Job not found"),
        JobError::Invalid(msg) => ApiResponse::bad_request(ErrorCode::ValidationError, &msg),
        JobError::RepositoryError(e) => {
            error!("Repository error on a job: {}", e);
            ApiResponse::internal_error()
        }
    }
}

/// Capture a job posting
///
/// Stores the posting as found. `source_text` is kept verbatim: postings get
/// taken down, and at interview time it is the only record of what was
/// actually asked for. Everything else can be re-derived from it.
#[utoipa::path(
    post,
    path = "/api/jobs",
    tag = "career",
    request_body = CreateJobRequest,
    responses(
        (status = 201, description = "Posting captured", body = inline(SuccessResponse<JobResponse>)),
        (status = 400, description = "No title or no company", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[post("/api/jobs")]
pub async fn create_job_handler(
    user: VerifiedUser,
    body: web::Json<CreateJobRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let b = body.into_inner();
    let input = CreateJobData {
        title: b.title,
        company: b.company,
        location: b.location,
        seniority: b.seniority,
        required_skills: b.required_skills,
        nice_to_have: b.nice_to_have,
        source_url: b.source_url,
        source_text: b.source_text,
    };

    match data
        .career
        .create_job
        .execute(UserId::from(user.user_id), input)
        .await
    {
        Ok(job) => ApiResponse::created(JobResponse::from(job)),
        Err(e) => map_error(e),
    }
}

/// Fetch a named set of postings rather than a page of them.
#[derive(Debug, Default, Deserialize, utoipa::IntoParams)]
pub struct JobIdsQuery {
    /// Comma-separated posting ids.
    ///
    /// For joining a page of applications back to the jobs they name, instead
    /// of asking for a large page and hoping the right ones are in it.
    ///
    /// Ids that do not exist, belong to someone else, or have been archived
    /// come back absent rather than as an error — a join must not fail because
    /// one of its rows was tidied away.
    #[param(example = "3f1b2c3d-...,7a2e9f10-...")]
    pub ids: Option<String>,
}

impl JobIdsQuery {
    /// The largest set this will look up in one request.
    ///
    /// Matches the page ceiling, since the caller is joining against at most
    /// one page of applications.
    const MAX_IDS: usize = 100;

    /// Parses the list, refusing rather than trimming.
    ///
    /// A join built from a silently shortened set is wrong, not slow: rows
    /// would render without a role or a company and nothing would say why. So
    /// too many ids is a 400, and so is one that is not a uuid.
    fn parsed(self) -> Result<JobFilter, String> {
        let Some(raw) = self.ids else {
            return Ok(JobFilter::default());
        };

        let parts: Vec<&str> = raw
            .split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .collect();

        if parts.len() > Self::MAX_IDS {
            return Err(format!(
                "At most {} ids per request, got {}",
                Self::MAX_IDS,
                parts.len()
            ));
        }

        let mut ids = Vec::with_capacity(parts.len());
        for p in parts {
            ids.push(uuid::Uuid::parse_str(p).map_err(|_| format!("`{p}` is not a valid id"))?);
        }

        Ok(JobFilter { ids: Some(ids) })
    }
}

/// List captured postings
#[utoipa::path(
    get,
    path = "/api/jobs",
    tag = "career",
    responses(
        (status = 200, description = "Postings, newest first", body = inline(SuccessResponse<CareerPageResult<JobResponse>>)),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
    ),
    params(ListPageQuery, JobIdsQuery),
    security(("BearerAuth" = []))
)]
#[get("/api/jobs")]
pub async fn get_jobs_handler(
    user: VerifiedUser,
    query: web::Query<ListPageQuery>,
    ids: web::Query<JobIdsQuery>,
    data: web::Data<AppState>,
) -> impl Responder {
    let filter = match ids.into_inner().parsed() {
        Ok(f) => f,
        Err(message) => return ApiResponse::bad_request(ErrorCode::InvalidRequest, &message),
    };

    match data
        .career
        .list_jobs
        .execute(
            UserId::from(user.user_id),
            filter,
            query.into_inner().into(),
        )
        .await
    {
        Ok(page) => ApiResponse::success(CareerPageResult {
            items: page
                .items
                .into_iter()
                .map(JobResponse::from)
                .collect::<Vec<_>>(),
            page: page.page,
            per_page: page.per_page,
            total: page.total,
        }),
        Err(e) => map_error(e),
    }
}

/// Read one posting
#[utoipa::path(
    get,
    path = "/api/jobs/{job_id}",
    tag = "career",
    params(("job_id" = Uuid, Path, description = "Identifier of the posting")),
    responses(
        (status = 200, description = "The posting", body = inline(SuccessResponse<JobResponse>)),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
        (status = 404, description = "No such posting, or it is not yours", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[get("/api/jobs/{job_id}")]
pub async fn get_job_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> impl Responder {
    match data
        .career
        .get_job
        .execute(UserId::from(user.user_id), path.into_inner())
        .await
    {
        Ok(job) => ApiResponse::success(JobResponse::from(job)),
        Err(e) => map_error(e),
    }
}

/// Edit a posting
#[utoipa::path(
    patch,
    path = "/api/jobs/{job_id}",
    tag = "career",
    params(("job_id" = Uuid, Path, description = "Identifier of the posting")),
    request_body = PatchJobRequest,
    responses(
        (status = 200, description = "The stored posting", body = inline(SuccessResponse<JobResponse>)),
        (status = 400, description = "Title or company blanked out", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
        (status = 404, description = "No such posting, or it is not yours", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[patch("/api/jobs/{job_id}")]
pub async fn patch_job_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    body: web::Json<PatchJobRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let b = body.into_inner();
    let patch = PatchJobData {
        title: b.title,
        company: b.company,
        location: b.location,
        seniority: b.seniority,
        required_skills: b.required_skills,
        nice_to_have: b.nice_to_have,
        source_url: b.source_url,
        source_text: b.source_text,
    };

    match data
        .career
        .patch_job
        .execute(UserId::from(user.user_id), path.into_inner(), patch)
        .await
    {
        Ok(job) => ApiResponse::success(JobResponse::from(job)),
        Err(e) => map_error(e),
    }
}

/// Archive a posting
///
/// Soft, like every other archive in this API. Applications keep pointing at
/// it — an application whose posting vanished would lose the only record of
/// what was asked for.
#[utoipa::path(
    delete,
    path = "/api/jobs/{job_id}",
    tag = "career",
    params(("job_id" = Uuid, Path, description = "Identifier of the posting")),
    responses(
        (status = 204, description = "Archived"),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
        (status = 404, description = "No such posting, or it is not yours", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[delete("/api/jobs/{job_id}")]
pub async fn archive_job_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> impl Responder {
    match data
        .career
        .archive_job
        .execute(UserId::from(user.user_id), path.into_inner())
        .await
    {
        Ok(()) => ApiResponse::no_content(),
        Err(e) => map_error(e),
    }
}

#[cfg(test)]
mod ids_query_tests {
    use super::*;

    fn q(ids: &str) -> JobIdsQuery {
        JobIdsQuery {
            ids: Some(ids.to_string()),
        }
    }

    #[test]
    fn no_ids_means_no_filter() {
        assert!(JobIdsQuery { ids: None }.parsed().unwrap().ids.is_none());
    }

    #[test]
    fn a_list_is_parsed_in_order() {
        let a = uuid::Uuid::new_v4();
        let b = uuid::Uuid::new_v4();

        let ids = q(&format!("{a},{b}")).parsed().unwrap().ids.unwrap();

        assert_eq!(ids, vec![a, b]);
    }

    /// Whitespace around a comma is what a hand-built query string looks like.
    #[test]
    fn spaces_around_the_commas_are_tolerated() {
        let a = uuid::Uuid::new_v4();
        let b = uuid::Uuid::new_v4();

        let ids = q(&format!(" {a} , {b} ")).parsed().unwrap().ids.unwrap();

        assert_eq!(ids, vec![a, b]);
    }

    /// The reason this refuses instead of trimming. A join built from a
    /// silently shortened set renders rows with no role and no company, and
    /// nothing anywhere says why.
    #[test]
    fn too_many_ids_is_refused_not_truncated() {
        let many = (0..=JobIdsQuery::MAX_IDS)
            .map(|_| uuid::Uuid::new_v4().to_string())
            .collect::<Vec<_>>()
            .join(",");

        let err = q(&many).parsed().unwrap_err();

        assert!(err.contains("At most 100"), "got: {err}");
    }

    #[test]
    fn exactly_the_maximum_is_allowed() {
        let max = (0..JobIdsQuery::MAX_IDS)
            .map(|_| uuid::Uuid::new_v4().to_string())
            .collect::<Vec<_>>()
            .join(",");

        assert_eq!(
            q(&max).parsed().unwrap().ids.unwrap().len(),
            JobIdsQuery::MAX_IDS
        );
    }

    #[test]
    fn something_that_is_not_an_id_is_refused() {
        let err = q("not-a-uuid").parsed().unwrap_err();

        assert!(err.contains("not a valid id"), "got: {err}");
    }

    /// An empty list asks for nothing and gets nothing. Reading it as "no
    /// filter" would hand back the whole table to a caller that asked for
    /// none of it.
    #[test]
    fn an_empty_list_asks_for_nothing() {
        let ids = q("").parsed().unwrap().ids.unwrap();

        assert!(ids.is_empty());
    }
}
