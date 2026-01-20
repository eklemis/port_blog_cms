use actix_web::{web, HttpResponse, Result};
use chrono::Utc;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use sea_orm::{DatabaseConnection, TransactionTrait};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::application::services::jwt::JwtClaims;

#[derive(Serialize)]
pub struct RandomAccountResponse {
    email: String,
    user_name: String,
    password: String,
}

#[derive(Serialize)]
pub struct CleanupResponse {
    deleted_resumes: u64,
    deleted_users: u64,
}

#[derive(Serialize)]
pub struct HealthResponse {
    status: String,
    environment: String,
}

#[derive(Serialize)]
pub struct TokenResponse {
    token: String,
}

#[derive(Debug)]
enum TokenKind {
    Valid,
    Expired,
    NotYetValid,
    InvalidSignature,
    Malformed,
}

impl std::str::FromStr for TokenKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Valid" => Ok(TokenKind::Valid),
            "Expired" => Ok(TokenKind::Expired),
            "NotYetValid" => Ok(TokenKind::NotYetValid),
            "InvalidSignature" => Ok(TokenKind::InvalidSignature),
            "Malformed" => Ok(TokenKind::Malformed),
            _ => Err(format!("Unknown token_kind: {}", s)),
        }
    }
}

#[derive(Debug)]
enum TokenType {
    Access,
    Refresh,
    Verification,
}

impl std::str::FromStr for TokenType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "access" => Ok(TokenType::Access),
            "refresh" => Ok(TokenType::Refresh),
            "verification" => Ok(TokenType::Verification),
            _ => Err(format!("Unknown token_type: {}", s)),
        }
    }
}

impl TokenType {
    fn as_str(&self) -> &str {
        match self {
            TokenType::Access => "access",
            TokenType::Refresh => "refresh",
            TokenType::Verification => "verification",
        }
    }
}

/// Generate random test credentials
/// GET /test/account/random
pub async fn generate_random_account() -> Result<HttpResponse> {
    let ts = chrono::Utc::now().timestamp();

    // Generate random suffix
    let random_suffix: String = (0..4)
        .map(|_| format!("{:x}", rand::random::<u8>() % 16))
        .collect();
    let random_suffix2: String = (0..4)
        .map(|_| format!("{:x}", rand::random::<u8>() % 16))
        .collect();

    let email = format!("user{}.{}@example.test", ts, random_suffix2);
    let user_name = format!("user_{}_{}", ts, random_suffix);

    // Ensure user_name is within bounds (3-50 chars)
    let safe_user_name = if user_name.len() > 50 {
        user_name[..50].to_string()
    } else {
        user_name
    };

    // Generate password (minimum 12 chars)
    let password = format!("{}{}", ts, random_suffix);
    let password = if password.len() < 12 {
        format!("{}_{}", password, random_suffix)
    } else {
        password
    };

    Ok(HttpResponse::Ok().json(RandomAccountResponse {
        email,
        user_name: safe_user_name,
        password,
    }))
}

/// Cleanup test data for a user
/// DELETE /test/cleanup/all/{user_id}
pub async fn cleanup_test_user(
    user_id: web::Path<Uuid>,
    db: web::Data<Arc<DatabaseConnection>>,
) -> Result<HttpResponse> {
    use sea_orm::{ConnectionTrait, Statement};

    let user_id = user_id.into_inner();

    // Dereference web::Data to get Arc<DatabaseConnection>, then dereference Arc to get &DatabaseConnection
    let txn = db.as_ref().begin().await.map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Transaction error: {}", e))
    })?;

    // Delete resumes first (foreign key constraint)
    let resumes_result = txn
        .execute(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "DELETE FROM resumes WHERE user_id = $1",
            vec![user_id.into()],
        ))
        .await
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Failed to delete resumes: {}", e))
        })?;

    // Delete user
    let user_result = txn
        .execute(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "DELETE FROM users WHERE id = $1",
            vec![user_id.into()],
        ))
        .await
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Failed to delete user: {}", e))
        })?;

    if user_result.rows_affected() == 0 {
        txn.rollback().await.ok();
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "User not found"
        })));
    }

    // Commit transaction
    txn.commit()
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Commit failed: {}", e)))?;

    Ok(HttpResponse::Ok().json(CleanupResponse {
        deleted_resumes: resumes_result.rows_affected(),
        deleted_users: user_result.rows_affected(),
    }))
}

