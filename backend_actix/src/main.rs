pub mod modules;
pub use modules::auth;
pub use modules::cv;
pub use modules::email;

use crate::auth::adapter::outgoing::user_query_postgres::UserQueryPostgres;
use crate::auth::adapter::outgoing::user_repository_postgres::UserRepositoryPostgres;
use crate::auth::application::services::hash::{HashingAlgorithm, PasswordHashingService};
use crate::auth::application::use_cases::create_user::CreateUserUseCase;
use crate::cv::adapter::outgoing::repository::CVRepoPostgres;
use crate::cv::application::use_cases::create_cv::CreateCVUseCase;
use crate::cv::application::use_cases::fetch_cv::FetchCVUseCase;
use crate::cv::application::use_cases::update_cv::UpdateCVUseCase;

// Email Service
use crate::email::adapter::outgoing::smtp_sender::SmtpEmailSender;
use crate::email::application::services::email_service::EmailService;

use actix_web::{web, App, HttpServer};
use sea_orm::{Database, DatabaseConnection};
use std::env;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct AppState {
    pub fetch_cv_use_case: FetchCVUseCase<CVRepoPostgres>,
    pub create_cv_use_case: CreateCVUseCase<CVRepoPostgres>,
    pub update_cv_use_case: UpdateCVUseCase<CVRepoPostgres>,
    pub create_user_use_case: CreateUserUseCase<UserQueryPostgres, UserRepositoryPostgres>,
    pub email_service: EmailService,
}

#[actix_web::main]
async fn start() -> std::io::Result<()> {
    // get env vars
    dotenvy::dotenv().ok();
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL is not set in .env file");
    let host = env::var("HOST").expect("HOST is not set in .env file");
    let port = env::var("PORT").expect("PORT is not set in .env file");
    // get env for email service
    let smtp_server = std::env::var("SMTP_SERVER").expect("SMTP_SERVER not set");
    let smtp_user = std::env::var("SMTP_USERNAME").expect("SMTP_USERNAME not set");
    let smtp_pass = std::env::var("SMTP_PASSWORD").expect("SMTP_PASSWORD not set");
    let from_email = std::env::var("EMAIL_FROM").expect("EMAIL_FROM not set");

    // 1. establish connection to database
    let conn: DatabaseConnection = Database::connect(&db_url)
        .await
        .expect("Failed to connect to database");
    let db_arc = Arc::new(conn);

    // 2) Create repository and use case
    let repo = CVRepoPostgres::new(Arc::clone(&db_arc));
    let fetch_cv_use_case = FetchCVUseCase::new(repo.clone());
    let create_cv_use_case = CreateCVUseCase::new(repo.clone());
    let update_cv_use_case = UpdateCVUseCase::new(repo.clone());

    // Create User Use Case
    let user_repo = UserRepositoryPostgres::new(Arc::clone(&db_arc));
    let user_query = UserQueryPostgres::new(Arc::clone(&db_arc));
    let password_hasher = PasswordHashingService::new(HashingAlgorithm::Argon2);
    let create_user_use_case = CreateUserUseCase::new(user_query, user_repo, password_hasher);

    // Setup email service
    let smtp_sender = SmtpEmailSender::new(&smtp_server, &smtp_user, &smtp_pass, &from_email);
    let email_service = EmailService::new(Arc::new(smtp_sender));

    // 3) build app state
    let state = AppState {
        fetch_cv_use_case,
        create_cv_use_case,
        update_cv_use_case,
        create_user_use_case,
        email_service,
    };

    // 4) Start the server
    let server_url = format!("{host}:{port}");
    println!("Server run on:{}", server_url);
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .configure(init_routes)
    })
    .bind(server_url)?
    .run()
    .await
}

fn init_routes(cfg: &mut web::ServiceConfig) {
    // CV
    cfg.service(crate::cv::adapter::incoming::routes::get_cv_handler);
    cfg.service(crate::cv::adapter::incoming::routes::create_cv_handler);
    cfg.service(crate::cv::adapter::incoming::routes::update_cv_handler);
    // Auth
    cfg.service(crate::auth::adapter::incoming::routes::create_user_handler);
}

fn main() {
    if let Err(e) = start() {
        eprintln!("Error starting app: {e}");
    }
}
