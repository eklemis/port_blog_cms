pub mod modules;
pub use modules::cv;

use actix_web::{web, App, HttpServer};
use sea_orm::{Database, DatabaseConnection};
use std::env;

use crate::cv::adapter::outgoing::repository::CVRepoPostgres;
use crate::cv::application::use_cases::create_cv::CreateCVUseCase;
use crate::cv::application::use_cases::fetch_cv::FetchCVUseCase;
use crate::cv::application::use_cases::update_cv::UpdateCVUseCase;

#[derive(Debug, Clone)]
pub struct AppState {
    pub fetch_cv_use_case: FetchCVUseCase<CVRepoPostgres>,
    pub create_cv_use_case: CreateCVUseCase<CVRepoPostgres>,
    pub update_cv_use_case: UpdateCVUseCase<CVRepoPostgres>,
}

#[actix_web::main]
async fn start() -> std::io::Result<()> {
    // get env vars
    dotenvy::dotenv().ok();
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL is not set in .env file");
    let host = env::var("HOST").expect("HOST is not set in .env file");
    let port = env::var("PORT").expect("PORT is not set in .env file");

    // 1. establish connection to database
    let conn: DatabaseConnection = Database::connect(&db_url)
        .await
        .expect("Failed to connect to database");
    // Apply migrations
    // -> create post table if not exists
    // Migrator::up(&conn, None).await.unwrap();

    // 2) Create repository and use case
    let repo = CVRepoPostgres::new(conn);
    let fetch_cv_use_case = FetchCVUseCase::new(repo.clone());
    let create_cv_use_case = CreateCVUseCase::new(repo.clone());
    let update_cv_use_case = UpdateCVUseCase::new(repo);

    // 3) build app state
    let state = AppState {
        fetch_cv_use_case,
        create_cv_use_case,
        update_cv_use_case,
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
    cfg.service(crate::cv::adapter::incoming::routes::get_cv_handler);
    cfg.service(crate::cv::adapter::incoming::routes::create_cv_handler);
    cfg.service(crate::cv::adapter::incoming::routes::update_cv_handler);
}

fn main() {
    if let Err(e) = start() {
        eprintln!("Error starting app: {e}");
    }
}
