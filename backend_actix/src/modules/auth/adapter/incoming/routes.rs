use crate::modules::auth::application::use_cases::create_user::{
    CreateUserError, ICreateUserUseCase,
};
use crate::AppState;
use actix_web::{post, web, HttpResponse, Responder};
use serde::Deserialize;

/// **📥 Request Structure for Creating a User**
#[derive(Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

/// **🚀 Create User API Endpoint**
#[post("/api/auth/register")]
pub async fn create_user_handler(
    req: web::Json<CreateUserRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let use_case = &data.create_user_use_case;

    let result = use_case
        .execute(
            req.username.clone(),
            req.email.clone(),
            req.password.clone(),
        )
        .await;

    match result {
        Ok(user) => HttpResponse::Created().json(user),
        Err(CreateUserError::UsernameAlreadyExists) => {
            HttpResponse::Conflict().body("Username already exists")
        }
        Err(CreateUserError::EmailAlreadyExists) => {
            HttpResponse::Conflict().body("Email already exists")
        }
        Err(CreateUserError::HashingFailed(e)) => {
            HttpResponse::InternalServerError().body(format!("Password hashing failed: {}", e))
        }
        Err(CreateUserError::RepositoryError(e)) => {
            HttpResponse::InternalServerError().body(format!("Database error: {}", e))
        }
    }
}
