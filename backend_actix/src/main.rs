pub mod modules;
pub use modules::auth;
pub use modules::cv;
pub use modules::email;

use crate::auth::adapter::outgoing::user_query_postgres::UserQueryPostgres;
use crate::auth::adapter::outgoing::user_repository_postgres::UserRepositoryPostgres;
use crate::auth::application::services::hash::{HashingAlgorithm, PasswordHashingService};
use crate::auth::application::services::jwt::{JwtConfig, JwtService};
use crate::auth::application::use_cases::{
    create_user::{CreateUserUseCase, ICreateUserUseCase},
    verify_user_email::{IVerifyUserEmailUseCase, VerifyUserEmailUseCase},
};
use crate::cv::adapter::outgoing::repository::CVRepoPostgres;
use crate::cv::application::use_cases::create_cv::{CreateCVUseCase, ICreateCVUseCase};
use crate::cv::application::use_cases::fetch_cv::{FetchCVUseCase, IFetchCVUseCase};
use crate::cv::application::use_cases::update_cv::{IUpdateCVUseCase, UpdateCVUseCase};

// Email Service
use crate::email::adapter::outgoing::smtp_sender::SmtpEmailSender;
use crate::email::application::services::EmailService;

use actix_web::{web, App, HttpServer};
use sea_orm::{Database, DatabaseConnection};
use std::env;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub fetch_cv_use_case: Arc<dyn IFetchCVUseCase + Send + Sync>,
    pub create_cv_use_case: Arc<dyn ICreateCVUseCase + Send + Sync>,
    pub update_cv_use_case: Arc<dyn IUpdateCVUseCase + Send + Sync>,
    pub create_user_use_case: Arc<dyn ICreateUserUseCase + Send + Sync>,
    pub verify_user_email_use_case: Arc<dyn IVerifyUserEmailUseCase + Send + Sync>,
}

#[actix_web::main]
#[cfg(not(tarpaulin_include))]
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

    let server_url = format!("{host}:{port}");
    println!("Server run on:{}", server_url);

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

    // Setup aut services
    let jwt_service = JwtService::new(JwtConfig::from_env());

    // Setup email service
    let smtp_sender = SmtpEmailSender::new(&smtp_server, &smtp_user, &smtp_pass, &from_email);
    let email_service = EmailService::new(Arc::new(smtp_sender));

    // Create User Use Case
    let user_repo = UserRepositoryPostgres::new(Arc::clone(&db_arc));
    let user_query = UserQueryPostgres::new(Arc::clone(&db_arc));
    let password_hasher = PasswordHashingService::new(HashingAlgorithm::Argon2);
    let create_user_use_case = CreateUserUseCase::new(
        user_query,
        user_repo.clone(),
        password_hasher,
        jwt_service.clone(),
        email_service,
        String::from(&server_url),
    );
    let verify_user_email_use_case = VerifyUserEmailUseCase::new(user_repo, jwt_service);

    // 3) Build app state - wrap each use case in Arc::new()
    let state = AppState {
        fetch_cv_use_case: Arc::new(fetch_cv_use_case),
        create_cv_use_case: Arc::new(create_cv_use_case),
        update_cv_use_case: Arc::new(update_cv_use_case),
        create_user_use_case: Arc::new(create_user_use_case),
        verify_user_email_use_case: Arc::new(verify_user_email_use_case),
    };

    // 4) Start the server
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .configure(init_routes)
    })
    .bind(server_url)?
    .run()
    .await
}

#[cfg(not(tarpaulin_include))]
fn init_routes(cfg: &mut web::ServiceConfig) {
    // CV
    cfg.service(crate::cv::adapter::incoming::routes::get_cv_handler);
    cfg.service(crate::cv::adapter::incoming::routes::create_cv_handler);
    cfg.service(crate::cv::adapter::incoming::routes::update_cv_handler);
    // Auth
    cfg.service(crate::auth::adapter::incoming::routes::create_user_handler);
    cfg.service(crate::auth::adapter::incoming::routes::verify_user_email_handler);
}

#[cfg(not(tarpaulin_include))]
fn main() {
    if let Err(e) = start() {
        eprintln!("Error starting app: {e}");
    }
}
