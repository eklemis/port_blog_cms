use actix_web::{get, web, Responder};
use uuid::Uuid;

use crate::{
    auth::adapter::incoming::web::extractors::auth::VerifiedUser, shared::api::ApiResponse,
    AppState,
};

#[get("/api/projects/{project_id}")]
pub async fn get_project_by_id_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> impl Responder {
    let project_id = path.into_inner();
    ApiResponse::success("data".to_string())
}
