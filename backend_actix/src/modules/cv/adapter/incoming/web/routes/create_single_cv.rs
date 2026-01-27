use crate::auth::adapter::incoming::web::extractors::auth::VerifiedUser;
use crate::cv::application::ports::outgoing::CreateCVData;
use crate::cv::application::use_cases::create_cv::CreateCVError;
use crate::cv::domain::entities::{
    ContactDetail, CoreSkill, Education, Experience, HighlightedProject,
};
use crate::shared::api::ApiResponse;
use crate::AppState;
use actix_web::{post, web, Responder};
use serde::{Deserialize, Serialize};
use tracing::error;

#[derive(Deserialize, Serialize)]
pub struct CreateCVRequest {
    pub role: String,
    pub bio: String,
    pub display_name: String,
    pub photo_url: String,
    pub core_skills: Vec<CoreSkill>,
    pub educations: Vec<EducationRequest>,
    pub experiences: Vec<ExperienceRequest>,
    pub highlighted_projects: Vec<HighlightedProjectRequest>,
    pub contact_info: Vec<ContactDetail>,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct EducationRequest {
    pub degree: String,
    pub institution: String,
    pub graduation_year: i32,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct ExperienceRequest {
    pub company: String,
    pub position: String,
    pub location: String,
    pub start_date: String,
    pub end_date: Option<String>,
    pub description: String,
    pub tasks: Vec<String>,
    pub achievements: Vec<String>,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct HighlightedProjectRequest {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub short_description: String,
}

#[post("/api/cvs")]
pub async fn create_cv_handler(
    user: VerifiedUser,
    req: web::Json<CreateCVRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let req = req.into_inner();

    let cv_data = CreateCVData {
        role: req.role,
        bio: req.bio,
        display_name: req.display_name,
        photo_url: req.photo_url,
        core_skills: req
            .core_skills
            .into_iter()
            .map(|e| CoreSkill {
                title: e.title,
                description: e.description,
            })
            .collect(),
        educations: req
            .educations
            .into_iter()
            .map(|e| Education {
                degree: e.degree,
                institution: e.institution,
                graduation_year: e.graduation_year,
            })
            .collect(),
        experiences: req
            .experiences
            .into_iter()
            .map(|exp| Experience {
                company: exp.company,
                position: exp.position,
                location: exp.location,
                start_date: exp.start_date,
                end_date: exp.end_date,
                description: exp.description,
                tasks: exp.tasks,
                achievements: exp.achievements,
            })
            .collect(),
        highlighted_projects: req
            .highlighted_projects
            .into_iter()
            .map(|hp| HighlightedProject {
                id: hp.id,
                title: hp.title,
                slug: hp.slug,
                short_description: hp.short_description,
            })
            .collect(),
        contact_info: req
            .contact_info
            .into_iter()
            .map(|cd| ContactDetail {
                title: cd.title,
                contact_type: cd.contact_type,
                content: cd.content,
            })
            .collect(),
    };

    match data.create_cv_use_case.execute(user.user_id, cv_data).await {
        Ok(created) => ApiResponse::created(created),

        Err(CreateCVError::RepositoryError(e)) => {
            error!("Repository error creating CV: {}", e);
            ApiResponse::internal_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::adapter::outgoing::jwt::{JwtConfig, JwtTokenService};
    use crate::auth::application::ports::outgoing::token_provider::TokenProvider;
    use crate::cv::application::ports::outgoing::CreateCVData;
    use crate::cv::application::use_cases::create_cv::{CreateCVError, ICreateCVUseCase};
    use crate::cv::domain::entities::{
        CVInfo, ContactDetail, ContactType, CoreSkill, Education, Experience, HighlightedProject,
    };
    use crate::tests::support::app_state_builder::TestAppStateBuilder;
    use actix_web::{http::StatusCode, test, web, App};
    use async_trait::async_trait;
    use serde_json::Value;
    use std::sync::Arc;
    use uuid::Uuid;

    /* --------------------------------------------------
     * Mock Create CV Use Case
     * -------------------------------------------------- */

    #[derive(Clone)]
    struct MockCreateCVUseCase {
        result: Result<CVInfo, CreateCVError>,
    }

    impl MockCreateCVUseCase {
        fn success(cv: CVInfo) -> Self {
            Self { result: Ok(cv) }
        }

        fn error(err: CreateCVError) -> Self {
            Self { result: Err(err) }
        }
    }

    #[async_trait]
    impl ICreateCVUseCase for MockCreateCVUseCase {
        async fn execute(
            &self,
            _user_id: Uuid,
            _cv_data: CreateCVData,
        ) -> Result<CVInfo, CreateCVError> {
            self.result.clone()
        }
    }

    /* --------------------------------------------------
     * Helpers
     * -------------------------------------------------- */

    fn jwt_service() -> JwtTokenService {
        JwtTokenService::new(JwtConfig {
            issuer: "Lotion".to_string(),
            secret_key: "test_secret_key_for_testing_purposes_only".to_string(),
            access_token_expiry: 3600,
            refresh_token_expiry: 86400,
            verification_token_expiry: 86400,
        })
    }

    fn token(user_id: Uuid, verified: bool) -> String {
        jwt_service()
            .generate_access_token(user_id, verified)
            .unwrap()
    }

    fn base_create_request() -> CreateCVRequest {
        CreateCVRequest {
            display_name: "John Doe".to_string(),
            role: "Software Engineer".to_string(),
            bio: "Experienced developer passionate about clean code".to_string(),
            photo_url: "https://example.com/photo.jpg".to_string(),
            core_skills: vec![],
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
            contact_info: vec![],
        }
    }

    fn full_request() -> CreateCVRequest {
        CreateCVRequest {
            core_skills: vec![
                CoreSkill {
                    title: "Rust".to_string(),
                    description: "Systems programming".to_string(),
                },
                CoreSkill {
                    title: "Python".to_string(),
                    description: "Backend development".to_string(),
                },
            ],
            educations: vec![EducationRequest {
                degree: "B.Sc. Computer Science".to_string(),
                institution: "MIT".to_string(),
                graduation_year: 2020,
            }],
            experiences: vec![ExperienceRequest {
                company: "TechCorp".to_string(),
                position: "Senior Developer".to_string(),
                location: "San Francisco, CA".to_string(),
                start_date: "2020-01-01".to_string(),
                end_date: Some("2023-12-31".to_string()),
                description: "Led backend development".to_string(),
                tasks: vec!["Designed APIs".to_string(), "Mentored juniors".to_string()],
                achievements: vec!["Increased performance by 50%".to_string()],
            }],
            highlighted_projects: vec![HighlightedProjectRequest {
                id: "proj-1".to_string(),
                title: "E-commerce Platform".to_string(),
                slug: "ecommerce-platform".to_string(),
                short_description: "Full-stack e-commerce solution".to_string(),
            }],
            contact_info: vec![
                ContactDetail {
                    contact_type: ContactType::PhoneNumber,
                    title: "Mobile".to_string(),
                    content: "+1234567890".to_string(),
                },
                ContactDetail {
                    contact_type: ContactType::WebPage,
                    title: "LinkedIn".to_string(),
                    content: "https://linkedin.com/in/johndoe".to_string(),
                },
            ],
            ..base_create_request()
        }
    }

    fn full_cv(user_id: Uuid) -> CVInfo {
        CVInfo {
            id: Uuid::new_v4(),
            user_id,
            display_name: "John Doe".to_string(),
            role: "Software Engineer".to_string(),
            bio: "Experienced developer passionate about clean code".to_string(),
            photo_url: "https://example.com/photo.jpg".to_string(),
            core_skills: full_request().core_skills,
            educations: vec![Education {
                degree: "B.Sc. Computer Science".to_string(),
                institution: "MIT".to_string(),
                graduation_year: 2020,
            }],
            experiences: vec![Experience {
                company: "TechCorp".to_string(),
                position: "Senior Developer".to_string(),
                location: "San Francisco, CA".to_string(),
                start_date: "2020-01-01".to_string(),
                end_date: Some("2023-12-31".to_string()),
                description: "Led backend development".to_string(),
                tasks: vec!["Designed APIs".to_string(), "Mentored juniors".to_string()],
                achievements: vec!["Increased performance by 50%".to_string()],
            }],
            highlighted_projects: vec![HighlightedProject {
                id: "proj-1".to_string(),
                title: "E-commerce Platform".to_string(),
                slug: "ecommerce-platform".to_string(),
                short_description: "Full-stack e-commerce solution".to_string(),
            }],
            contact_info: full_request().contact_info,
        }
    }

    /* --------------------------------------------------
     * Success Cases
     * -------------------------------------------------- */

    #[actix_web::test]
    async fn test_create_cv_success() {
        let user_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_create_cv(MockCreateCVUseCase::success(full_cv(user_id)))
            .build();

        let jwt_service = jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(create_cv_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/cvs")
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .set_json(&full_request())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);

        // 🔽 minimal & safe
        let body: Value = test::read_body_json(resp).await;
        let cv: CVInfo = serde_json::from_value(body["data"].clone()).unwrap();

        assert_eq!(cv.user_id, user_id);
        assert_eq!(cv.core_skills.len(), 2);
        assert_eq!(cv.contact_info.len(), 2);
    }

    /* --------------------------------------------------
     * Error & Auth Cases (unchanged behavior)
     * -------------------------------------------------- */

    #[actix_web::test]
    async fn test_create_cv_repository_error() {
        let user_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_create_cv(MockCreateCVUseCase::error(CreateCVError::RepositoryError(
                "Database connection failed".to_string(),
            )))
            .build();

        let jwt_service = jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(create_cv_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/cvs")
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, true))))
            .set_json(&base_create_request())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[actix_web::test]
    async fn test_create_cv_unverified_user() {
        let user_id = Uuid::new_v4();

        let app_state = TestAppStateBuilder::default()
            .with_create_cv(MockCreateCVUseCase::success(full_cv(user_id)))
            .build();

        let jwt_service = jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service);

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(create_cv_handler),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/cvs")
            .insert_header(("Authorization", format!("Bearer {}", token(user_id, false))))
            .set_json(&base_create_request())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }
}
