pub mod modules;
pub use modules::auth;
pub use modules::cv;
pub use modules::email;
pub mod health;

// Test helpers module - only compiled with feature flag
#[cfg(feature = "test-helpers")]
mod test_helpers;

// ... (all your existing imports remain the same)
use crate::auth::adapter::outgoing::jwt::{JwtConfig, JwtTokenService};
use crate::auth::adapter::outgoing::token_repository_redis::RedisTokenRepository;
use crate::auth::adapter::outgoing::user_query_postgres::UserQueryPostgres;
use crate::auth::adapter::outgoing::user_repository_postgres::UserRepositoryPostgres;
use crate::auth::application::orchestrator::user_registration::UserRegistrationOrchestrator;
use crate::auth::application::use_cases::{
    create_user::{CreateUserUseCase, ICreateUserUseCase},
    login_user::{ILoginUserUseCase, LoginUserUseCase},
    logout_user::{ILogoutUseCase, LogoutUseCase},
    soft_delete_user::{ISoftDeleteUserUseCase, SoftDeleteUserUseCase},
    verify_user_email::{IVerifyUserEmailUseCase, VerifyUserEmailUseCase},
};

use crate::cv::adapter::outgoing::cv_repo_postgres::CVRepoPostgres;
use crate::cv::application::use_cases::create_cv::{CreateCVUseCase, ICreateCVUseCase};
use crate::cv::application::use_cases::fetch_cv_by_id::{FetchCVByIdUseCase, IFetchCVByIdUseCase};
use crate::cv::application::use_cases::fetch_user_cvs::{FetchCVUseCase, IFetchCVUseCase};
use crate::cv::application::use_cases::patch_cv::{IPatchCVUseCase, PatchCVUseCase};
use crate::cv::application::use_cases::update_cv::{IUpdateCVUseCase, UpdateCVUseCase};

use crate::email::adapter::outgoing::smtp_sender::SmtpEmailSender;
use crate::email::application::services::UserEmailService;
use crate::modules::auth::application::use_cases::refresh_token::IRefreshTokenUseCase;
use crate::modules::email::application::ports::outgoing::user_email_notifier::UserEmailNotifier;

use actix_web::{web, App, HttpServer};
use deadpool_redis::{Config, Runtime};

use sea_orm::{ConnectOptions, Database};
use std::env;
use std::sync::Arc;
use std::time::Duration;

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
    pub register_user_orchestrator: Arc<UserRegistrationOrchestrator>,
    pub verify_user_email_use_case: Arc<dyn IVerifyUserEmailUseCase + Send + Sync>,
    pub login_user_use_case: Arc<dyn ILoginUserUseCase + Send + Sync>,
    pub refresh_token_use_case: Arc<dyn IRefreshTokenUseCase + Send + Sync>,
    pub logout_user_use_case: Arc<dyn ILogoutUseCase + Send + Sync>,
    pub soft_delete_user_use_case: Arc<dyn ISoftDeleteUserUseCase + Send + Sync>,
}

