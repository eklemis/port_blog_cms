use crate::api::schemas::{ErrorDetail, ErrorResponse, SuccessResponse};
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::OpenApi;

// Auth
use crate::auth::adapter::incoming::web::routes::{
    CreateUserRequest, LoginRequestDto, LoginResponse, LoginUserInfo, LogoutRequestDto,
    LogoutResponseBody, RefreshTokenRequestDto, RefreshTokenResponseBody, RegisterUserResponse,
    RegisteredUser, UpdateUserRequest, UpdateUserResponse, UserProfileResponse,
    VerifyEmailResponse,
};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Portfolio CMS API",
        version = "1.0.0",
        description = "API documentation for Portfolio Content Management System",
        contact(
            name = "API Support",
            email = "support@example.com"
        )
    ),
    paths(
        // Auth endpoints
        crate::auth::adapter::incoming::web::routes::register_user_handler,
        crate::auth::adapter::incoming::web::routes::login_user_handler,
        crate::auth::adapter::incoming::web::routes::logout_user_handler,
        crate::auth::adapter::incoming::web::routes::get_user_profile_handler,
        crate::auth::adapter::incoming::web::routes::refresh_token_handler,
        crate::auth::adapter::incoming::web::routes::verify_user_email_handler,

        // User endpoints
        crate::auth::adapter::incoming::web::routes::update_user_profile_handler,
        crate::auth::adapter::incoming::web::routes::soft_delete_user_handler,

        // CV endpoints
        // create_cv_handler,
        // get_cvs_handler,
        // get_cv_by_id_handler,
        // get_public_cv_by_id_handler,
        // update_cv_handler,
        // patch_cv_handler,
        // hard_delete_cv_handler,

        // Project endpoints
        // create_project_handler,
        // get_projects_handler,
        // get_public_projects_handler,
        // get_project_by_id_handler,
        // get_public_single_project_handler,
        // patch_project_handler,
        // hard_delete_project_handler,
        // add_project_topic_handler,
        // remove_project_topic_handler,
        // get_project_topics_handler,
        // clear_project_topics_handler,

        // Topic endpoints
        // create_topic_handler,
        // get_topics_handler,
        // soft_delete_topic_handler,

        // Media endpoints
        // init_upload_handler,
        // get_variant_read_url_handler,
        // list_media_handler,
    ),
    components(
        schemas(
            // Response wrappers
            SuccessResponse<RegisterUserResponse>,
            ErrorResponse,
            ErrorDetail,

            // Auth DTOs
            CreateUserRequest,
            RegisterUserResponse,
            RegisteredUser,
            LoginRequestDto,
            LoginResponse,
            LoginUserInfo,
            LogoutRequestDto,
            LogoutResponseBody,
            UserProfileResponse,
            RefreshTokenRequestDto,
            RefreshTokenResponseBody,
            UpdateUserRequest,
            UpdateUserResponse,
            VerifyEmailResponse
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "auth", description = "Authentication endpoints"),
        (name = "users", description = "User management endpoints"),
        (name = "cvs", description = "CV/Resume management endpoints"),
        (name = "projects", description = "Project management endpoints"),
        (name = "topics", description = "Topic management endpoints"),
        (name = "media", description = "Media/file management endpoints"),
    )
)]
pub struct ApiDoc;

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "BearerAuth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .description(Some("Enter your JWT token"))
                        .build(),
                ),
            )
        }
    }
}