/// Health check for test helpers
/// GET /test/health
pub async fn health_check() -> Result<HttpResponse> {
    let env = std::env::var("RUST_ENV").unwrap_or_else(|_| "development".to_string());

    // Additional safety check
    if env == "production" {
        tracing::error!("🚨 Test helper routes active in production!");
        return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "status": "error",
            "reason": "test-helper-running-in-production"
        })));
    }

    Ok(HttpResponse::Ok().json(HealthResponse {
        status: "ok".to_string(),
        environment: env,
    }))
}

/// Generate test JWT tokens with various states (Valid, Expired, NotYetValid, InvalidSignature, Malformed)
/// GET /test/token/{token_type}/{token_kind}/{user_id}?is_verified=true
pub async fn generate_test_token(
    path: web::Path<(String, String, String)>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> Result<HttpResponse> {
    let (token_type_str, token_kind_str, user_id_str) = path.into_inner();

    // Parse user_id
    let user_id = Uuid::parse_str(&user_id_str)
        .map_err(|_| actix_web::error::ErrorBadRequest("Invalid UUID format"))?;

    // Parse token_type
    let token_type: TokenType = token_type_str
        .parse()
        .map_err(|e: String| actix_web::error::ErrorBadRequest(e))?;

    // Parse token_kind
    let token_kind: TokenKind = token_kind_str
        .parse()
        .map_err(|e: String| actix_web::error::ErrorBadRequest(e))?;

    // Parse is_verified query param
    let is_verified = query
        .get("is_verified")
        .map(|v| v == "true")
        .unwrap_or(false);

    tracing::debug!(
        "Generating test token - Type: {}, Kind: {:?}, User ID: {}, Verified: {}",
        token_type.as_str(),
        token_kind,
        user_id,
        is_verified
    );

    // Get JWT secret from environment (must match production config)
    let valid_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "test-secret".to_string());

    // Intentionally wrong secret for InvalidSignature testing
    let invalid_secret = "wrong-secret";

    let now = Utc::now().timestamp();

    let (claims, secret) = match token_kind {
        TokenKind::Valid => {
            let claims = JwtClaims {
                sub: user_id,
                exp: now + 3600,
                iat: now,
                nbf: now - 32,
                token_type: token_type.as_str().to_string(),
                is_verified,
            };
            (claims, valid_secret.as_str())
        }
        TokenKind::Expired => {
            let claims = JwtClaims {
                sub: user_id,
                iat: now - 7200,
                nbf: now - 7200,
                exp: now - 60, // Expired 60 seconds ago
                token_type: token_type.as_str().to_string(),
                is_verified,
            };
            (claims, valid_secret.as_str())
        }
        TokenKind::NotYetValid => {
            let claims = JwtClaims {
                sub: user_id,
                iat: now,
                nbf: now + 300, // Not valid for another 5 minutes (> 30s leeway)
                exp: now + 3600,
                token_type: token_type.as_str().to_string(),
                is_verified,
            };
            (claims, valid_secret.as_str())
        }
        TokenKind::InvalidSignature => {
            let claims = JwtClaims {
                sub: user_id,
                iat: now,
                nbf: now,
                exp: now + 3600,
                token_type: token_type.as_str().to_string(),
                is_verified,
            };
            (claims, invalid_secret)
        }
        TokenKind::Malformed => {
            // Return a completely malformed token
            let malformed_token = format!("malformed.{}.token", Uuid::new_v4());
            return Ok(HttpResponse::Ok().json(TokenResponse {
                token: malformed_token,
            }));
        }
    };

    // Encode the token
    let encoding_key = EncodingKey::from_secret(secret.as_bytes());
    let token = encode(&Header::new(Algorithm::HS256), &claims, &encoding_key).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Token encoding error: {}", e))
    })?;

    Ok(HttpResponse::Ok().json(TokenResponse { token }))
}

/// Configure test helper routes
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/test")
            .route("/health", web::get().to(health_check))
            .route("/account/random", web::get().to(generate_random_account))
            .route(
                "/cleanup/all/{user_id}",
                web::delete().to(cleanup_test_user),
            )
            .route(
                "/token/{token_type}/{token_kind}/{user_id}",
                web::get().to(generate_test_token),
            ),
    );
}