#[actix_web::main]
#[cfg(not(tarpaulin_include))]
async fn start() -> std::io::Result<()> {
    use crate::auth::{
        adapter::outgoing::security::argon2_hasher::Argon2Hasher,
        application::{
            orchestrator::user_registration::UserRegistrationOrchestrator,
            ports::outgoing::token_provider::TokenProvider,
            use_cases::refresh_token::RefreshTokenUseCase,
        },
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,actix_web=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting application...");

    // 🚨 SAFETY GUARD: Prevent test-helpers in production
    #[cfg(feature = "test-helpers")]
    {
        let env = env::var("RUST_ENV").unwrap_or_else(|_| "development".to_string());
        if env == "production" {
            panic!("🚨 FATAL: test-helpers feature enabled in production environment!");
        }
        tracing::warn!(
            "⚠️  Test helper routes are ENABLED for environment: {}",
            env
        );
    }
    // Environtment variable loading
    let env = std::env::var("RUST_ENV").unwrap_or_else(|_| "development".to_string());

    // Try .env.{environment} first, then fall back to .env
    let env_file = format!(".env.{}", env);
    if dotenvy::from_filename(&env_file).is_err() {
        dotenvy::dotenv().ok();
    }

    // Load Env. variables
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL is not set in .env file");
    let host = env::var("HOST").expect("HOST is not set in .env file");
    let port = env::var("PORT").expect("PORT is not set in .env file");
    let redis_url = env::var("REDIS_URL").expect("REDIS_URL is not set in .env file");

    // SMTP SETUPS
    let from_email = std::env::var("EMAIL_FROM").expect("EMAIL_FROM not set");
    let smtp_sender = if std::env::var("RUST_ENV").as_deref() == Ok("test") {
        // Local Mailpit
        let host = std::env::var("SMTP_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port: u16 = std::env::var("SMTP_PORT")
            .unwrap_or_else(|_| "1025".to_string())
            .parse()
            .expect("Invalid SMTP_PORT");

        SmtpEmailSender::new_local(&host, port, &from_email)
    } else {
        // Production SMTP
        let smtp_server = std::env::var("SMTP_SERVER").expect("SMTP_SERVER not set");
        let smtp_user = std::env::var("SMTP_USERNAME").expect("SMTP_USERNAME not set");
        let smtp_pass = std::env::var("SMTP_PASSWORD").expect("SMTP_PASSWORD not set");

        SmtpEmailSender::new(&smtp_server, &smtp_user, &smtp_pass, &from_email)
    };

    let server_url = format!("{host}:{port}");
    println!("Server run on: {}", server_url);

    // Database connection
    let mut opt = ConnectOptions::new(db_url);
    opt.max_connections(50)
        .min_connections(10)
        .connect_timeout(Duration::from_secs(5))
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(300))
        .max_lifetime(Duration::from_secs(1800))
        .sqlx_logging(false);

    let conn = Database::connect(opt)
        .await
        .expect("Failed to connect to database");

    let db_arc = Arc::new(conn);

    // Redis connection
    let redis_pool = Config::from_url(&redis_url)
        .create_pool(Some(Runtime::Tokio1))
        .expect("Failed to create Redis pool");

    let redis_arc = Arc::new(redis_pool);

    // Create repositories and use cases (unchanged)
    let cv_repo = CVRepoPostgres::new(Arc::clone(&db_arc));
    let fetch_cv_use_case = FetchCVUseCase::new(cv_repo.clone());
    let fetch_cv_by_id_use_case = FetchCVByIdUseCase::new(cv_repo.clone());
    let create_cv_use_case = CreateCVUseCase::new(cv_repo.clone());
    let update_cv_use_case = UpdateCVUseCase::new(cv_repo.clone());
    let patch_cv_use_case = PatchCVUseCase::new(cv_repo.clone());

    let jwt_service = JwtTokenService::new(JwtConfig::from_env());

    let user_email_service =
        UserEmailService::new(jwt_service.clone(), smtp_sender, String::from(&server_url));

    let user_repo = UserRepositoryPostgres::new(Arc::clone(&db_arc));
    let user_query = UserQueryPostgres::new(Arc::clone(&db_arc));
    let redis_token_repo = RedisTokenRepository::new(Arc::clone(&redis_arc));
    let argon2_password_hasher = if std::env::var("RUST_ENV").as_deref() == Ok("production") {
        Argon2Hasher::budget_vps()
    } else {
        Argon2Hasher::fast_env()
    };

    // User Registration componenets
    let create_user_use_case = CreateUserUseCase::new(
        user_query.clone(),
        user_repo.clone(),
        Arc::new(argon2_password_hasher.clone()),
    );
    let create_user_uc_arc: Arc<dyn ICreateUserUseCase + Send + Sync> =
        Arc::new(create_user_use_case);
    let email_notifier_arc: Arc<dyn UserEmailNotifier + Send + Sync> = Arc::new(user_email_service);

    let register_user_orchestrator =
        UserRegistrationOrchestrator::new(create_user_uc_arc, email_notifier_arc);

    let verify_user_email_use_case =
        VerifyUserEmailUseCase::new(user_repo.clone(), Arc::new(jwt_service.clone()));
    let login_user_use_case = LoginUserUseCase::new(
        user_query,
        Arc::new(argon2_password_hasher),
        Arc::new(jwt_service.clone()),
    );
    let refresh_token_use_case = RefreshTokenUseCase::new(Arc::new(jwt_service.clone()));
    let logout_user_use_case =
        LogoutUseCase::new(redis_token_repo.clone(), Arc::new(jwt_service.clone()));
    let soft_delete_user_use_case = SoftDeleteUserUseCase::new(user_repo, redis_token_repo);

    let state = AppState {
        fetch_cv_use_case: Arc::new(fetch_cv_use_case),
        fetch_cv_by_id_use_case: Arc::new(fetch_cv_by_id_use_case),
        create_cv_use_case: Arc::new(create_cv_use_case),
        update_cv_use_case: Arc::new(update_cv_use_case),
        patch_cv_use_case: Arc::new(patch_cv_use_case),
        register_user_orchestrator: Arc::new(register_user_orchestrator),
        verify_user_email_use_case: Arc::new(verify_user_email_use_case),
        login_user_use_case: Arc::new(login_user_use_case),
        refresh_token_use_case: Arc::new(refresh_token_use_case),
        logout_user_use_case: Arc::new(logout_user_use_case),
        soft_delete_user_use_case: Arc::new(soft_delete_user_use_case),
    };

    let token_provider_arc: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service);
    // Clone db_arc for use in HttpServer closure
    let db_for_server = Arc::clone(&db_arc);

    HttpServer::new(move || {
        let mut app = App::new()
            .app_data(web::Data::new(state.clone()))
            .app_data(web::Data::new(Arc::clone(&token_provider_arc)))
            .app_data(web::Data::new(Arc::clone(&db_for_server)))
            .app_data(web::Data::new(Arc::clone(&redis_arc)))
            .configure(init_routes);

        // Conditionally add test routes
        #[cfg(feature = "test-helpers")]
        {
            app = app.configure(test_helpers::configure_routes);
        }

        app
    })
    .bind(server_url)?
    .run()
    .await
}

