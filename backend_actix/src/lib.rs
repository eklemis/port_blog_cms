#![deny(missing_docs)]
//! Portfolio CMS backend — the API behind `port_blog_cms`.
//!
//! The crate is laid out as ports and adapters. Each module under [`modules`]
//! is a vertical slice that owns its routes, its business logic and its
//! tables, and splits internally into `adapter/{incoming,outgoing}` and
//! `application/{ports,…}`. The rule that holds it together is that the
//! `application` layer never depends on `adapter` — adapters implement the
//! traits (*ports*) the application layer declares, and [`start`] is the only
//! place that decides which implementation is used.
//!
//! See `docs/ARCHITECTURE.md` for the layering rule in full, a request traced
//! end to end, and where new code belongs.
//!
//! # Layout
//!
//! - [`modules`] — the seven feature slices: auth, blog, cv, email,
//!   multimedia, project, topic.
//! - [`shared`] — the `ApiResponse` envelope, the `ErrorCode` vocabulary, CORS
//!   and rate limiting. Cross-cutting concerns that are no module's business.
//! - [`api`] — the OpenAPI document, served at `/swagger-ui/`.
//! - [`health`] — the `/health` and `/ready` probes.
//!
//! # Entry point
//!
//! [`start`] builds every adapter, wires them into [`AppState`] and runs the
//! server. The binary in `main.rs` is a shim that installs the rustls crypto
//! provider and calls it.

/// The seven feature modules. Each is a vertical slice; see
/// `docs/ARCHITECTURE.md`.
pub mod modules;
pub use modules::auth;
pub use modules::blog;
pub use modules::cv;
pub use modules::email;
pub use modules::multimedia;
pub use modules::project;
pub use modules::topic;
/// The OpenAPI document and the wrapper types it describes.
pub mod api;
/// The `/health` and `/ready` probes.
pub mod health;
/// Cross-cutting concerns that belong to no module: the response envelope,
/// the error-code vocabulary, CORS and rate limiting.
pub mod shared;

// Test helpers module - only compiled with feature flag
#[cfg(feature = "test-helpers")]
mod test_helpers;

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
use crate::cv::application::use_cases::fetch_user_cvs::{FetchCVService, IFetchCVUseCase};
use crate::cv::application::use_cases::patch_cv::{IPatchCVUseCase, PatchCVUseCase};
use crate::cv::application::use_cases::update_cv::{IUpdateCVUseCase, UpdateCVUseCase};

use crate::email::adapter::outgoing::smtp_sender::SmtpEmailSender;
use crate::email::application::services::UserEmailService;
use crate::modules::auth::application::helpers::UserIdentityResolver;
use crate::modules::auth::application::services::GetPublicProfileService;
use crate::modules::auth::application::services::UpdateUserProfileService;
use crate::modules::auth::application::use_cases::fetch_profile::FetchUserProfileUseCase;
use crate::modules::auth::application::use_cases::get_public_profile::GetPublicProfileUseCase;
use crate::modules::auth::application::use_cases::refresh_token::IRefreshTokenUseCase;
use crate::modules::auth::application::use_cases::request_password_reset::IRequestPasswordResetUseCase;
use crate::modules::auth::application::use_cases::resend_verification_email::{
    IResendVerificationEmailUseCase, ResendVerificationEmailUseCase,
};
use crate::modules::auth::application::use_cases::reset_password::IResetPasswordUseCase;
use crate::modules::auth::application::use_cases::update_profile::UpdateUserProfileUseCase;
use crate::modules::cv::application::use_cases::get_public_single_cv::GetPublicSingleCvUseCase;
use crate::modules::cv::application::use_cases::hard_delete_cv::HardDeleteCvUseCase;
use crate::modules::cv::application::use_cases::restore_cv::RestoreDeletedCvUseCase;
use crate::modules::cv::application::use_cases::soft_delete_cv::SoftDeleteCvUseCase;
use crate::modules::email::application::ports::outgoing::password_reset_notifier::PasswordResetNotifier;
use crate::modules::email::application::ports::outgoing::user_email_notifier::UserEmailNotifier;
use crate::modules::multimedia::adapter::outgoing::db::AvatarLoaderPostgres;

use crate::modules::blog::application::blog_preview_use_cases::BlogPreviewUseCases;
use crate::modules::blog::application::blog_use_cases::BlogUseCases;
use crate::modules::cv::application::cv_snapshot_use_cases::CvSnapshotUseCases;
use crate::modules::multimedia::application::domain::policies::upload_policy::UploadPolicy;
use crate::modules::multimedia::application::media_use_cases::MultimediaUseCases;
use crate::modules::multimedia::application::ports::incoming::services::{
    GetMediaStatusesService, GetMediaUsageService, GetPublicVariantUrlService,
    HardDeleteMediaService, PatchMediaService, RestoreMediaService,
};
use crate::modules::project::application::project_use_cases::ProjectUseCases;
use crate::modules::topic::application::ports::incoming::use_cases::CreateTopicUseCase;
use crate::modules::topic::application::ports::incoming::use_cases::GetTopicUsageUseCase;
use crate::modules::topic::application::ports::incoming::use_cases::GetTopicsUseCase;
use crate::modules::topic::application::ports::incoming::use_cases::PatchTopicUseCase;
use crate::modules::topic::application::ports::incoming::use_cases::SoftDeleteTopicUseCase;
use crate::shared::api::{build_cors, custom_json_config};
use crate::shared::rate_limit::{RateLimit, RateLimitStore, RedisRateLimitStore};

