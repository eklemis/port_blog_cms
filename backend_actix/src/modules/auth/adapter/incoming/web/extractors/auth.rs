use actix_web::{dev::Payload, web, Error as ActixError, FromRequest, HttpRequest, HttpResponse};
use std::{
    future::{ready, Ready},
    sync::Arc,
};
use uuid::Uuid;

use crate::{auth::application::helpers::ResolveUserIdError, shared::api::ApiResponse};
use crate::{auth::application::ports::outgoing::token_provider::TokenProvider, AppState};

/// Represents an authenticated user (verified or not)
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
    pub is_verified: bool,
}

fn create_api_error(response: HttpResponse) -> ActixError {
    actix_web::error::InternalError::from_response("", response).into()
}

impl FromRequest for AuthenticatedUser {
    type Error = ActixError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let jwt_service =
            match req.app_data::<actix_web::web::Data<Arc<dyn TokenProvider + Send + Sync>>>() {
                Some(service) => service,
                None => {
                    return ready(Err(create_api_error(ApiResponse::internal_error())));
                }
            };

        // Extract token from Authorization header
        let token = match extract_token_from_header(req) {
            Some(t) => t,
            None => {
                return ready(Err(create_api_error(ApiResponse::unauthorized(
                    "MISSING_AUTH_HEADER",
                    "Missing or invalid authorization header",
                ))));
            }
        };

        // Verify token
        match jwt_service.verify_token(&token) {
            Ok(claims) => {
                if claims.token_type != "access" {
                    return ready(Err(create_api_error(ApiResponse::unauthorized(
                        "INVALID_TOKEN_TYPE",
                        "Invalid token type",
                    ))));
                }

                ready(Ok(AuthenticatedUser {
                    user_id: claims.sub,
                    is_verified: claims.is_verified,
                }))
            }
            Err(_) => ready(Err(create_api_error(ApiResponse::unauthorized(
                "INVALID_TOKEN",
                "Invalid or expired token",
            )))),
        }
    }
}

/// Represents a verified authenticated user
#[derive(Debug, Clone)]
pub struct VerifiedUser {
    pub user_id: Uuid,
}

impl FromRequest for VerifiedUser {
    type Error = ActixError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let auth_user_future = AuthenticatedUser::from_request(req, payload);

        match auth_user_future.into_inner() {
            Ok(auth_user) => {
                if !auth_user.is_verified {
                    return ready(Err(create_api_error(ApiResponse::forbidden(
                        "EMAIL_NOT_VERIFIED",
                        "Email verification required",
                    ))));
                }

                ready(Ok(VerifiedUser {
                    user_id: auth_user.user_id,
                }))
            }
            Err(e) => ready(Err(e)),
        }
    }
}

fn extract_token_from_header(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get("Authorization")?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(|s| s.to_string())
}

pub async fn resolve_owner_id_or_response(
    data: &web::Data<AppState>,
    username: &str,
) -> Result<Uuid, HttpResponse> {
    match data.user_identity_resolver.by_username(username).await {
        Ok(owner_id) => Ok(owner_id.value()),

        Err(ResolveUserIdError::NotFound) => {
            Err(ApiResponse::not_found("USER_NOT_FOUND", "User not found"))
        }

        Err(ResolveUserIdError::RepositoryError(msg)) => {
            tracing::error!("Repository error resolving username {}: {}", username, msg);
            Err(ApiResponse::internal_error())
        }
    }
}
