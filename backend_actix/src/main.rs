pub mod modules;
pub use modules::auth;
pub use modules::cv;
pub use modules::email;

// auth modul resources
use crate::auth::adapter::outgoing::user_query_postgres::UserQueryPostgres;
use crate::auth::adapter::outgoing::user_repository_postgres::UserRepositoryPostgres;
use crate::auth::application::services::hash::{HashingAlgorithm, PasswordHashingService};
use crate::auth::application::services::jwt::{JwtConfig, JwtService};
use crate::auth::application::use_cases::{
    create_user::{CreateUserUseCase, ICreateUserUseCase},
    login_user::{ILoginUserUseCase, LoginUserUseCase},
    verify_user_email::{IVerifyUserEmailUseCase, VerifyUserEmailUseCase},
};

// cv module resources
use crate::cv::adapter::outgoing::cv_repo_postgres::CVRepoPostgres;
use crate::cv::application::use_cases::create_cv::{CreateCVUseCase, ICreateCVUseCase};
use crate::cv::application::use_cases::fetch_cv_by_id::{FetchCVByIdUseCase, IFetchCVByIdUseCase};
use crate::cv::application::use_cases::fetch_user_cvs::{FetchCVUseCase, IFetchCVUseCase};
use crate::cv::application::use_cases::patch_cv::{IPatchCVUseCase, PatchCVUseCase};
use crate::cv::application::use_cases::update_cv::{IUpdateCVUseCase, UpdateCVUseCase};

// Email Service
use crate::email::adapter::outgoing::smtp_sender::SmtpEmailSender;
use crate::email::application::services::EmailService;
use crate::modules::auth::application::use_cases::refresh_token::IRefreshTokenUseCase;

use actix_web::{web, App, HttpServer};
use sea_orm::{ConnectOptions, Database};
use std::env;
use std::sync::Arc;
use std::time::Duration;

// Logging
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(test)]
mod tests;

#[derive(Clone)]
pub struct AppState {
    pub fetch_cv_use_case: Arc<dyn IFetchCVUseCase + Send + Sync>,
    pub fetch_cv_by_id_use_case: Arc<dyn IFetchCVByIdUseCase + Send + Sync>,
    pub create_cv_use_case: Arc<dyn ICreateCVUseCase + Send + Sync>,
    pub update_cv_use_case: Arc<dyn IUpdateCVUseCase + Send + Sync>,
    pub patch_cv_use_case: Arc<dyn IPatchCVUseCase + Send + Sync>,
    pub create_user_use_case: Arc<dyn ICreateUserUseCase + Send + Sync>,
    pub verify_user_email_use_case: Arc<dyn IVerifyUserEmailUseCase + Send + Sync>,
    pub login_user_use_case: Arc<dyn ILoginUserUseCase + Send + Sync>,
    refresh_token_use_case: Arc<dyn IRefreshTokenUseCase + Send + Sync>,
}

#[actix_web::main]
#[cfg(not(tarpaulin_include))]
async fn start() -> std::io::Result<()> {
    // Initialize tracing

    use crate::auth::application::use_cases::refresh_token::RefreshTokenUseCase;
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,actix_web=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting application...");

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
    let mut opt = ConnectOptions::new(db_url);
    opt
        // Core pool sizing
        .max_connections(50)
        .min_connections(10)
        // Timeouts (fail fast instead of piling up)
        .connect_timeout(Duration::from_secs(5))
        .acquire_timeout(Duration::from_secs(5))
        // Hygiene
        .idle_timeout(Duration::from_secs(300))
        .max_lifetime(Duration::from_secs(1800))
        // Noise reduction
        .sqlx_logging(false);

    let conn = Database::connect(opt)
        .await
        .expect("Failed to connect to database");

    let db_arc = Arc::new(conn);

    // 2) Create repository and use case
    let cv_repo = CVRepoPostgres::new(Arc::clone(&db_arc));
    let fetch_cv_use_case = FetchCVUseCase::new(cv_repo.clone());
    let fetch_cv_by_id_use_case = FetchCVByIdUseCase::new(cv_repo.clone());
    let create_cv_use_case = CreateCVUseCase::new(cv_repo.clone());
    let update_cv_use_case = UpdateCVUseCase::new(cv_repo.clone());
    let patch_cv_use_case = PatchCVUseCase::new(cv_repo.clone());

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
        user_query.clone(),
        user_repo.clone(),
        password_hasher.clone(),
        jwt_service.clone(),
        email_service,
        String::from(&server_url),
    );
    let verify_user_email_use_case = VerifyUserEmailUseCase::new(user_repo, jwt_service.clone());
    let login_user_use_case =
        LoginUserUseCase::new(user_query, password_hasher, jwt_service.clone());
    let refresh_token_use_case = RefreshTokenUseCase::new(jwt_service);

    // 3) Build app state - wrap each use case in Arc::new()
    let state = AppState {
        fetch_cv_use_case: Arc::new(fetch_cv_use_case),
        fetch_cv_by_id_use_case: Arc::new(fetch_cv_by_id_use_case),
        create_cv_use_case: Arc::new(create_cv_use_case),
        update_cv_use_case: Arc::new(update_cv_use_case),
        patch_cv_use_case: Arc::new(patch_cv_use_case),
        create_user_use_case: Arc::new(create_user_use_case),
        verify_user_email_use_case: Arc::new(verify_user_email_use_case),
        login_user_use_case: Arc::new(login_user_use_case),
        refresh_token_use_case: Arc::new(refresh_token_use_case),
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
    cfg.service(crate::cv::adapter::incoming::routes::get_cv_by_id_handler);
    cfg.service(crate::cv::adapter::incoming::routes::create_cv_handler);
    cfg.service(crate::cv::adapter::incoming::routes::update_cv_handler);
    cfg.service(crate::cv::adapter::incoming::routes::patch_cv_handler);
    // Auth
    cfg.service(crate::auth::adapter::incoming::routes::create_user_handler);
    cfg.service(crate::auth::adapter::incoming::routes::verify_user_email_handler);
    cfg.service(crate::auth::adapter::incoming::routes::login_user_handler);
    cfg.service(crate::auth::adapter::incoming::routes::refresh_token_handler);
}

#[cfg(not(tarpaulin_include))]
fn main() {
    if let Err(e) = start() {
        eprintln!("Error starting app: {e}");
    }
}
