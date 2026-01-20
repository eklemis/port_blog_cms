use actix_web::{get, web, HttpResponse, Responder};
use redis::aio::ConnectionManager;
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Serialize)]
struct ReadinessResponse {
    status: &'static str,
    database: &'static str,
    redis: &'static str,
}

/// LIVENESS PROBE
/// - No I/O
/// - No DB
/// - No Redis
#[get("/health")]
pub async fn health() -> impl Responder {
    HttpResponse::Ok().json(HealthResponse { status: "ok" })
}

/// READINESS PROBE
/// - Checks critical dependencies
#[get("/ready")]
pub async fn readiness(
    db: web::Data<Arc<DatabaseConnection>>,
    redis: web::Data<Arc<Mutex<ConnectionManager>>>,
) -> impl Responder {
    let db_status = match db
        .execute(Statement::from_string(
            db.get_database_backend(),
            "SELECT 1",
        ))
        .await
    {
        Ok(_) => "ok",
        Err(_) => "unhealthy",
    };

    let redis_status = {
        let mut conn = redis.lock().await;
        match redis::cmd("PING").query_async::<String>(&mut *conn).await {
            Ok(_) => "ok",
            Err(_) => "unhealthy",
        }
    };

    let overall_status = if db_status == "ok" && redis_status == "ok" {
        "ok"
    } else {
        "unhealthy"
    };

    if overall_status == "ok" {
        HttpResponse::Ok().json(ReadinessResponse {
            status: "ok",
            database: db_status,
            redis: redis_status,
        })
    } else {
        HttpResponse::ServiceUnavailable().json(ReadinessResponse {
            status: "unhealthy",
            database: db_status,
            redis: redis_status,
        })
    }
}
