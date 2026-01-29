use actix_web::{get, web, Responder};
use serde::{Deserialize, Serialize};

use crate::{
    auth::adapter::incoming::web::extractors::auth::VerifiedUser, shared::api::ApiResponse,
    AppState,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectResponse {
    title: String,
    description: String,
}
#[get("/api/projects")]
pub async fn get_projects_handler(user: VerifiedUser, data: web::Data<AppState>) -> impl Responder {
    ApiResponse::success(vec![ProjectResponse {
        title: "test".to_string(),
        description: "test description".to_string(),
    }])
}
