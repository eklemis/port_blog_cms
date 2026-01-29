use actix_web::{patch, web, Responder};
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
struct PatchedProjectResponse {
    title: String,
    description: String,
}

#[patch("/api/cvs")]
pub async fn patch_project_handler(
    user: VerifiedUser,
    req: web::Json<CreateProjectRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let req = req.into_inner();

    ApiResponse::success(PatchedProjectResponse {
        title: "test".to_string(),
        description: "test description".to_string(),
    })
}