use actix_web::{middleware::Logger, web, App, HttpServer};
use deadpool_redis::{Config, Runtime};

use sea_orm::{ConnectOptions, Database};
use sqlx::postgres::PgConnectOptions;
use std::env;
use std::sync::Arc;
use std::time::Duration;

use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(test)]
mod tests;

#[derive(Clone)]
/// Everything a route handler can reach, assembled once at startup.
///
/// Handlers receive it through `web::Data`. Each field is a trait object, so
/// a handler depends on the contract and never on the adapter behind it —
/// [`start`] is the only place that decides which implementation is used.
///
/// Two shapes coexist: the older modules contribute one flat field per use
/// case, the newer ones a single grouped struct. Prefer the grouped form; see
/// the convention section of `docs/ARCHITECTURE.md`.
pub struct AppState {
    /// The `fetch cv` use case.
    pub fetch_cv_use_case: Arc<dyn IFetchCVUseCase + Send + Sync>,
    /// The `fetch cv by id` use case.
    pub fetch_cv_by_id_use_case: Arc<dyn IFetchCVByIdUseCase + Send + Sync>,
    /// The `get public single cv` use case.
    pub get_public_single_cv_use_case: Arc<dyn GetPublicSingleCvUseCase + Send + Sync>,
    /// The `create cv` use case.
    pub create_cv_use_case: Arc<dyn ICreateCVUseCase + Send + Sync>,
    /// The `update cv` use case.
    pub update_cv_use_case: Arc<dyn IUpdateCVUseCase + Send + Sync>,
    /// The `patch cv` use case.
    pub patch_cv_use_case: Arc<dyn IPatchCVUseCase + Send + Sync>,
    /// Registration, which spans user creation and the verification email.
    /// A concrete type rather than a trait object: it is the only orchestrator.
    pub register_user_orchestrator: Arc<UserRegistrationOrchestrator>,
    /// The `verify user email` use case.
    pub verify_user_email_use_case: Arc<dyn IVerifyUserEmailUseCase + Send + Sync>,
    /// The `login user` use case.
    pub login_user_use_case: Arc<dyn ILoginUserUseCase + Send + Sync>,
    /// The `refresh token` use case.
    pub refresh_token_use_case: Arc<dyn IRefreshTokenUseCase + Send + Sync>,
    /// The `request password reset` use case.
    pub request_password_reset_use_case: Arc<dyn IRequestPasswordResetUseCase + Send + Sync>,
    /// The `resend verification email` use case.
    pub resend_verification_email_use_case: Arc<dyn IResendVerificationEmailUseCase + Send + Sync>,
    /// The `reset password` use case.
    pub reset_password_use_case: Arc<dyn IResetPasswordUseCase + Send + Sync>,
    /// The `logout user` use case.
    pub logout_user_use_case: Arc<dyn ILogoutUseCase + Send + Sync>,
    /// The `soft delete user` use case.
    pub soft_delete_user_use_case: Arc<dyn ISoftDeleteUserUseCase + Send + Sync>,
    /// The `fetch user profile` use case.
    pub fetch_user_profile_use_case: Arc<dyn FetchUserProfileUseCase + Send + Sync>,
    /// The `get public profile` use case.
    pub get_public_profile_use_case: Arc<dyn GetPublicProfileUseCase + Send + Sync>,
    /// The `update user profile` use case.
    pub update_user_profile_use_case: Arc<dyn UpdateUserProfileUseCase + Send + Sync>,
    /// The `hard delete cv` use case.
    pub hard_delete_cv_use_case: Arc<dyn HardDeleteCvUseCase + Send + Sync>,
    /// The `soft delete cv` use case.
    pub soft_delete_cv_use_case: Arc<dyn SoftDeleteCvUseCase + Send + Sync>,
    /// The `restore cv` use case.
    pub restore_cv_use_case: Arc<dyn RestoreDeletedCvUseCase + Send + Sync>,
    /// The `create topic` use case.
    pub create_topic_use_case: Arc<dyn CreateTopicUseCase + Send + Sync>,
    /// The `get topics` use case.
    pub get_topics_use_case: Arc<dyn GetTopicsUseCase + Send + Sync>,
    /// The `get topic usage` use case.
    pub get_topic_usage_use_case: Arc<dyn GetTopicUsageUseCase + Send + Sync>,
    /// The `patch topic` use case.
    pub patch_topic_use_case: Arc<dyn PatchTopicUseCase + Send + Sync>,
    /// The `soft delete topic` use case.
    pub soft_delete_topic_use_case: Arc<dyn SoftDeleteTopicUseCase + Send + Sync>,
    /// Blog's use cases, grouped.
    pub blog: BlogUseCases,
    /// The draft-preview use cases.
    pub blog_preview: BlogPreviewUseCases,
    /// The CV-snapshot use cases.
    pub cv_snapshot: CvSnapshotUseCases,
    /// Project's use cases, grouped.
    pub project: ProjectUseCases,
    /// Multimedia's use cases, grouped.
    pub multimedia: MultimediaUseCases,
    /// Turns a username into a `UserId`. Shared by the public routes, which
    /// address users by name rather than id.
    pub user_identity_resolver: UserIdentityResolver,
    /// Size, dimension and MIME limits applied to uploads.
    pub multimedia_upload_policy: UploadPolicy,
}

