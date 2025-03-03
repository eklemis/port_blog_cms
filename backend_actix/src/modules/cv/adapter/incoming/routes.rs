use crate::cv::application::use_cases::create_cv::{CreateCVError, ICreateCVUseCase};
use crate::cv::application::use_cases::fetch_cv::{FetchCVError, IFetchCVUseCase};
use crate::cv::application::use_cases::update_cv::{IUpdateCVUseCase, UpdateCVError};
use crate::cv::domain::entities::{CVInfo, Education, Experience, HighlightedProject};
use crate::AppState;
use actix_web::{get, post, put, web, HttpResponse, Responder};
use uuid::Uuid;

#[get("/api/cv/{user_id}")]
pub async fn get_cv_handler(
    path: web::Path<Uuid>,
    data: web::Data<AppState>, // The state from .app_data(...)
) -> impl Responder {
    let user_id = path.into_inner();
    // 1) Call the existing use case from AppState
    let result = data.fetch_cv_use_case.execute(user_id).await;

    // 2) Map the result to an HTTP response
    match result {
        Ok(cv_info) => HttpResponse::Ok().json(cv_info),
        Err(FetchCVError::CVNotFound) => HttpResponse::NotFound().finish(),
        Err(FetchCVError::RepositoryError(err_msg)) => {
            HttpResponse::InternalServerError().body(err_msg)
        }
    }
}

#[derive(serde::Deserialize)]
pub struct CreateCVRequest {
    pub bio: String,
    pub photo_url: String,
    pub educations: Vec<EducationRequest>,
    pub experiences: Vec<ExperienceRequest>,
    pub highlighted_projects: Vec<HighlightedProjectRequest>,
}

#[derive(serde::Deserialize)]
pub struct EducationRequest {
    pub degree: String,
    pub institution: String,
    pub graduation_year: i32,
}

#[derive(serde::Deserialize)]
pub struct ExperienceRequest {
    pub company: String,
    pub position: String,
    pub start_date: String,
    pub end_date: Option<String>,
    pub description: String,
}

#[derive(serde::Deserialize)]
pub struct HighlightedProjectRequest {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub short_description: String,
}

#[post("/api/cv/{user_id}")]
pub async fn create_cv_handler(
    path: web::Path<Uuid>,
    req: web::Json<CreateCVRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let user_id = path.into_inner();

    // Map the request fields to domain objects
    let cv_data = CVInfo {
        bio: req.bio.clone(),
        photo_url: req.photo_url.clone(),
        educations: req
            .educations
            .iter()
            .map(|e| Education {
                degree: e.degree.clone(),
                institution: e.institution.clone(),
                graduation_year: e.graduation_year,
            })
            .collect(),
        experiences: req
            .experiences
            .iter()
            .map(|exp| Experience {
                company: exp.company.clone(),
                position: exp.position.clone(),
                start_date: exp.start_date.clone(),
                end_date: exp.end_date.clone(),
                description: exp.description.clone(),
            })
            .collect(),
        highlighted_projects: req
            .highlighted_projects
            .iter()
            .map(|hp| HighlightedProject {
                id: hp.id.clone(),
                title: hp.title.clone(),
                slug: hp.slug.clone(),
                short_description: hp.short_description.clone(),
            })
            .collect(),
    };

    // Call the use case
    match data.create_cv_use_case.execute(user_id, cv_data).await {
        Ok(created) => HttpResponse::Created().json(created),
        Err(CreateCVError::AlreadyExists) => HttpResponse::Conflict().body("CV already exists"),
        Err(CreateCVError::RepositoryError(e)) => HttpResponse::InternalServerError().body(e),
    }
}

#[derive(serde::Deserialize)]
pub struct UpdateCVRequest {
    pub bio: String,
    pub photo_url: String,
    pub educations: Vec<EducationRequest>,
    pub experiences: Vec<ExperienceRequest>,
    pub highlighted_projects: Vec<HighlightedProjectRequest>,
}

#[put("/api/cv/{user_id}")]
pub async fn update_cv_handler(
    path: web::Path<Uuid>,
    req: web::Json<UpdateCVRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let user_id = path.into_inner();

    let cv_data = CVInfo {
        bio: req.bio.clone(),
        photo_url: req.photo_url.clone(),
        educations: req
            .educations
            .iter()
            .map(|e| Education {
                degree: e.degree.clone(),
                institution: e.institution.clone(),
                graduation_year: e.graduation_year,
            })
            .collect(),
        experiences: req
            .experiences
            .iter()
            .map(|exp| Experience {
                company: exp.company.clone(),
                position: exp.position.clone(),
                start_date: exp.start_date.clone(),
                end_date: exp.end_date.clone(),
                description: exp.description.clone(),
            })
            .collect(),
        highlighted_projects: req
            .highlighted_projects
            .iter()
            .map(|hp| HighlightedProject {
                id: hp.id.clone(),
                title: hp.title.clone(),
                slug: hp.slug.clone(),
                short_description: hp.short_description.clone(),
            })
            .collect(),
    };

    match data.update_cv_use_case.execute(user_id, cv_data).await {
        Ok(updated) => HttpResponse::Ok().json(updated),
        Err(UpdateCVError::CVNotFound) => HttpResponse::NotFound().body("CV not found"),
        Err(UpdateCVError::RepositoryError(e)) => HttpResponse::InternalServerError().body(e),
    }
}
