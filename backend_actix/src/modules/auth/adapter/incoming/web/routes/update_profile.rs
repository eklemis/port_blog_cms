use crate::api::schemas::{ErrorResponse, SuccessResponse};
use crate::{
    auth::{
        adapter::incoming::web::extractors::auth::AuthenticatedUser,
        application::{
            domain::entities::UserId,
            use_cases::update_profile::{UpdateUserError, UpdateUserInput},
        },
    },
    shared::api::ApiResponse,
    AppState,
};
use actix_web::{put, web, Responder};
use serde::{Deserialize, Serialize};
use tracing::error;
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    /// New full name for the user
    #[schema(example = "John Smith")]
    full_name: String,
}

#[derive(Serialize, ToSchema)]
pub struct UpdateUserResponse {
    /// User ID (UUID)
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    user_id: String,

    /// Email address
    #[schema(example = "john@example.com")]
    email: String,

    /// Username
    #[schema(example = "johndoe")]
    username: String,

    /// Updated full name
    #[schema(example = "John Smith")]
    full_name: String,
}

/// Update current user profile
///
/// Updates the authenticated user's profile information. Currently only full name can be updated.
#[utoipa::path(
    put,
    path = "/api/users/me",
    tag = "users",
    request_body = UpdateUserRequest,
    responses(
        (
            status = 200,
            description = "Profile updated successfully",
            body = inline(SuccessResponse<UpdateUserResponse>),
            example = json!({
                "success": true,
                "data": {
                    "userId": "123e4567-e89b-12d3-a456-426614174000",
                    "email": "john@example.com",
                    "username": "johndoe",
                    "fullName": "John Smith"
                }
            })
        ),
        (
            status = 400,
            description = "Invalid full name",
            body = ErrorResponse,
            example = json!({
                "success": false,
                "error": {
                    "code": "INVALID_FULL_NAME",
                    "message": "Full name must be between 2 and 100 characters"
                }
            })
        ),
        (
            status = 401,
            description = "Not authenticated",
            body = ErrorResponse,
            example = json!({
                "success": false,
                "error": {
                    "code": "UNAUTHORIZED",
                    "message": "Authentication required"
                }
            })
        ),
        (
            status = 500,
            description = "Internal server error",
            body = ErrorResponse,
            example = json!({
                "success": false,
                "error": {
                    "code": "INTERNAL_ERROR",
                    "message": "An unexpected error occurred"
                }
            })
        ),
    ),
    security(
        ("BearerAuth" = [])
    )
)]
#[put("/api/users/me")]
pub async fn update_user_profile_handler(
    user: AuthenticatedUser,
    req: web::Json<UpdateUserRequest>,
    app_data: web::Data<AppState>,
) -> impl Responder {
    let input = UpdateUserInput {
        user_id: UserId::from(user.user_id),
        full_name: req.into_inner().full_name,
    };

    match app_data.update_user_profile_use_case.execute(input).await {
        Ok(output) => ApiResponse::success(UpdateUserResponse {
            user_id: output.user_id.value().to_string(),
            email: output.email,
            username: output.username,
            full_name: output.full_name,
        }),
        Err(UpdateUserError::InvalidFullName(msg)) => {
            ApiResponse::bad_request("INVALID_FULL_NAME", &msg)
        }
        Err(UpdateUserError::RepositoryError(e)) => {
            error!("Repository error updating user profile: {}", e);
            ApiResponse::internal_error()
        }
        Err(UpdateUserError::QueryError(e)) => {
            error!("Query error updating user profile: {}", e);
            ApiResponse::internal_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        auth::application::{
            ports::outgoing::{
                token_provider::{TokenClaims, TokenError, TokenProvider},
                user_query::UserQueryError,
                user_repository::UserRepositoryError,
            },
            use_cases::update_profile::{
                UpdateUserError, UpdateUserInput, UpdateUserOutput, UpdateUserProfileUseCase,
            },
        },
        tests::support::app_state_builder::TestAppStateBuilder,
    };
    use actix_web::{test, web, App};
    use async_trait::async_trait;
    use std::sync::Arc;
    use uuid::Uuid;

    struct MockUpdateUserProfileUseCase {
        result: Result<UpdateUserOutput, UpdateUserError>,
    }

    #[async_trait]
    impl UpdateUserProfileUseCase for MockUpdateUserProfileUseCase {
        async fn execute(
            &self,
            _data: UpdateUserInput,
        ) -> Result<UpdateUserOutput, UpdateUserError> {
            self.result.clone()
        }
    }

    struct MockTokenProvider {
        user_id: Uuid,
        is_verified: bool,
    }

    impl TokenProvider for MockTokenProvider {
        fn generate_access_token(
            &self,
            _user_id: Uuid,
            _is_verified: bool,
        ) -> Result<String, TokenError> {
            unimplemented!()
        }

        fn generate_refresh_token(
            &self,
            _user_id: Uuid,
            _is_verified: bool,
        ) -> Result<String, TokenError> {
            unimplemented!()
        }

        fn verify_token(&self, _token: &str) -> Result<TokenClaims, TokenError> {
            Ok(TokenClaims {
                sub: self.user_id,
                token_type: "access".to_string(),
                is_verified: self.is_verified,
                exp: 9999999999,
                iat: 0,
                nbf: 0,
            })
        }

        fn refresh_access_token(&self, _refresh_token: &str) -> Result<String, TokenError> {
            unimplemented!()
        }

        fn generate_verification_token(&self, _user_id: Uuid) -> Result<String, TokenError> {
            unimplemented!()
        }

        fn verify_verification_token(&self, _token: &str) -> Result<Uuid, TokenError> {
            unimplemented!()
        }
    }

    fn create_update_user_output(user_id: Uuid, full_name: &str) -> UpdateUserOutput {
        UpdateUserOutput {
            user_id: user_id.into(),
            email: "test@example.com".to_string(),
            username: "testuser".to_string(),
            full_name: full_name.to_string(),
        }
    }

    fn create_token_provider(
        user_id: Uuid,
        is_verified: bool,
    ) -> web::Data<Arc<dyn TokenProvider + Send + Sync>> {
        web::Data::new(Arc::new(MockTokenProvider {
            user_id,
            is_verified,
        }) as Arc<dyn TokenProvider + Send + Sync>)
    }

    #[actix_web::test]
    async fn test_update_user_profile_success() {
        let user_id = Uuid::new_v4();
        let new_full_name = "Updated Name";

        let mock_use_case = MockUpdateUserProfileUseCase {
            result: Ok(create_update_user_output(user_id, new_full_name)),
        };

        let app_state = TestAppStateBuilder::default()
            .with_update_user_profile(mock_use_case)
            .build();

        let token_provider = create_token_provider(user_id, true);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(token_provider)
                .service(update_user_profile_handler),
        )
        .await;

        let req = test::TestRequest::put()
            .uri("/api/users/me")
            .insert_header(("Authorization", "Bearer test_token"))
            .insert_header(("Content-Type", "application/json"))
            .set_json(serde_json::json!({
                "full_name": new_full_name
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert_eq!(body["data"]["user_id"], user_id.to_string());
        assert_eq!(body["data"]["email"], "test@example.com");
        assert_eq!(body["data"]["username"], "testuser");
        assert_eq!(body["data"]["full_name"], new_full_name);
        assert!(body.get("error").is_none());
    }

    #[actix_web::test]
    async fn test_update_user_profile_invalid_full_name() {
        let user_id = Uuid::new_v4();

        let mock_use_case = MockUpdateUserProfileUseCase {
            result: Err(UpdateUserError::InvalidFullName(
                "Full name must be 2-100 characters".to_string(),
            )),
        };

        let app_state = TestAppStateBuilder::default()
            .with_update_user_profile(mock_use_case)
            .build();

        let token_provider = create_token_provider(user_id, true);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(token_provider)
                .service(update_user_profile_handler),
        )
        .await;

        let req = test::TestRequest::put()
            .uri("/api/users/me")
            .insert_header(("Authorization", "Bearer test_token"))
            .insert_header(("Content-Type", "application/json"))
            .set_json(serde_json::json!({
                "full_name": "A"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 400);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "INVALID_FULL_NAME");
        assert!(body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("2-100 characters"));
        assert!(body.get("data").is_none());
    }

    #[actix_web::test]
    async fn test_update_user_profile_repository_error() {
        let user_id = Uuid::new_v4();

        let mock_use_case = MockUpdateUserProfileUseCase {
            result: Err(UpdateUserError::RepositoryError(
                UserRepositoryError::DatabaseError("Connection failed".to_string()),
            )),
        };

        let app_state = TestAppStateBuilder::default()
            .with_update_user_profile(mock_use_case)
            .build();

        let token_provider = create_token_provider(user_id, true);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(token_provider)
                .service(update_user_profile_handler),
        )
        .await;

        let req = test::TestRequest::put()
            .uri("/api/users/me")
            .insert_header(("Authorization", "Bearer test_token"))
            .insert_header(("Content-Type", "application/json"))
            .set_json(serde_json::json!({
                "full_name": "Valid Name"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 500);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "INTERNAL_ERROR");
        assert_eq!(body["error"]["message"], "An unexpected error occurred");
        assert!(body.get("data").is_none());
    }

    #[actix_web::test]
    async fn test_update_user_profile_query_error() {
        let user_id = Uuid::new_v4();

        let mock_use_case = MockUpdateUserProfileUseCase {
            result: Err(UpdateUserError::QueryError(UserQueryError::DatabaseError(
                "Query failed".to_string(),
            ))),
        };

        let app_state = TestAppStateBuilder::default()
            .with_update_user_profile(mock_use_case)
            .build();

        let token_provider = create_token_provider(user_id, true);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(token_provider)
                .service(update_user_profile_handler),
        )
        .await;

        let req = test::TestRequest::put()
            .uri("/api/users/me")
            .insert_header(("Authorization", "Bearer test_token"))
            .insert_header(("Content-Type", "application/json"))
            .set_json(serde_json::json!({
                "full_name": "Valid Name"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 500);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "INTERNAL_ERROR");
        assert_eq!(body["error"]["message"], "An unexpected error occurred");
        assert!(body.get("data").is_none());
    }
}