/// Builds every adapter, wires them into [`AppState`], and runs the server.
///
/// The composition root: the only place that knows both halves of every port.
/// Excluded from coverage, because covering it means booting the process
/// against a real database and Redis.
#[actix_web::main]
pub async fn start() -> std::io::Result<()> {
    use crate::{
        auth::{
            adapter::outgoing::security::argon2_hasher::Argon2Hasher,
            application::{
                orchestrator::user_registration::UserRegistrationOrchestrator,
                ports::outgoing::token_provider::TokenProvider,
                services::password::BasicPasswordPolicy, services::FetchUserProfileService,
                use_cases::refresh_token::RefreshTokenUseCase,
                use_cases::request_password_reset::RequestPasswordResetUseCase,
                use_cases::reset_password::ResetPasswordUseCase,
            },
        },
        blog::{
            adapter::outgoing::{
                BlogPostArchiverPostgres, BlogPostQueryPostgres, BlogPostRepositoryPostgres,
                BlogPostTopicRepositoryPostgres, DraftPreviewStorePostgres,
            },
            application::ports::incoming::use_cases::{
                ArchiveBlogPostUseCase, AttachBlogPostTopicUseCase, DetachBlogPostTopicUseCase,
                HardDeleteBlogPostUseCase, RestoreBlogPostUseCase,
            },
            application::service::{
                ArchiveBlogPostService, AttachBlogPostTopicService, BulkBlogPostsService,
                ClearBlogPostTopicsService, CreateBlogPostService, DetachBlogPostTopicService,
                GetBlogPostTopicsService, GetBlogPostsService, GetDraftPreviewService,
                GetPublicBlogPostService, GetPublicBlogPostsService, GetSingleBlogPostService,
                HardDeleteBlogPostService, PatchBlogPostService, ReadDraftPreviewService,
                ReadPreviewMediaService, RestoreBlogPostService, RevokeDraftPreviewService,
                ShareDraftService, SlugAvailableService,
            },
        },
        cv::{
            adapter::outgoing::{CVArchiverPostgres, CVQueryPostgres, CvSnapshotStorePostgres},
            application::services::{
                CreateCvSnapshotService, GetCvSnapshotService, GetPublicSingleCvService,
                HardDeleteCvService, RestoreCvService, SoftDeleteCvService,
            },
        },
        multimedia::{
            adapter::outgoing::{
                cloud_storage::GcsStorageQuery,
                db::{preview_media_resolver, MediaQueryPostgres, MediaRepositoryPostgres},
            },
            application::ports::incoming::services::{
                BulkMediaService, CreateUploadMediaUrlService, DeleteMediaService, GetMediaService,
                GetVariantReadUrlService, ListMediaService,
            },
            application::ports::incoming::use_cases::{
                DeleteMediaUseCase, HardDeleteMediaUseCase, RestoreMediaUseCase,
            },
        },
        project::{
            adapter::outgoing::{
                ProjectArchiverPostgres, ProjectQueryPostgres, ProjectRepositoryPostgres,
                ProjectTopicRepositoryPostgres,
            },
            application::ports::incoming::use_cases::{
                AddProjectTopicUseCase, HardDeleteProjectUseCase, RemoveProjectTopicUseCase,
                RestoreProjectUseCase, SoftDeleteProjectUseCase,
            },
            application::service::{
                AddProjectTopicService, BulkProjectsService, ClearProjectTopicsService,
                CreateProjectService, GetProjectTopicsService, GetProjectsService,
                GetPublicSingleProjectService, GetSingleProjectService, HardDeleteProjectService,
                PatchProjectService, ProjectSlugAvailableService, RemoveProjectTopicService,
                RestoreProjectService, SoftDeleteProjectService,
            },
        },
        topic::{
            adapter::outgoing::{TopicQueryPostgres, TopicRepositoryPostgres},
            application::services::{
                CreateTopicService, GetTopicUsageService, GetTopicsService, PatchTopicService,
                SoftDeleteTopicService,
            },
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

    // Postgres and Redis
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL is not set in .env file");
    let redis_url = env::var("REDIS_URL").expect("REDIS_URL is not set in .env file");

    // Application host and port
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let server_url = format!("{host}:{port}");
    println!("Server run on: {}", server_url);

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

    // Database connection
    let mut opt = ConnectOptions::new(db_url);
    // IMPORTANT for PgBouncer / pooled connections
    opt.map_sqlx_postgres_opts(|pg: PgConnectOptions| pg.statement_cache_capacity(0));
    opt.max_connections(5)
        .min_connections(0)
        .connect_timeout(Duration::from_secs(30))
        .acquire_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(300))
        .max_lifetime(Duration::from_secs(1800))
        .sqlx_logging(false);

    let conn = Database::connect(opt).await.unwrap_or_else(|e| {
        eprintln!("DB connect error: {e:?}");
        std::process::exit(1);
    });

    let db_arc = Arc::new(conn);

    // Redis connection
    let redis_pool = Config::from_url(&redis_url)
        .create_pool(Some(Runtime::Tokio1))
        .expect("Failed to create Redis pool");

    let redis_arc = Arc::new(redis_pool);

    // Create CV repositories and use cases (unchanged)
    let cv_repo = CVRepoPostgres::new(Arc::clone(&db_arc));
    let cv_query = CVQueryPostgres::new(Arc::clone(&db_arc));

    let cv_archiver = CVArchiverPostgres::new(Arc::clone(&db_arc));
    let fetch_cv_use_case = FetchCVService::new(cv_query.clone());
    let fetch_cv_by_id_use_case = FetchCVByIdUseCase::new(cv_repo.clone());
    let get_public_single_cv_uc = GetPublicSingleCvService::new(cv_query.clone());

    let create_cv_use_case = CreateCVUseCase::new(cv_repo.clone());
    let update_cv_use_case = UpdateCVUseCase::new(cv_repo.clone());
    let patch_cv_use_case = PatchCVUseCase::new(cv_repo.clone());
    let hard_delete_cv_use_case = HardDeleteCvService::new(cv_archiver.clone(), cv_repo.clone());
    let soft_delete_cv_use_case = SoftDeleteCvService::new(cv_archiver.clone(), cv_repo.clone());
    let restore_cv_use_case = RestoreCvService::new(cv_archiver, cv_repo.clone());

    // Auth related services and adapters
    let jwt_service = JwtTokenService::new(JwtConfig::from_env());

    let verification_handler_url = env::var("VERIFICATION_HANDLER_URL")
        .unwrap_or_else(|_| "0.0.0.0:5173/email/verification".to_string());
    let password_reset_handler_url = env::var("PASSWORD_RESET_HANDLER_URL")
        .unwrap_or_else(|_| "0.0.0.0:5173/password-reset".to_string());

    let user_email_service = UserEmailService::new(
        smtp_sender,
        String::from(&verification_handler_url),
        password_reset_handler_url,
    );

    let user_repo = UserRepositoryPostgres::new(Arc::clone(&db_arc));
    let get_public_profile_service = GetPublicProfileService::new(
        UserQueryPostgres::new(Arc::clone(&db_arc)),
        Arc::new(AvatarLoaderPostgres::new(Arc::clone(&db_arc))),
    );
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
        Arc::new(BasicPasswordPolicy),
    );
    let create_user_uc_arc: Arc<dyn ICreateUserUseCase + Send + Sync> =
        Arc::new(create_user_use_case);
    let user_email_service_arc = Arc::new(user_email_service);
    let email_notifier_arc: Arc<dyn UserEmailNotifier + Send + Sync> =
        Arc::clone(&user_email_service_arc) as Arc<dyn UserEmailNotifier + Send + Sync>;
    let password_reset_notifier_arc: Arc<dyn PasswordResetNotifier + Send + Sync> =
        user_email_service_arc as Arc<dyn PasswordResetNotifier + Send + Sync>;

    let register_user_orchestrator = UserRegistrationOrchestrator::new(
        create_user_uc_arc,
        Arc::new(jwt_service.clone()),
        Arc::clone(&email_notifier_arc),
    );

    let verify_user_email_use_case =
        VerifyUserEmailUseCase::new(user_repo.clone(), Arc::new(jwt_service.clone()));
    let login_user_use_case = LoginUserUseCase::new(
        user_query.clone(),
        Arc::new(argon2_password_hasher.clone()),
        Arc::new(jwt_service.clone()),
    );
    let refresh_token_use_case = RefreshTokenUseCase::new(Arc::new(jwt_service.clone()));
    let request_password_reset_use_case = RequestPasswordResetUseCase::new(
        user_query.clone(),
        Arc::new(jwt_service.clone()),
        password_reset_notifier_arc,
    );
    let resend_verification_email_use_case = ResendVerificationEmailUseCase::new(
        user_query.clone(),
        Arc::new(jwt_service.clone()),
        Arc::clone(&email_notifier_arc),
    );
    let reset_password_use_case = ResetPasswordUseCase::new(
        user_repo.clone(),
        redis_token_repo.clone(),
        Arc::new(jwt_service.clone()),
        Arc::new(argon2_password_hasher.clone()),
        Arc::new(BasicPasswordPolicy),
    );
    let logout_user_use_case =
        LogoutUseCase::new(redis_token_repo.clone(), Arc::new(jwt_service.clone()));
    let soft_delete_user_use_case = SoftDeleteUserUseCase::new(user_repo.clone(), redis_token_repo);
    let fetch_user_profile_service = FetchUserProfileService::new(user_query.clone());
    let update_user_profile_service = UpdateUserProfileService::new(user_repo.clone());
    let identity_resolver = UserIdentityResolver::new(Arc::new(user_query.clone()));

    // Topics use cases, repo and query
    let topic_repo = TopicRepositoryPostgres::new(Arc::clone(&db_arc));
    let topic_query = TopicQueryPostgres::new(Arc::clone(&db_arc));
    let create_topic_uc = CreateTopicService::new(topic_repo.clone());
    let get_topics_uc = GetTopicsService::new(topic_query.clone());
    let get_topic_usage_uc = GetTopicUsageService::new(topic_query.clone());
    let patch_topic_uc = PatchTopicService::new(topic_repo.clone());
    let soft_delete_topic_uc = SoftDeleteTopicService::new(topic_query.clone(), topic_repo.clone());

    // Blog use cases, repos and query
    let blog_repo = BlogPostRepositoryPostgres::new(Arc::clone(&db_arc));
    let blog_query = BlogPostQueryPostgres::new(Arc::clone(&db_arc));
    let blog_archiver = BlogPostArchiverPostgres::new(Arc::clone(&db_arc));
    let blog_topic_repo = BlogPostTopicRepositoryPostgres::new(Arc::clone(&db_arc));

    // Built as named Arcs because the bulk use case fans out to these same
    // instances rather than constructing its own — one implementation of the
    // ownership rules, exercised by both the single and the batch routes.
    let blog_archive: Arc<dyn ArchiveBlogPostUseCase + Send + Sync> =
        Arc::new(ArchiveBlogPostService::new(blog_archiver.clone()));
    let blog_restore: Arc<dyn RestoreBlogPostUseCase + Send + Sync> =
        Arc::new(RestoreBlogPostService::new(blog_archiver.clone()));
    let blog_hard_delete: Arc<dyn HardDeleteBlogPostUseCase + Send + Sync> =
        Arc::new(HardDeleteBlogPostService::new(blog_archiver));
    let blog_attach_topic: Arc<dyn AttachBlogPostTopicUseCase + Send + Sync> =
        Arc::new(AttachBlogPostTopicService::new(blog_topic_repo.clone()));
    let blog_detach_topic: Arc<dyn DetachBlogPostTopicUseCase + Send + Sync> =
        Arc::new(DetachBlogPostTopicService::new(blog_topic_repo.clone()));

    let blog_use_cases = BlogUseCases {
        slug_available: Arc::new(SlugAvailableService::new(blog_query.clone())),
        create: Arc::new(CreateBlogPostService::new(blog_repo.clone())),
        list: Arc::new(GetBlogPostsService::new(blog_query.clone())),
        list_public: Arc::new(GetPublicBlogPostsService::new(blog_query.clone())),
        get_single: Arc::new(GetSingleBlogPostService::new(blog_query.clone())),
        get_public: Arc::new(GetPublicBlogPostService::new(blog_query.clone())),
        patch: Arc::new(PatchBlogPostService::new(blog_repo)),
        bulk: Arc::new(BulkBlogPostsService::new(
            Arc::clone(&blog_archive),
            Arc::clone(&blog_restore),
            Arc::clone(&blog_hard_delete),
            Arc::clone(&blog_attach_topic),
            Arc::clone(&blog_detach_topic),
        )),
        archive: blog_archive,
        restore: blog_restore,
        hard_delete: blog_hard_delete,
        attach_topic: blog_attach_topic,
        detach_topic: blog_detach_topic,
        clear_topics: Arc::new(ClearBlogPostTopicsService::new(blog_topic_repo)),
        get_topics: Arc::new(GetBlogPostTopicsService::new(blog_query)),
    };

    let preview_store = DraftPreviewStorePostgres::new(Arc::clone(&db_arc));
    let blog_preview_use_cases = BlogPreviewUseCases {
        share: Arc::new(ShareDraftService::new(preview_store.clone())),
        get: Arc::new(GetDraftPreviewService::new(preview_store.clone())),
        revoke: Arc::new(RevokeDraftPreviewService::new(preview_store.clone())),
        read: Arc::new(ReadDraftPreviewService::new(
            preview_store.clone(),
            BlogPostQueryPostgres::new(Arc::clone(&db_arc)),
            UserQueryPostgres::new(Arc::clone(&db_arc)),
        )),
        read_media: Arc::new(ReadPreviewMediaService::new(
            preview_store,
            preview_media_resolver(
                MediaQueryPostgres::new(Arc::clone(&db_arc)),
                GcsStorageQuery::new(),
            ),
        )),
    };

    let snapshot_store = CvSnapshotStorePostgres::new(Arc::clone(&db_arc));
    let cv_snapshot_use_cases = CvSnapshotUseCases {
        create: Arc::new(CreateCvSnapshotService::new(snapshot_store.clone())),
        get: Arc::new(GetCvSnapshotService::new(snapshot_store)),
    };

    // Project use cases, repos and query
    let project_repo = ProjectRepositoryPostgres::new(Arc::clone(&db_arc));
    let project_topic_repo = ProjectTopicRepositoryPostgres::new(Arc::clone(&db_arc));
    let project_archiver = ProjectArchiverPostgres::new(Arc::clone(&db_arc));

    let project_query = ProjectQueryPostgres::new(Arc::clone(&db_arc));
    let create_project_uc = CreateProjectService::new(project_repo.clone());
    let get_project_uc = GetProjectsService::new(project_query.clone());
    let get_single_project_uc = GetSingleProjectService::new(project_query.clone());
    let patch_project_uc = PatchProjectService::new(project_repo.clone());
    let get_public_single_project_uc = GetPublicSingleProjectService::new(project_query.clone());
    let add_topic_uc = AddProjectTopicService::new(project_topic_repo.clone());
    let remove_topic_uc = RemoveProjectTopicService::new(project_topic_repo.clone());
    let clear_topics_uc = ClearProjectTopicsService::new(project_topic_repo.clone());
    let get_project_topics_uc = GetProjectTopicsService::new(project_query.clone());
    let hard_delete_project_uc = HardDeleteProjectService::new(project_archiver.clone());
    let soft_delete_project_uc = SoftDeleteProjectService::new(project_archiver.clone());

    // Named Arcs so the bulk use case fans out to these same instances — one
    // implementation of the ownership rules, exercised by both routes.
    let project_restore: Arc<dyn RestoreProjectUseCase + Send + Sync> =
        Arc::new(RestoreProjectService::new(project_archiver.clone()));
    let project_hard_delete: Arc<dyn HardDeleteProjectUseCase + Send + Sync> =
        Arc::new(hard_delete_project_uc);
    let project_soft_delete: Arc<dyn SoftDeleteProjectUseCase + Send + Sync> =
        Arc::new(soft_delete_project_uc);
    let project_add_topic: Arc<dyn AddProjectTopicUseCase + Send + Sync> = Arc::new(add_topic_uc);
    let project_remove_topic: Arc<dyn RemoveProjectTopicUseCase + Send + Sync> =
        Arc::new(remove_topic_uc);

    let project_use_cases = ProjectUseCases {
        bulk: Arc::new(BulkProjectsService::new(
            Arc::clone(&project_soft_delete),
            Arc::clone(&project_restore),
            Arc::clone(&project_hard_delete),
            Arc::clone(&project_add_topic),
            Arc::clone(&project_remove_topic),
        )),
        restore: project_restore,
        slug_available: Arc::new(ProjectSlugAvailableService::new(project_query.clone())),
        create: Arc::new(create_project_uc),
        hard_delete: project_hard_delete,
        soft_delete: project_soft_delete,
        patch: Arc::new(patch_project_uc),
        get_list: Arc::new(get_project_uc),
        get_single: Arc::new(get_single_project_uc),
        get_public_single: Arc::new(get_public_single_project_uc),

        add_topic: project_add_topic,
        get_topics: Arc::new(get_project_topics_uc),
        remove_topic: project_remove_topic,
        clear_topics: Arc::new(clear_topics_uc),
    };

    // Mulitmedia Use Cases
    let storage_query = GcsStorageQuery::new();
    let media_repo = MediaRepositoryPostgres::new(Arc::clone(&db_arc));
    let media_repo_for_lifecycle = MediaRepositoryPostgres::new(Arc::clone(&db_arc));
    let delete_media_uc = DeleteMediaService::new(media_repo.clone());
    let create_upload_media_signed_url =
        CreateUploadMediaUrlService::new(storage_query.clone(), media_repo);
    let media_query = MediaQueryPostgres::new(Arc::clone(&db_arc));
    let create_variant_get_url =
        GetVariantReadUrlService::new(storage_query.clone(), media_query.clone());
    let public_variant_url = GetPublicVariantUrlService::new(media_query.clone(), storage_query);
    let patch_media = PatchMediaService::new(media_repo_for_lifecycle.clone());
    let restore_media = RestoreMediaService::new(media_repo_for_lifecycle.clone());
    let hard_delete_media = HardDeleteMediaService::new(media_repo_for_lifecycle);
    let get_media_usage = GetMediaUsageService::new(media_query.clone());
    let get_media_statuses = GetMediaStatusesService::new(media_query.clone());
    let get_media_uc = GetMediaService::new(media_query.clone());
    let list_media = ListMediaService::new(media_query);
    let media_archive: Arc<dyn DeleteMediaUseCase + Send + Sync> = Arc::new(delete_media_uc);
    let media_restore: Arc<dyn RestoreMediaUseCase + Send + Sync> = Arc::new(restore_media);
    let media_hard_delete: Arc<dyn HardDeleteMediaUseCase + Send + Sync> =
        Arc::new(hard_delete_media);

    let media_use_cases = MultimediaUseCases {
        bulk: Arc::new(BulkMediaService::new(
            Arc::clone(&media_archive),
            Arc::clone(&media_restore),
            Arc::clone(&media_hard_delete),
        )),
        create_signed_post_url: Arc::new(create_upload_media_signed_url),
        create_signed_get_url: Arc::new(create_variant_get_url),
        get_public_variant_url: Arc::new(public_variant_url),
        patch_media: Arc::new(patch_media),
        restore_media: media_restore,
        hard_delete_media: media_hard_delete,
        get_media_usage: Arc::new(get_media_usage),
        get_media_statuses: Arc::new(get_media_statuses),
        list_media: Arc::new(list_media),
        delete_media: media_archive,
        get_media: Arc::new(get_media_uc),
    };
    let image_upload_policy = UploadPolicy::from_env();

    let state = AppState {
        fetch_cv_use_case: Arc::new(fetch_cv_use_case),
        fetch_cv_by_id_use_case: Arc::new(fetch_cv_by_id_use_case),
        get_public_single_cv_use_case: Arc::new(get_public_single_cv_uc),
        create_cv_use_case: Arc::new(create_cv_use_case),
        update_cv_use_case: Arc::new(update_cv_use_case),
        patch_cv_use_case: Arc::new(patch_cv_use_case),
        register_user_orchestrator: Arc::new(register_user_orchestrator),
        verify_user_email_use_case: Arc::new(verify_user_email_use_case),
        login_user_use_case: Arc::new(login_user_use_case),
        refresh_token_use_case: Arc::new(refresh_token_use_case),
        request_password_reset_use_case: Arc::new(request_password_reset_use_case),
        resend_verification_email_use_case: Arc::new(resend_verification_email_use_case),
        reset_password_use_case: Arc::new(reset_password_use_case),
        logout_user_use_case: Arc::new(logout_user_use_case),
        soft_delete_user_use_case: Arc::new(soft_delete_user_use_case),
        fetch_user_profile_use_case: Arc::new(fetch_user_profile_service),
        get_public_profile_use_case: Arc::new(get_public_profile_service),
        update_user_profile_use_case: Arc::new(update_user_profile_service),
        hard_delete_cv_use_case: Arc::new(hard_delete_cv_use_case),
        soft_delete_cv_use_case: Arc::new(soft_delete_cv_use_case),
        restore_cv_use_case: Arc::new(restore_cv_use_case),
        create_topic_use_case: Arc::new(create_topic_uc),
        get_topics_use_case: Arc::new(get_topics_uc),
        get_topic_usage_use_case: Arc::new(get_topic_usage_uc),
        patch_topic_use_case: Arc::new(patch_topic_uc),
        soft_delete_topic_use_case: Arc::new(soft_delete_topic_uc),
        blog: blog_use_cases,
        blog_preview: blog_preview_use_cases,
        cv_snapshot: cv_snapshot_use_cases,
        project: project_use_cases,
        multimedia: media_use_cases,
        user_identity_resolver: identity_resolver,
        multimedia_upload_policy: image_upload_policy,
    };

    let rate_limit_store: Arc<dyn RateLimitStore> =
        Arc::new(RedisRateLimitStore::new(Arc::clone(&redis_arc)));

    let token_provider_arc: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service);
    // Clone db_arc for use in HttpServer closure
    let db_for_server = Arc::clone(&db_arc);

    HttpServer::new(move || {
        use utoipa::OpenApi;
        use utoipa_swagger_ui::SwaggerUi;

        use crate::api::openapi::ApiDoc;

        #[allow(unused_mut)]
        let mut app = App::new()
            // Logger is wrapped last so it sits outermost and also records
            // CORS preflight requests that never reach a handler.
            .wrap(Logger::default())
            .wrap(build_cors())
            // Inside CORS, so a rejected preflight never consumes quota, and a
            // 429 still carries the headers a browser needs to read it.
            .wrap(RateLimit::new(Arc::clone(&rate_limit_store)))
            .app_data(web::Data::new(state.clone()))
            .app_data(web::Data::new(Arc::clone(&token_provider_arc)))
            .app_data(web::Data::new(Arc::clone(&db_for_server)))
            .app_data(web::Data::new(Arc::clone(&redis_arc)))
            .app_data(custom_json_config())
            // ✅ Swagger UI service
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-docs/openapi.json", ApiDoc::openapi()),
            )
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

/// Registers every route on the Actix app.
///
/// **A new handler must be added here and to `src/api/openapi.rs`** — a test
/// fails if the OpenAPI document and the registered routes disagree.
pub fn init_routes(cfg: &mut web::ServiceConfig) {
    // Health
    cfg.service(crate::health::health);
    cfg.service(crate::health::readiness);
    // CV
    cfg.service(crate::cv::adapter::incoming::web::routes::get_cvs_handler);
    cfg.service(crate::cv::adapter::incoming::web::routes::get_cv_by_id_handler);
    cfg.service(crate::cv::adapter::incoming::web::routes::get_public_cv_by_id_handler);
    cfg.service(crate::cv::adapter::incoming::web::routes::create_cv_handler);
    cfg.service(crate::cv::adapter::incoming::web::routes::update_cv_handler);
    cfg.service(crate::cv::adapter::incoming::web::routes::patch_cv_handler);
    cfg.service(crate::cv::adapter::incoming::web::routes::hard_delete_cv_handler);
    cfg.service(crate::cv::adapter::incoming::web::routes::soft_delete_cv_handler);
    cfg.service(crate::cv::adapter::incoming::web::routes::restore_cv_handler);
    // Auth
    cfg.service(crate::auth::adapter::incoming::web::routes::register_user_handler);
    cfg.service(crate::auth::adapter::incoming::web::routes::verify_user_email_handler);
    cfg.service(crate::auth::adapter::incoming::web::routes::login_user_handler);
    cfg.service(crate::auth::adapter::incoming::web::routes::refresh_token_handler);
    cfg.service(crate::auth::adapter::incoming::web::routes::request_password_reset_handler);
    cfg.service(crate::auth::adapter::incoming::web::routes::resend_verification_handler);
    cfg.service(crate::auth::adapter::incoming::web::routes::reset_password_handler);
    cfg.service(crate::auth::adapter::incoming::web::routes::logout_user_handler);
    cfg.service(crate::auth::adapter::incoming::web::routes::soft_delete_user_handler);
    cfg.service(crate::auth::adapter::incoming::web::routes::get_public_profile_handler);
    cfg.service(crate::auth::adapter::incoming::web::routes::get_user_profile_handler);
    cfg.service(crate::auth::adapter::incoming::web::routes::update_user_profile_handler);
    // Topic
    cfg.service(crate::topic::adapter::incoming::web::routes::patch_topic_handler);
    cfg.service(crate::topic::adapter::incoming::web::routes::get_topic_usage_handler);
    cfg.service(crate::topic::adapter::incoming::web::routes::get_topics_handler);
    cfg.service(crate::topic::adapter::incoming::web::routes::create_topic_handler);
    cfg.service(crate::topic::adapter::incoming::web::routes::soft_delete_topic_handler);
    // Project
    cfg.service(crate::project::adapter::incoming::web::routes::project_slug_available_handler);
    cfg.service(crate::project::adapter::incoming::web::routes::get_projects_handler);
    cfg.service(crate::project::adapter::incoming::web::routes::get_public_projects_handler);
    cfg.service(crate::project::adapter::incoming::web::routes::create_project_handler);
    cfg.service(crate::project::adapter::incoming::web::routes::hard_delete_project_handler);
    cfg.service(crate::project::adapter::incoming::web::routes::get_project_by_id_handler);
    cfg.service(crate::project::adapter::incoming::web::routes::get_public_single_project_handler);
    cfg.service(crate::project::adapter::incoming::web::routes::patch_project_handler);
    cfg.service(crate::project::adapter::incoming::web::routes::restore_project_handler);
    cfg.service(crate::project::adapter::incoming::web::routes::soft_delete_project_handler);
    cfg.service(crate::project::adapter::incoming::web::routes::add_project_topic_handler);
    cfg.service(crate::project::adapter::incoming::web::routes::get_project_topics_handler);
    cfg.service(crate::project::adapter::incoming::web::routes::remove_project_topic_handler);
    cfg.service(crate::project::adapter::incoming::web::routes::clear_project_topics_handler);
    // Blog
    cfg.service(crate::blog::adapter::incoming::web::routes::create_blog_post_handler);
    cfg.service(crate::blog::adapter::incoming::web::routes::blog_slug_available_handler);
    cfg.service(crate::blog::adapter::incoming::web::routes::get_blog_posts_handler);
    cfg.service(crate::blog::adapter::incoming::web::routes::get_public_blog_posts_handler);
    cfg.service(crate::blog::adapter::incoming::web::routes::get_public_blog_post_handler);
    cfg.service(crate::blog::adapter::incoming::web::routes::get_single_blog_post_handler);
    cfg.service(crate::blog::adapter::incoming::web::routes::patch_blog_post_handler);
    cfg.service(crate::blog::adapter::incoming::web::routes::archive_blog_post_handler);
    cfg.service(crate::blog::adapter::incoming::web::routes::restore_blog_post_handler);
    cfg.service(crate::blog::adapter::incoming::web::routes::bulk_blog_posts_handler);
    cfg.service(crate::blog::adapter::incoming::web::routes::share_draft_handler);
    cfg.service(crate::blog::adapter::incoming::web::routes::get_draft_preview_handler);
    cfg.service(crate::blog::adapter::incoming::web::routes::revoke_draft_preview_handler);
    cfg.service(crate::blog::adapter::incoming::web::routes::read_draft_preview_handler);
    cfg.service(crate::blog::adapter::incoming::web::routes::read_preview_media_handler);
    cfg.service(crate::cv::adapter::incoming::web::routes::create_cv_snapshot_handler);
    cfg.service(crate::cv::adapter::incoming::web::routes::get_cv_snapshot_handler);
    cfg.service(crate::project::adapter::incoming::web::routes::bulk_projects_handler);
    cfg.service(crate::multimedia::adapter::incoming::web::routes::bulk_media_handler);
    cfg.service(crate::blog::adapter::incoming::web::routes::hard_delete_blog_post_handler);
    cfg.service(crate::blog::adapter::incoming::web::routes::attach_blog_post_topic_handler);
    cfg.service(crate::blog::adapter::incoming::web::routes::detach_blog_post_topic_handler);
    cfg.service(crate::blog::adapter::incoming::web::routes::clear_blog_post_topics_handler);
    cfg.service(crate::blog::adapter::incoming::web::routes::get_blog_post_topics_handler);
    // Multimedia
    cfg.service(crate::multimedia::adapter::incoming::web::routes::init_upload_handler);
    cfg.service(crate::multimedia::adapter::incoming::web::routes::get_variant_read_url_handler);
    cfg.service(crate::multimedia::adapter::incoming::web::routes::list_media_handler);
    cfg.service(crate::multimedia::adapter::incoming::web::routes::delete_media_handler);
    cfg.service(crate::multimedia::adapter::incoming::web::routes::get_media_handler);
    cfg.service(crate::multimedia::adapter::incoming::web::routes::patch_media_handler);
    cfg.service(crate::multimedia::adapter::incoming::web::routes::restore_media_handler);
    cfg.service(crate::multimedia::adapter::incoming::web::routes::hard_delete_media_handler);
    cfg.service(crate::multimedia::adapter::incoming::web::routes::get_media_statuses_handler);
    cfg.service(crate::multimedia::adapter::incoming::web::routes::get_media_usage_handler);
    cfg.service(crate::multimedia::adapter::incoming::web::routes::get_public_variant_handler);
}
