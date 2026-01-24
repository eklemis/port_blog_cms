use actix_web::{dev::Payload, Error as ActixError, FromRequest, HttpRequest};
use std::{
    future::{ready, Ready},
    sync::Arc,
};
use uuid::Uuid;

use crate::auth::application::ports::outgoing::token_provider::TokenProvider;

/// Represents an authenticated user (verified or not)
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
    pub is_verified: bool,
}

impl FromRequest for AuthenticatedUser {
    type Error = ActixError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let jwt_service =
            match req.app_data::<actix_web::web::Data<Arc<dyn TokenProvider + Send + Sync>>>() {
                Some(service) => service,
                None => {
                    return ready(Err(actix_web::error::ErrorInternalServerError(
                        "JWT service not configured",
                    )))
                }
            };

        // Extract token from Authorization header
        let token = match extract_token_from_header(req) {
            Some(t) => t,
            None => {
                return ready(Err(actix_web::error::ErrorUnauthorized(
                    "Missing or invalid authorization header",
                )))
            }
        };

        // Verify token
        match jwt_service.verify_token(&token) {
            Ok(claims) => {
                if claims.token_type != "access" {
                    return ready(Err(actix_web::error::ErrorUnauthorized(
                        "Invalid token type",
                    )));
                }

                ready(Ok(AuthenticatedUser {
                    user_id: claims.sub,
                    is_verified: claims.is_verified,
                }))
            }
            Err(e) => ready(Err(actix_web::error::ErrorUnauthorized(e))),
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
                    return ready(Err(actix_web::error::ErrorForbidden(
                        "Email verification required",
                    )));
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
