use actix_web::{post, web, Responder};
use serde::{Deserialize, Serialize};

use crate::{
    auth::adapter::incoming::web::extractors::auth::VerifiedUser, shared::api::ApiResponse,
    AppState,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CreateProjectRequest {
    title: String,
    description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CreateProjectResponse {
    title: String,
    description: String,
}

#[post("/api/cvs")]
pub async fn create_project_handler(
    user: VerifiedUser,
    req: web::Json<CreateProjectRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let req = req.into_inner();

    ApiResponse::created(CreateProjectResponse {
        title: "test".to_string(),
        description: "test description".to_string(),
    })
}