#[cfg(not(tarpaulin_include))]
fn init_routes(cfg: &mut web::ServiceConfig) {
    // Health
    cfg.service(crate::health::health);
    cfg.service(crate::health::readiness);
    // CV
    cfg.service(crate::cv::adapter::incoming::web::routes::get_cvs_handler);
    cfg.service(crate::cv::adapter::incoming::web::routes::get_cv_by_id_handler);
    cfg.service(crate::cv::adapter::incoming::web::routes::create_cv_handler);
    cfg.service(crate::cv::adapter::incoming::web::routes::update_cv_handler);
    cfg.service(crate::cv::adapter::incoming::web::routes::patch_cv_handler);
    // Auth
    cfg.service(crate::auth::adapter::incoming::web::routes::register_user_handler);
    cfg.service(crate::auth::adapter::incoming::web::routes::verify_user_email_handler);
    cfg.service(crate::auth::adapter::incoming::web::routes::login_user_handler);
    cfg.service(crate::auth::adapter::incoming::web::routes::refresh_token_handler);
    cfg.service(crate::auth::adapter::incoming::web::routes::logout_user_handler);
    cfg.service(crate::auth::adapter::incoming::web::routes::soft_delete_user_handler);
}

#[cfg(not(tarpaulin_include))]
fn main() {
    if let Err(e) = start() {
        eprintln!("Error starting app: {e}");
    }
}
