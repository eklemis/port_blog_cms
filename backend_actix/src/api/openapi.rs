use crate::api::schemas::{ErrorDetail, ErrorResponse, SuccessResponse};
use crate::blog::adapter::incoming::web::dto::{
    BlogPostCardResponse, BlogPostDetailResponse, BlogPostResponse, BlogPostTopicRequest,
    BlogPostTopicResponse, CreateBlogPostRequest, PatchBlogPostRequest,
};
use crate::blog::application::ports::outgoing::{BlogPageResult, BlogPostSort};
use crate::cv::adapter::incoming::web::dto::{
    ContactDetailDto, ContactTypeDto, CoreSkillDto, CvResponse, EducationDto, ExperienceDto,
    HighlightedProjectDto,
};
use crate::cv::adapter::incoming::web::routes::{
    CreateCVRequest, PatchCVRequest, ReplaceOp, UpdateCVRequest,
};
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::OpenApi;

// Auth
use crate::auth::adapter::incoming::web::routes::{
    CreateUserRequest, LoginRequestDto, LoginResponse, LoginUserInfo, LogoutRequestDto,
    LogoutResponseBody, RefreshTokenRequestDto, RefreshTokenResponseBody, RegisterUserResponse,
    RegisteredUser, UpdateUserRequest, UpdateUserResponse, UserProfileResponse,
    VerifyEmailResponse,
};
use crate::cv::application::ports::outgoing::{
    CVPageResult, // Just import the generic type
    CVSort,
};
use crate::multimedia::adapter::incoming::web::routes::{
    GetVariantUrlResponse, InitUploadRequest, InitUploadResponse, ListMediaResponse,
};
use crate::multimedia::application::domain::entities::{
    AttachmentTarget, MediaRole, MediaSize, MediaState,
};
use crate::multimedia::application::ports::incoming::use_cases::{MediaDetail, MediaItem};
use crate::project::adapter::incoming::web::routes::{
    AddProjectTopicRequest, CreateProjectRequest, PatchProjectRequest, RemoveProjectTopicRequest,
};
use crate::project::application::ports::outgoing::project_query::{
    PageResult, ProjectCardView, ProjectSort, ProjectTopicItem, ProjectView,
};
use crate::project::application::ports::outgoing::project_repository::ProjectResult;
use crate::topic::adapter::incoming::web::routes::{CreateTopicRequest, TopicResponse};
use crate::topic::application::ports::outgoing::TopicResult;

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
        // Health probes
        crate::health::health,
        crate::health::readiness,

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
        crate::cv::adapter::incoming::web::routes::create_cv_handler,
        crate::cv::adapter::incoming::web::routes::get_cvs_handler,
        crate::cv::adapter::incoming::web::routes::get_cv_by_id_handler,
        crate::cv::adapter::incoming::web::routes::get_public_cv_by_id_handler,
        crate::cv::adapter::incoming::web::routes::update_cv_handler,
        crate::cv::adapter::incoming::web::routes::patch_cv_handler,
        crate::cv::adapter::incoming::web::routes::hard_delete_cv_handler,
        crate::cv::adapter::incoming::web::routes::soft_delete_cv_handler,
        crate::cv::adapter::incoming::web::routes::restore_cv_handler,

        // Blog endpoints
        crate::blog::adapter::incoming::web::routes::create_blog_post_handler,
        crate::blog::adapter::incoming::web::routes::get_blog_posts_handler,
        crate::blog::adapter::incoming::web::routes::get_single_blog_post_handler,
        crate::blog::adapter::incoming::web::routes::patch_blog_post_handler,
        crate::blog::adapter::incoming::web::routes::archive_blog_post_handler,
        crate::blog::adapter::incoming::web::routes::restore_blog_post_handler,
        crate::blog::adapter::incoming::web::routes::hard_delete_blog_post_handler,
        crate::blog::adapter::incoming::web::routes::get_public_blog_posts_handler,
        crate::blog::adapter::incoming::web::routes::get_public_blog_post_handler,
        crate::blog::adapter::incoming::web::routes::attach_blog_post_topic_handler,
        crate::blog::adapter::incoming::web::routes::detach_blog_post_topic_handler,
        crate::blog::adapter::incoming::web::routes::clear_blog_post_topics_handler,
        crate::blog::adapter::incoming::web::routes::get_blog_post_topics_handler,

        // Project endpoints
        crate::project::adapter::incoming::web::routes::create_project_handler,
        crate::project::adapter::incoming::web::routes::get_projects_handler,
        crate::project::adapter::incoming::web::routes::get_public_projects_handler,
        crate::project::adapter::incoming::web::routes::get_project_by_id_handler,
        crate::project::adapter::incoming::web::routes::get_public_single_project_handler,
        crate::project::adapter::incoming::web::routes::patch_project_handler,
        crate::project::adapter::incoming::web::routes::soft_delete_project_handler,
        crate::project::adapter::incoming::web::routes::hard_delete_project_handler,
        crate::project::adapter::incoming::web::routes::add_project_topic_handler,
        crate::project::adapter::incoming::web::routes::remove_project_topic_handler,
        crate::project::adapter::incoming::web::routes::get_project_topics_handler,
        crate::project::adapter::incoming::web::routes::clear_project_topics_handler,

        // Topic endpoints
        crate::topic::adapter::incoming::web::routes::create_topic_handler,
        crate::topic::adapter::incoming::web::routes::get_topics_handler,
        crate::topic::adapter::incoming::web::routes::soft_delete_topic_handler,

        // Media endpoints
        crate::multimedia::adapter::incoming::web::routes::init_upload_handler,
        crate::multimedia::adapter::incoming::web::routes::get_variant_read_url_handler,
        crate::multimedia::adapter::incoming::web::routes::list_media_handler,
        crate::multimedia::adapter::incoming::web::routes::delete_media_handler,
        crate::multimedia::adapter::incoming::web::routes::get_media_handler,
    ),
    components(
        schemas(
            // Response wrappers
            SuccessResponse<RegisterUserResponse>,
            ErrorResponse,
            ErrorDetail,

            // Health probes
            crate::health::HealthResponse,
            crate::health::ReadinessResponse,

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
            VerifyEmailResponse,

            // CV wire types. These live in the adapter, not the domain, so
            // `cv::domain::entities` carries no HTTP concerns.
            CvResponse,
            CoreSkillDto,
            EducationDto,
            ExperienceDto,
            HighlightedProjectDto,
            ContactDetailDto,
            ContactTypeDto,
            CVPageResult<CvResponse>,
            CVSort,

            // CV request bodies
            CreateCVRequest,
            UpdateCVRequest,
            PatchCVRequest,
            ReplaceOp<CoreSkillDto>,
            ReplaceOp<EducationDto>,
            ReplaceOp<ExperienceDto>,
            ReplaceOp<HighlightedProjectDto>,
            ReplaceOp<ContactDetailDto>,
            // Topic DTOs
            CreateTopicRequest,
            TopicResponse,
            TopicResult,

            // Blog DTOs
            BlogPostResponse,
            BlogPostDetailResponse,
            BlogPostCardResponse,
            BlogPostTopicResponse,
            CreateBlogPostRequest,
            PatchBlogPostRequest,
            BlogPostTopicRequest,
            BlogPostSort,
            BlogPageResult<BlogPostCardResponse>,

            // Project DTOs
            CreateProjectRequest,
            PatchProjectRequest,
            AddProjectTopicRequest,
            RemoveProjectTopicRequest,
            ProjectResult,
            ProjectView,
            ProjectCardView,
            ProjectTopicItem,
            ProjectSort,
            PageResult<ProjectCardView>,

            // Media DTOs
            InitUploadRequest,
            InitUploadResponse,
            GetVariantUrlResponse,
            ListMediaResponse,
            MediaItem,
            MediaDetail,
            AttachmentTarget,
            MediaRole,
            MediaSize,
            MediaState,

        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "health", description = "Liveness and readiness probes"),
        (name = "auth", description = "Authentication endpoints"),
        (name = "users", description = "User management endpoints"),
        (name = "cvs", description = "CV/Resume management endpoints"),
        (name = "blog", description = "Blog post authoring and publication"),
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::collections::BTreeSet;

    fn doc() -> Value {
        serde_json::to_value(ApiDoc::openapi()).expect("ApiDoc must serialize")
    }

    /// Collects every `#/components/schemas/X` reference anywhere in the document.
    fn collect_refs(node: &Value, out: &mut BTreeSet<String>) {
        match node {
            Value::Object(map) => {
                for (key, value) in map {
                    if key == "$ref" {
                        if let Some(name) = value
                            .as_str()
                            .and_then(|r| r.strip_prefix("#/components/schemas/"))
                        {
                            out.insert(name.to_string());
                        }
                    }
                    collect_refs(value, out);
                }
            }
            Value::Array(items) => {
                for item in items {
                    collect_refs(item, out);
                }
            }
            _ => {}
        }
    }

    fn defined_schemas(doc: &Value) -> BTreeSet<String> {
        doc["components"]["schemas"]
            .as_object()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// The failure this guards against: documenting a handler whose request or
    /// response body type was never added to `components(schemas(...))`. utoipa
    /// still emits a `$ref` to it, so Swagger UI renders a broken, empty model
    /// and nothing fails at compile time.
    #[test]
    fn every_schema_reference_resolves_to_a_defined_component() {
        let doc = doc();

        let mut referenced = BTreeSet::new();
        collect_refs(&doc, &mut referenced);

        let defined = defined_schemas(&doc);
        let dangling: Vec<_> = referenced.difference(&defined).collect();

        assert!(
            dangling.is_empty(),
            "OpenAPI document references schemas that are not registered in \
             components(schemas(...)): {dangling:?}"
        );
    }

    #[test]
    fn documents_every_registered_path_with_at_least_one_response() {
        let doc = doc();
        let paths = doc["paths"].as_object().expect("paths object");

        assert!(!paths.is_empty(), "no paths documented");

        for (path, item) in paths {
            let operations = item.as_object().expect("path item object");
            for (method, operation) in operations {
                let responses = operation["responses"]
                    .as_object()
                    .unwrap_or_else(|| panic!("{method} {path} has no responses block"));
                assert!(
                    !responses.is_empty(),
                    "{method} {path} documents no responses"
                );
            }
        }
    }

    /// Walks the source tree for Actix route macros and checks each one appears
    /// in the document. This is what caught `/health` and `/ready` sitting
    /// undocumented; a hand-maintained list would have drifted instead.
    ///
    /// Recognises both the bare attribute form and the fully qualified
    /// `actix_web::`-prefixed form, since the codebase uses both. The patterns
    /// are assembled at runtime rather than written literally, so that this
    /// comment cannot match its own scanner.
    #[test]
    fn every_registered_route_is_documented() {
        use std::fs;

        fn visit(dir: &std::path::Path, found: &mut BTreeSet<String>) {
            for entry in fs::read_dir(dir).expect("readable source dir") {
                let path = entry.expect("readable entry").path();
                if path.is_dir() {
                    visit(&path, found);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    let text = fs::read_to_string(&path).expect("readable source file");
                    for (idx, _) in text.match_indices("#[") {
                        let rest = &text[idx + 2..];
                        let rest = rest.strip_prefix("actix_web::").unwrap_or(rest);
                        for method in ["get", "post", "put", "patch", "delete"] {
                            let prefix = format!("{method}(\"");
                            if let Some(tail) = rest.strip_prefix(prefix.as_str()) {
                                if let Some(end) = tail.find('"') {
                                    found.insert(format!("{} {}", method, &tail[..end]));
                                }
                            }
                        }
                    }
                }
            }
        }

        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut registered = BTreeSet::new();
        visit(&src, &mut registered);

        assert!(
            registered.len() > 20,
            "route scan found only {} routes; the scanner is probably broken",
            registered.len()
        );

        let doc = doc();
        let mut documented = BTreeSet::new();
        for (path, item) in doc["paths"].as_object().expect("paths object") {
            for method in item.as_object().expect("path item").keys() {
                documented.insert(format!("{method} {path}"));
            }
        }

        let missing: Vec<_> = registered.difference(&documented).collect();
        assert!(
            missing.is_empty(),
            "these routes are served but absent from the OpenAPI document: {missing:?}"
        );
    }

    /// Collects the keys of every `properties` object at any depth, which is
    /// exactly the set of JSON field names the API puts on the wire. Schema
    /// names themselves are PascalCase by convention and are not collected.
    fn collect_property_names(node: &Value, out: &mut BTreeSet<String>) {
        match node {
            Value::Object(map) => {
                if let Some(Value::Object(props)) = map.get("properties") {
                    out.extend(props.keys().cloned());
                }
                for value in map.values() {
                    collect_property_names(value, out);
                }
            }
            Value::Array(items) => {
                for item in items {
                    collect_property_names(item, out);
                }
            }
            _ => {}
        }
    }

    /// The API serialises snake_case throughout. Two structs in `init_upload`
    /// once carried `rename_all = "camelCase"`, which made a single endpoint
    /// disagree with every other one and was easy to miss by reading code.
    /// Pinning it here means a stray rename fails the suite instead of
    /// surprising a client.
    ///
    /// This governs field names only. Enum *values* are lowercase by design
    /// (`MediaSize::Thumbnail` serialises as `thumbnail`) and are not
    /// property names, so they are unaffected.
    #[test]
    fn every_response_field_is_snake_case() {
        let doc = doc();

        let mut properties = BTreeSet::new();
        collect_property_names(&doc, &mut properties);

        assert!(
            properties.len() > 40,
            "only {} properties found; the collector is probably broken",
            properties.len()
        );

        let offenders: Vec<_> = properties
            .iter()
            .filter(|name| name.chars().any(|c| c.is_uppercase()))
            .collect();

        assert!(
            offenders.is_empty(),
            "these API fields are not snake_case: {offenders:?}"
        );
    }
}

