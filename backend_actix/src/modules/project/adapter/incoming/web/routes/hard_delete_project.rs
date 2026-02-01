use actix_web::{delete, web, Responder};

use crate::{
    auth::adapter::incoming::web::extractors::auth::VerifiedUser, shared::api::ApiResponse,
    AppState,
};

#[delete("/api/projects")]
pub async fn hard_delete_project_handler(
    _user: VerifiedUser,
    _data: web::Data<AppState>,
) -> impl Responder {
    ApiResponse::no_content()
}
