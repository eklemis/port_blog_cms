use actix_web::{get, web, HttpResponse, Responder};
use deadpool_redis::redis::cmd;
use deadpool_redis::Pool;
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use serde::Serialize;
use std::sync::Arc;

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
///
/// The Redis handle must match what `main.rs` registers: a `deadpool_redis::Pool`.
/// Asking for any other type here makes the extractor fail and the probe 500 before
/// a single dependency is checked.
#[get("/ready")]
pub async fn readiness(
    db: web::Data<Arc<DatabaseConnection>>,
    redis: web::Data<Arc<Pool>>,
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

    let redis_status = match redis.get().await {
        Ok(mut conn) => match cmd("PING").query_async::<String>(&mut conn).await {
            Ok(_) => "ok",
            Err(_) => "unhealthy",
        },
        Err(_) => "unhealthy",
    };

    if db_status == "ok" && redis_status == "ok" {
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

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{http::StatusCode, test, App};
    use deadpool_redis::{Config, Runtime};
    use sea_orm::{DatabaseBackend, DbErr, MockDatabase, MockExecResult};
    use serde_json::Value;

    fn mock_db_ok() -> DatabaseConnection {
        MockDatabase::new(DatabaseBackend::Postgres)
            .append_exec_results([MockExecResult {
                last_insert_id: 0,
                rows_affected: 1,
            }])
            .into_connection()
    }

    fn mock_db_err() -> DatabaseConnection {
        MockDatabase::new(DatabaseBackend::Postgres)
            .append_exec_errors([DbErr::Custom("db down".to_string())])
            .into_connection()
    }

    /// Points at a port nothing listens on, so `pool.get()` always fails.
    /// Keeps the readiness tests hermetic — no live Redis required.
    fn unreachable_redis_pool() -> Pool {
        Config::from_url("redis://127.0.0.1:1")
            .create_pool(Some(Runtime::Tokio1))
            .expect("pool config is valid even when the server is absent")
    }

    #[actix_web::test]
    async fn health_reports_ok_without_touching_dependencies() {
        let app = test::init_service(App::new().service(health)).await;

        let req = test::TestRequest::get().uri("/health").to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["status"], "ok");
    }

    /// Regression test: `readiness` must ask for the same Redis type `main.rs`
    /// registers. If the extractor cannot resolve its app data, Actix answers
    /// 500 and no dependency is ever checked — so a 503 with a populated body
    /// is what proves the wiring is right.
    #[actix_web::test]
    async fn readiness_reports_unhealthy_when_redis_is_unreachable() {
        let db: web::Data<Arc<DatabaseConnection>> = web::Data::new(Arc::new(mock_db_ok()));
        let redis: web::Data<Arc<Pool>> = web::Data::new(Arc::new(unreachable_redis_pool()));

        let app = test::init_service(
            App::new()
                .app_data(db)
                .app_data(redis)
                .service(readiness),
        )
        .await;

        let req = test::TestRequest::get().uri("/ready").to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["status"], "unhealthy");
        assert_eq!(body["database"], "ok");
        assert_eq!(body["redis"], "unhealthy");
    }

    #[actix_web::test]
    async fn readiness_reports_database_unhealthy_when_query_fails() {
        let db: web::Data<Arc<DatabaseConnection>> = web::Data::new(Arc::new(mock_db_err()));
        let redis: web::Data<Arc<Pool>> = web::Data::new(Arc::new(unreachable_redis_pool()));

        let app = test::init_service(
            App::new()
                .app_data(db)
                .app_data(redis)
                .service(readiness),
        )
        .await;

        let req = test::TestRequest::get().uri("/ready").to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["database"], "unhealthy");
        assert_eq!(body["redis"], "unhealthy");
    }
}
